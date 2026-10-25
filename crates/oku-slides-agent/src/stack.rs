//! La pile de l'agent pour un hôte : un agent par onglet (outils MCP, fil
//! gardé) sur la connexion ACP partagée, et le relais des évènements vers
//! l'interface, enveloppés `{tab, event}`.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use oku_slides_i18n::Localize;
use oku_slides_mcp::{HubError, McpEvent, SurfaceAction};
use oku_slides_session::Platform;
use oku_slides_session::agent::{AgentUi, Sessions, TabSession};
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

/// Commande de l'interface vers l'agent d'un onglet (JSON `{"cmd": …, "tab": …}`),
/// la même pour Tauri et HTTP.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum AgentCommand {
    /// Une consigne ; avec des images collées (`images`), elle n'est jamais prise pour une réponse
    /// à une question ni pour un mot de raccourci.
    Prompt {
        tab: String,
        text: String,
        #[serde(default)]
        ui: AgentUi,
        /// La question à laquelle cette consigne répond : celle qui reçoit les touches 1–4
        /// (`quickTarget`). Absente, ou question qui n'attend plus : une consigne ordinaire.
        #[serde(default)]
        answer: Option<String>,
        /// Les images collées envoyées avec la consigne (ids `sha256:…` du brouillon).
        #[serde(default)]
        images: Vec<String>,
    },
    /// Fine-tune l'onglet importé : un tour dans la conversation de l'onglet, démarrant l'agent.
    ///
    /// Le fil montre le texte localisé `agent-tune-prompt`, dans la langue courante de
    /// l'interface ; ce texte est aussi le dernier bloc du prompt, après les consignes d'import
    /// cachées. Réservé à l'hôte ([`Stack::tune`], quand l'import demande le fine-tuning :
    /// `WorkspaceOutcome::Tune`) : jamais lu du canal de l'interface.
    #[serde(skip_deserializing)]
    Tune {
        tab: String,
    },
    Cancel {
        tab: String,
    },
    NewConversation {
        tab: String,
    },
    SurfaceAction {
        tab: String,
        surface: String,
        name: String,
        #[serde(default)]
        context: serde_json::Map<String, serde_json::Value>,
    },
    /// L'interface (ré)abonnée demande le fil gardé de l'onglet et ses surfaces ouvertes.
    Resync {
        tab: String,
    },
    /// Une autre vue de l'onglet (fenêtre de l'agent) s'abonne.
    ///
    /// Rejoue l'état, le brouillon, puis le fil de l'onglet ; ne démarre jamais l'onglet.
    Follow {
        tab: String,
    },
    /// Le brouillon de l'onglet ; `None` : fermé. Refusé si l'onglet n'a pas d'agent démarré.
    SetDraft {
        tab: String,
        text: Option<String>,
        /// La vue qui l'écrit, rendue avec l'écho.
        origin: Option<String>,
    },
    /// Une image collée (`data` : ses octets en base64), ajoutée au brouillon de l'onglet.
    ///
    /// Refusée sans agent démarré, au-delà de `[paste] max_draft_images`, ou par la session.
    PasteImage {
        tab: String,
        data: String,
    },
    /// Retire l'image `id` du brouillon.
    RemoveImage {
        tab: String,
        id: String,
    },
    /// L'interface ouvre le panneau des packs du deck : la surface va à cet onglet.
    ShowPacks {
        tab: String,
    },
    /// L'interface ouvre la boîte des réglages de la dictée et de l'agent.
    ShowSettings {
        tab: String,
    },
    /// L'interface demande si l'agent est utilisable (évènement `availability` à cet onglet).
    Availability {
        tab: String,
    },
}

impl AgentCommand {
    /// L'onglet visé.
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

