//! La pile de l'agent pour un hôte : un agent par onglet (outils MCP, fil
//! gardé) sur la connexion ACP partagée, et le relais des évènements vers
//! l'interface, enveloppés `{tab, event}`.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use oku_slides_i18n::Localize;
use oku_slides_mcp::{HubError, McpEvent, SurfaceAction};
use oku_slides_session::Platform;
use oku_slides_session::agent::{AgentUi, AttachmentRef, Sessions, TabSession};
use oku_slides_stt::Stt;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::choice::AgentChoice;
use crate::command_error::{CommandError, DraftTooLongSnafu, NoAgentSnafu, OriginTooLongSnafu};
use crate::keeper::{Keep, Keeper};
use crate::settings_port::AppSettings;
use crate::shortcut::{THEME_SHORTCUT, matches_shortcut};
use crate::tab_agent::{Command, Instruction};
use crate::tab_entry::{Out, TabEntry, TabLink, TabPorts, emit};
use crate::table::Table;
use crate::voice_port::AppVoice;
use crate::{AgentEvent, Link};

/// Taille maximale d'un brouillon, en octets UTF-8.
pub const MAX_DRAFT_BYTES: usize = 256 * 1024;
/// Taille maximale de l'origine d'un brouillon (identifiant de vue), en octets.
pub const MAX_ORIGIN_BYTES: usize = 64;

/// A command from the interface to a tab's agent.
///
/// JSON `{"cmd": …, "tab": …}`, the same for Tauri and HTTP.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum AgentCommand {
    /// An instruction for the tab's agent.
    ///
    /// With pasted images (`images`), it is never taken for an answer to a question nor for a
    /// shortcut word.
    Prompt {
        /// The target tab.
        tab: String,
        /// The instruction, as spoken or typed.
        text: String,
        /// What the interface shows with the instruction: slide, selected element and field.
        #[serde(default)]
        ui: AgentUi,
        /// The question this instruction answers: the one receiving keys 1–4
        /// (`quickTarget`). Absent, or a question no longer waiting: an ordinary instruction.
        #[serde(default)]
        answer: Option<String>,
        /// The pasted images sent with the instruction (draft ids `sha256:…`).
        #[serde(default)]
        images: Vec<String>,
    },
    /// Fine-tunes the imported tab: a turn in the tab's conversation, starting the agent.
    ///
    /// The thread shows the localized text `agent-tune-prompt`, in the interface's current
    /// language; this text is also the last block of the prompt, after the hidden import
    /// instructions. Reserved to the host ([`Stack::tune`], when the import asks for
    /// fine-tuning: `WorkspaceOutcome::Tune`): never read from the interface channel.
    #[serde(skip_deserializing)]
    Tune {
        /// The imported tab.
        tab: String,
    },
    /// Interrupts the tab's running turn.
    Cancel {
        /// The target tab.
        tab: String,
    },
    /// Starts a new conversation in the tab: the thread starts over.
    NewConversation {
        /// The target tab.
        tab: String,
    },
    /// The user acts on a surface (button, choice, form).
    SurfaceAction {
        /// The tab showing the surface.
        tab: String,
        /// Id of the surface.
        surface: String,
        /// Name of the action.
        name: String,
        /// Form values at the time of the action.
        #[serde(default)]
        context: serde_json::Map<String, serde_json::Value>,
    },
    /// The (re)subscribed interface asks for the tab's kept thread and its open surfaces.
    Resync {
        /// The target tab.
        tab: String,
    },
    /// Another view of the tab (the agent window) subscribes.
    ///
    /// Replays the state, the draft, then the tab's thread; never starts the tab.
    Follow {
        /// The followed tab.
        tab: String,
    },
    /// The tab's draft; `None`: closed. Refused if the tab has no started agent.
    SetDraft {
        /// The target tab.
        tab: String,
        /// The draft's text; `None` closes it.
        text: Option<String>,
        /// The view writing it, returned with the echo.
        origin: Option<String>,
    },
    /// A pasted image (`data`: its bytes in base64), added to the tab's draft.
    ///
    /// Refused without a started agent, beyond `[paste] max_draft_images`, or by the session.
    PasteImage {
        /// The target tab.
        tab: String,
        /// The image bytes, in base64.
        data: String,
    },
    /// Removes image `id` from the draft.
    RemoveImage {
        /// The target tab.
        tab: String,
        /// Id of the image to remove.
        id: String,
    },
    /// The interface opens the deck's packs panel: the surface goes to this tab.
    ShowPacks {
        /// The target tab.
        tab: String,
    },
    /// The interface opens the dictation and agent settings box.
    ShowSettings {
        /// The target tab.
        tab: String,
    },
    /// The interface asks whether the agent is usable (`availability` event to this tab).
    Availability {
        /// The target tab.
        tab: String,
    },
}