    /// Traite une commande de l'interface ; l'erreur est lue par l'interface.
    ///
    /// Peut bloquer (un clic dans la boîte des réglages peut écrire `config.toml`, la boîte lit
    /// l'état du modèle) : à appeler hors des fils du runtime ; un raccourci « thème » rend jusqu'à 12 miniatures, 4 s au plus.
    pub fn handle(&self, cmd: AgentCommand) -> Result<(), String> {
        if self.is_closed(cmd.tab()) {
            tracing::debug!(tab = cmd.tab(), "commande pour un onglet fermé : ignorée");
            return Ok(());
        }
        match cmd {
            AgentCommand::Prompt { tab, text, ui, answer, images } => {
                // Fermé entre-temps : ignorée, comme ci-dessus.
                let Some(t) = self.open(&tab) else { return Ok(()) };
                // Réponse à la question désignée, jamais à une autre. Si elle n'attend plus (répondue
                // ou annulée entre-temps), la consigne n'est pas perdue : elle part comme toute
                // consigne (nouveau tour ; pendant un tour, elle l'interrompt puis le remplace).
                if images.is_empty() && answer.is_some_and(|id| t.hub.answer(&id, &text)) {
                    return Ok(());
                }
                // Raccourci « thème » : le sélecteur de l'hôte, sans tour ni agent ; un tour en cours
                // continue. Hors du mode deck, ou s'il échoue (journalisé), la consigne part à l'agent.
                if images.is_empty() && matches_shortcut(&text, THEME_SHORTCUT, &self.platform.live_localizer.current())
                {
                    match t.hub.show_themes(ui.slide.clone()) {
                        Ok(true) => return Ok(()),
                        Ok(false) => {}
                        Err(e) => {
                            tracing::warn!(tab, error = %snafu::Report::from_error(&e), "sélecteur de thème impossible");
                        }
                    }
                }
                let images = crate::paste::resolve_images(&t, &images, self.platform.settings.paste.max_draft_images)
                    .map_err(|e| {
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "consigne refusée");
                    e.localize(&self.platform.live_localizer.current())
                })?;
                t.send(Command::Prompt(Instruction { text, ui, tune: false, images }));
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
            // Démarre l'onglet (outils, gardien ; pas l'agent ACP) : le panneau va au hub de l'onglet.
            AgentCommand::ShowPacks { tab } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                return match t.hub.show_packs() {
                    Ok(true) => Ok(()),
                    // Le bouton n'existe qu'en mode deck : un changement de mode l'a devancé.
                    Ok(false) => {
                        tracing::debug!(tab, "panneau des packs hors du mode deck : ignoré");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::info!(tab, error = %snafu::Report::from_error(&e), "panneau des packs impossible");
                        Err(e.localize(&self.platform.live_localizer.current()))
                    }
                };
            }
            // Démarre l'onglet (outils, gardien ; pas l'agent ACP) : un onglet rouvert retrouve son fil.
            // L'état de l'agent d'abord, puis le fil (montré par le gardien, après sa relecture).
            AgentCommand::Resync { tab } => match self.open(&tab) {
                Some(t) => {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show);
                }
                // Fermé entre-temps : rien à montrer.
                None => self.replay_closed(&tab),
            },
            AgentCommand::Follow { tab } => match self.tab(&tab) {
                Some(t) => {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show);
                }
                None => self.replay_closed(&tab),
            },
            AgentCommand::SetDraft { tab, text, origin } => {
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
                match self.tab(&tab) {
                    Some(t) => t.set_draft(&self.out_tx, text, origin),
                    // Refusé, pas ignoré : la vue qui écrit attend un écho ou une erreur (tâche 6).
                    None => {
                        let e = CommandError::from(NoAgentSnafu.build());
                        tracing::debug!(tab, error = %snafu::Report::from_error(&e), "brouillon refusé");
                        return Err(e.localize(&self.platform.live_localizer.current()));
                    }
                }
            }
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
            AgentCommand::RemoveImage { tab, id } => match self.tab(&tab) {
                Some(t) => t.remove_image(&self.out_tx, &id),
                None => {
                    let e = CommandError::from(NoAgentSnafu.build());
                    return Err(e.localize(&self.platform.live_localizer.current()));
                }
            },
            AgentCommand::Availability { tab } => {
                emit(&self.out_tx, "agent", &tab, &AgentEvent::Availability { unusable: self.availability() });
            }
        }
        Ok(())
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