impl AgentCommand {
    /// The target tab.
    #[must_use]
    pub fn tab(&self) -> &str {
        match self {
            Self::Prompt { tab, .. }
            | Self::Tune { tab, .. }
            | Self::Cancel { tab }
            | Self::NewConversation { tab }
            | Self::SurfaceAction { tab, .. }
            | Self::Resync { tab }
            | Self::Follow { tab }
            | Self::SetDraft { tab, .. }
            | Self::PasteImage { tab, .. }
            | Self::RemoveImage { tab, .. }
            | Self::ShowPacks { tab }
            | Self::ShowSettings { tab }
            | Self::Availability { tab } => tab,
        }
    }
}

/// Clé Fluent du texte du fil pour le fine-tuning d'un import.
const TUNE_PROMPT: &str = "agent-tune-prompt";

/// Attente de l'arrêt de l'agent ACP à la fin de l'app (`Stack::shutdown`).
const SHUTDOWN_WITHIN: Duration = Duration::from_secs(2);

/// Les agents des onglets d'un hôte, sur un seul agent ACP ; chaque onglet
/// démarre à sa première commande et s'arrête à sa fermeture (ou avec la pile).
///
/// Garde une copie de la [`Platform`] : comme elle, à lâcher hors d'un
/// contexte async si c'est sa dernière copie (`Storage` possède un runtime).
#[derive(Debug)]
pub struct Stack {
    link: Link,
    sessions: Arc<dyn Sessions>,
    platform: Platform,
    settings: Arc<AppSettings>,
    rt: Handle,
    table: Arc<Mutex<Table>>,
    out_tx: mpsc::UnboundedSender<Out>,
    out_rx: Mutex<Option<mpsc::UnboundedReceiver<Out>>>,
    /// L'agent choisi : celui du prochain lancement, sa disponibilité, le panneau.
    choice: Arc<AgentChoice>,
    /// La dictée et l'agent de l'app : la boîte des réglages de chaque onglet.
    voice: Arc<AppVoice>,
}

impl Stack {
    /// Rien ne démarre ici : les outils et le gardien d'un onglet démarrent
    /// à sa première commande, `resync` compris ; l'agent ACP au premier
    /// prompt ; tout dans `rt`. `stt` : la dictée de l'hôte, pour la boîte des réglages
    /// et l'outil `settings`. Seules la veille de la dictée et le suivi de la boîte partent ici.
    pub fn start(sessions: Arc<dyn Sessions>, platform: &Platform, stt: Option<Stt>, rt: Handle) -> Self {
        let a = platform.settings.agent.active();
        let link = Link::new(AgentChoice::launch_of(a, platform));
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        let choice = Arc::new(AgentChoice::new(platform, link.clone()));
        let voice = AppVoice::new(stt.clone(), choice.clone(), rt.clone());
        let table = Arc::new(Mutex::new(Table::default()));
        crate::voice_refresh::spawn(&rt, Arc::downgrade(&voice), Arc::downgrade(&table), out_tx.clone());
        Self {
            choice,
            voice,
            link,
            sessions,
            platform: platform.clone(),
            settings: Arc::new(AppSettings::new(platform, stt)),
            rt,
            table,
            out_tx,
            out_rx: Mutex::new(Some(out_rx)),
        }
    }

    /// Handles an interface command; the interface reads the error.
    ///
    /// May block (a click in the settings box may write `config.toml`, the box reads the
    /// model's state): call it off the runtime threads; a "theme" shortcut renders up to 12
    /// thumbnails, 4 s at most.
    ///
    /// # Errors
    ///
    /// Returns the localized message when the command is refused: rejected instruction, draft or
    /// pasted image, refused surface action, no started agent, or packs panel failure.
    pub fn handle(&self, cmd: AgentCommand) -> Result<(), String> {
        if self.is_closed(cmd.tab()) {
            tracing::debug!(tab = cmd.tab(), "commande pour un onglet fermé : ignorée");
            return Ok(());
        }
        match cmd {
            AgentCommand::Prompt { tab, text, ui, answer, images } => {
                if let Some((t, images)) = self.route_prompt(&tab, answer, &text, ui.slide.as_deref(), &images)? {
                    t.send(Command::Prompt(Instruction { text, ui, tune: false, images }));
                }
            }
            AgentCommand::Tune { tab } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                let text = self.platform.live_localizer.current().tr(TUNE_PROMPT);
                let ui = AgentUi::default();
                t.send(Command::Prompt(Instruction { text, ui, tune: true, images: Vec::new() }));
            }
            AgentCommand::Cancel { tab } => {
                if let Some(t) = self.tab(&tab) {
                    t.hub.cancel_asks();
                    t.send(Command::Cancel);
                }
            }
            // Un onglet sans agent n'a aucune surface de tour à fermer : rien à faire pour lui.
            AgentCommand::NewConversation { tab } => {
                if let Some(t) = self.tab(&tab) {
                    t.send(Command::NewConversation);
                }
            }
            AgentCommand::SurfaceAction { tab, surface, name, context } => {
                // Un onglet sans agent n'a aucune surface ouverte. L'interface affiche le texte
                // reçu : dit dans la langue de l'utilisateur, l'erreur complète reste au journal.
                let acted = match self.tab(&tab) {
                    Some(t) => t.hub.act(SurfaceAction { surface, name, context }),
                    None => Err(HubError::unknown_surface(surface)),
                };
                return acted.map_err(|e| {
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "action de surface refusée");
                    e.localize(&self.platform.live_localizer.current())
                });
            }
            AgentCommand::ShowPacks { tab } => return self.show_packs(&tab),
            // Starts the tab (tools, keeper; not the ACP agent): a reopened tab gets its thread back.
            // The agent's state first, then the thread (shown by the keeper, after its replay).
            AgentCommand::Resync { tab } => {
                if let Some(t) = self.open(&tab) {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show);
                } else {
                    // Closed in the meantime: nothing to show.
                    self.replay_closed(&tab);
                }
            }
            AgentCommand::Follow { tab } => {
                if let Some(t) = self.tab(&tab) {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show);
                } else {
                    self.replay_closed(&tab);
                }
            }
            AgentCommand::SetDraft { tab, text, origin } => return self.set_draft(&tab, text, origin),
            // Démarre l'onglet (outils, gardien ; pas l'agent ACP) : la boîte va au hub de l'onglet.
            AgentCommand::ShowSettings { tab } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                t.hub.show_settings();
            }
            AgentCommand::PasteImage { tab, data } => {
                let l = self.platform.live_localizer.current();
                let Some(t) = self.tab(&tab) else {
                    return Err(CommandError::from(NoAgentSnafu.build()).localize(&l));
                };
                return crate::paste::paste_image(&t, &self.out_tx, &self.platform.settings.paste, &data).map_err(
                    |e| {
                        tracing::info!(tab, error = %snafu::Report::from_error(&e), "image collée refusée");
                        e.localize(&l)
                    },
                );
            }
            AgentCommand::RemoveImage { tab, id } => {
                if let Some(t) = self.tab(&tab) {
                    t.remove_image(&self.out_tx, &id);
                } else {
                    let e = CommandError::from(NoAgentSnafu.build());
                    return Err(e.localize(&self.platform.live_localizer.current()));
                }
            }
            AgentCommand::Availability { tab } => {
                emit(&self.out_tx, "agent", &tab, &AgentEvent::Availability { unusable: self.availability() });
            }
        }
        Ok(())
    }

    /// Routes an instruction: answer, theme shortcut, or the agent with its images.
    ///
    /// Returns the tab and resolved images of an instruction that goes to the agent; `None` when
    /// nothing goes to the agent: tab closed, question answered or theme picker shown.
    fn route_prompt(
        &self,
        tab: &str,
        answer: Option<String>,
        text: &str,
        slide: Option<&str>,
        images: &[String],
    ) -> Result<Option<(TabLink, Vec<AttachmentRef>)>, String> {
        // Closed in the meantime: ignored, as in `handle`.
        let Some(t) = self.open(tab) else { return Ok(None) };
        // An answer to the designated question, never to another one. If it no longer waits
        // (answered or cancelled in the meantime), the instruction is not lost: it goes like any
        // instruction (a new turn; during a turn, it interrupts then replaces it).
        if images.is_empty() && answer.is_some_and(|id| t.hub.answer(&id, text)) {
            return Ok(None);
        }
        // "Theme" shortcut: the host's picker, with no turn nor agent; a running turn goes on.
        // Outside deck mode, or if it fails (logged), the instruction goes to the agent.
        if images.is_empty() && matches_shortcut(text, THEME_SHORTCUT, &self.platform.live_localizer.current()) {
            match t.hub.show_themes(slide) {
                Ok(true) => return Ok(None),
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(tab, error = %snafu::Report::from_error(&e), "sélecteur de thème impossible");
                }
            }
        }
        let images =
            crate::paste::resolve_images(&t, images, self.platform.settings.paste.max_draft_images).map_err(|e| {
                tracing::info!(tab, error = %snafu::Report::from_error(&e), "consigne refusée");
                e.localize(&self.platform.live_localizer.current())
            })?;
        Ok(Some((t, images)))
    }

    /// Sets the tab's draft after checking its sizes; the error is localized.
    fn set_draft(&self, tab: &str, text: Option<String>, origin: Option<String>) -> Result<(), String> {
        let checked = match (text.as_ref().map(String::len), origin.as_ref().map(String::len)) {
            (Some(bytes), _) if bytes > MAX_DRAFT_BYTES => {
                Err(DraftTooLongSnafu { bytes, max: MAX_DRAFT_BYTES }.build())
            }
            (_, Some(bytes)) if bytes > MAX_ORIGIN_BYTES => {
                Err(OriginTooLongSnafu { bytes, max: MAX_ORIGIN_BYTES }.build())
            }
            _ => Ok(()),
        };
        if let Err(inner) = checked {
            let e = CommandError::from(inner);
            tracing::info!(tab, error = %snafu::Report::from_error(&e), "brouillon refusé");
            return Err(e.localize(&self.platform.live_localizer.current()));
        }
        // Refused, not ignored: the writing view waits for an echo or an error (t-654).
        let Some(t) = self.tab(tab) else {
            let e = CommandError::from(NoAgentSnafu.build());
            tracing::debug!(tab, error = %snafu::Report::from_error(&e), "brouillon refusé");
            return Err(e.localize(&self.platform.live_localizer.current()));
        };
        t.set_draft(&self.out_tx, text, origin);
        Ok(())
    }

    /// Shows the deck's packs panel in the tab; the error is localized.
    ///
    /// Starts the tab (tools, keeper; not the ACP agent): the panel goes to the tab's hub.
    fn show_packs(&self, tab: &str) -> Result<(), String> {
        let Some(t) = self.open(tab) else { return Ok(()) };
        match t.hub.show_packs() {
            Ok(true) => Ok(()),
            // The button exists only in deck mode: a mode change got ahead of it.
            Ok(false) => {
                tracing::debug!(tab, "panneau des packs hors du mode deck : ignoré");
                Ok(())
            }
            Err(e) => {
                tracing::info!(tab, error = %snafu::Report::from_error(&e), "panneau des packs impossible");
                Err(e.localize(&self.platform.live_localizer.current()))
            }
        }
    }

    /// Lance le tour de fine-tuning de l'onglet importé `tab` (voir [`AgentCommand::Tune`]).
    ///
    /// Pour l'hôte, quand un import demande le fine-tuning ; démarre l'agent de l'onglet.
    /// Refusé si l'agent est inutilisable ([`Stack::availability`]) : rien n'est lancé, et
    /// l'onglet reçoit la raison (`AgentEvent::Availability`), comme après un lancement raté.
    /// Peut bloquer (lit le disque) : à appeler hors des fils du runtime.
    pub fn tune(&self, tab: &str) {
        if let Some(reason) = self.availability() {
            tracing::warn!(tab, reason, "fine-tuning de l'import non lancé : agent inutilisable");
            emit(&self.out_tx, "agent", tab, &AgentEvent::Availability { unusable: Some(reason) });
            return;
        }
        if let Err(e) = self.handle(AgentCommand::Tune { tab: tab.to_owned() }) {
            tracing::warn!(tab, error = %e, "fine-tuning de l'import non lancé");
        }
    }

    /// Pourquoi l'agent choisi est inutilisable, localisé ; `None` s'il semble utilisable.
    ///
    /// Son programme introuvable, l'état du moteur local pour l'agent intégré, sinon l'échec
    /// récent de son dernier lancement. Peut bloquer : lit le disque.
    pub fn availability(&self) -> Option<String> {
        self.choice.availability()
    }

    /// Le deck de l'onglet a changé depuis l'interface (voir
    /// `Request::changes_deck`) : fermer les surfaces orphelines.
    pub fn after_change(&self, tab: &str) {
        if let Some(t) = self.tab(tab) {
            t.hub.prune();
        }
    }

    /// L'onglet est fermé : son agent s'arrête (tour annulé, questions
    /// relâchées), ses outils aussi.
    pub fn tab_closed(&self, tab: &str) {
        self.voice.tab_closed(tab);
        let gone = {
            let mut table = self.table();
            table.closed.insert(tab.to_owned());
            table.tabs.remove(tab)
        };
        // Lâché hors du verrou : l'arrêt d'un onglet ne bloque pas les autres.
        drop(gone);
    }

    /// Fin de l'app : chaque onglet s'arrête, les conversations en file sont
    /// écrites, puis l'agent ACP est tué (chacun attendu au plus 2 s) ; toute
    /// commande ensuite est ignorée.
    pub async fn shutdown(&self) {
        // Plus aucun changement d'agent : un choix en attente tombe.
        self.voice.shutdown();
        // Le `.part` du modèle (2,5 Go) ne doit pas grossir après la fin de l'app.
        self.choice.cancel_download();
        let gone = {
            let mut table = self.table();
            table.shut = true;
            std::mem::take(&mut table.tabs)
        };
        let keepers: Vec<Keeper> = gone.values().map(|e| e.link.keeper.clone()).collect();
        drop(gone);
        // Les fins de tour encore en file sont écrites avant l'arrêt (bornées, comme l'agent).
        let flushed = async {
            for keeper in &keepers {
                keeper.flush().await;
            }
        };
        if tokio::time::timeout(SHUTDOWN_WITHIN, flushed).await.is_err() {
            tracing::warn!("conversations de l'agent pas toutes écrites à temps");
        }
        self.link.shutdown(SHUTDOWN_WITHIN).await;
    }

    /// Ce que voit une vue d'un onglet sans agent démarré.
    ///
    /// Repos, aucun brouillon, aucune surface.
    fn replay_closed(&self, tab: &str) {
        emit(&self.out_tx, "agent", tab, &AgentEvent::idle());
        emit(&self.out_tx, "agent", tab, &AgentEvent::Draft { text: None, origin: None, images: Vec::new() });
        emit(&self.out_tx, "mcp", tab, &McpEvent::Surfaces { list: Vec::new() });
    }

    fn is_closed(&self, tab: &str) -> bool {
        self.table().refuses(tab)
    }

    /// Relaie les évènements de tous les onglets vers `sink` : canal
    /// (`"mcp"`, `"agent"`) et JSON `{"tab": …, "event": …}`. Une seule fois.
    pub fn forward(&self, sink: impl Fn(&'static str, String) + Send + Sync + 'static) {
        let taken = self.out_rx.lock().unwrap_or_else(PoisonError::into_inner).take();
        let Some(mut rx) = taken else {
            tracing::warn!("forward déjà appelé : ignoré");
            return;
        };
        self.rt.spawn(async move {
            while let Some((channel, json)) = rx.recv().await {
                sink(channel, json);
            }
        });
    }

    fn table(&self) -> std::sync::MutexGuard<'_, Table> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// L'onglet s'il a déjà un agent ; le verrou n'est pas gardé.
    fn tab(&self, tab: &str) -> Option<TabLink> {
        self.table().tabs.get(tab).map(|e| e.link.clone())
    }

    /// L'onglet, démarré s'il n'a pas encore d'agent ; `None` s'il est fermé
    /// (ou l'app finie), vérifié sous le verrou où il serait inséré.
    fn open(&self, tab: &str) -> Option<TabLink> {
        let mut table = self.table();
        if table.refuses(tab) {
            return None;
        }
        if let Some(e) = table.tabs.get(tab) {
            return Some(e.link.clone());
        }
        let session = TabSession::new(tab, self.sessions.clone());
        let ports = TabPorts { settings: self.settings.clone(), voice: self.voice.clone() };
        let entry = TabEntry::start(session, &self.platform, ports, self.link.clone(), &self.rt, self.out_tx.clone());
        let link = entry.link.clone();
        table.tabs.insert(tab.to_owned(), entry);
        Some(link)
    }
}
