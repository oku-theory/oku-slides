//! La pile de l'agent pour un hôte : un agent par onglet (outils MCP, fil
//! gardé) sur la connexion ACP partagée, et le relais des évènements vers
//! l'interface, enveloppés `{tab, event}`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use oku_slides_i18n::Localize;
use oku_slides_mcp::{HubError, McpEvent, SurfaceAction};
use oku_slides_platform::config_write::write_value;
use oku_slides_session::Platform;
use oku_slides_session::agent::{AgentScope, AgentUi, Sessions, TabSession};
use oku_slides_stt::Stt;
use snafu::{OptionExt, ResultExt, ensure};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::command_error::{
    AgentLockedSnafu, AgentNotSavedSnafu, CommandError, DraftTooLongSnafu, NoAgentSnafu, OriginTooLongSnafu,
    UnknownAgentSnafu,
};
use crate::keeper::{Keep, Keeper};
use crate::picker::picker_surface;
use crate::settings_port::AppSettings;
use crate::tab_agent::Command;
use crate::tab_entry::{Out, TabEntry, TabLink, emit};
use crate::{AgentEvent, Launch, Link};

/// Taille maximale d'un brouillon, en octets UTF-8.
pub const MAX_DRAFT_BYTES: usize = 256 * 1024;
/// Taille maximale de l'origine d'un brouillon (identifiant de vue), en octets.
pub const MAX_ORIGIN_BYTES: usize = 64;

/// Commande de l'interface vers l'agent d'un onglet (JSON `{"cmd": …, "tab": …}`),
/// la même pour Tauri et HTTP.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum AgentCommand {
    Prompt {
        tab: String,
        /// La conversation visée ; absente : celle des slides.
        #[serde(default)]
        scope: AgentScope,
        text: String,
        #[serde(default)]
        ui: AgentUi,
        /// La question à laquelle cette consigne répond : celle qui reçoit les touches 1–4
        /// (`quickTarget`). Absente, ou question qui n'attend plus : une consigne ordinaire.
        #[serde(default)]
        answer: Option<String>,
    },
    Cancel {
        tab: String,
    },
    NewConversation {
        tab: String,
        /// La conversation à recommencer ; absente : celle des slides.
        #[serde(default)]
        scope: AgentScope,
    },
    SurfaceAction {
        tab: String,
        surface: String,
        name: String,
        #[serde(default)]
        context: serde_json::Map<String, serde_json::Value>,
    },
    /// L'interface (ré)abonnée demande le fil gardé de `scope` et ses surfaces ouvertes.
    Resync {
        tab: String,
        /// Le fil à montrer ; absent : celui des slides.
        #[serde(default)]
        scope: AgentScope,
    },
    /// Une autre vue de l'onglet (fenêtre de l'agent) s'abonne.
    ///
    /// Rejoue l'état, le brouillon, puis le fil de la portée montrée ; ne démarre jamais l'onglet.
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
    /// L'interface ouvre le panneau des agents : la surface est émise à cet onglet.
    Picker {
        tab: String,
    },
    /// Choisit l'agent de l'app (bascule immédiate), puis réémet le panneau à cet onglet.
    SelectAgent {
        tab: String,
        /// L'agent choisi, clé de `[agent.presets]`.
        id: String,
    },
}

impl AgentCommand {
    /// L'onglet visé.
    pub fn tab(&self) -> &str {
        match self {
            Self::Prompt { tab, .. }
            | Self::Cancel { tab }
            | Self::NewConversation { tab, .. }
            | Self::SurfaceAction { tab, .. }
            | Self::Resync { tab, .. }
            | Self::Follow { tab }
            | Self::SetDraft { tab, .. }
            | Self::Picker { tab }
            | Self::SelectAgent { tab, .. } => tab,
        }
    }
}

/// Attente de l'arrêt de l'agent ACP à la fin de l'app (`Stack::shutdown`).
const SHUTDOWN_WITHIN: Duration = Duration::from_secs(2);

/// Les onglets démarrés et fermés, sous un seul verrou : un onglet ne
/// démarre jamais après sa fermeture, même si les deux se croisent.
#[derive(Debug, Default)]
struct Table {
    tabs: HashMap<String, TabEntry>,
    /// Onglets fermés : leurs identifiants ne reviennent jamais, une
    /// commande tardive pour l'un d'eux est ignorée (sinon elle relancerait
    /// un agent que plus rien n'arrêterait).
    closed: HashSet<String>,
    /// Fin de l'app (`Stack::shutdown`) : plus aucun onglet ne démarre.
    shut: bool,
}

impl Table {
    fn refuses(&self, tab: &str) -> bool {
        self.shut || self.closed.contains(tab)
    }
}

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
    table: Mutex<Table>,
    out_tx: mpsc::UnboundedSender<Out>,
    out_rx: Mutex<Option<mpsc::UnboundedReceiver<Out>>>,
    /// L'agent choisi (clé de `[agent.presets]`) : celui du prochain lancement.
    active: Mutex<String>,
}

impl Stack {
    /// Rien ne démarre ici : les outils et le gardien d'un onglet démarrent
    /// à sa première commande, `resync` compris ; l'agent ACP au premier
    /// prompt ; tout dans `rt`. `stt` : la dictée de l'hôte, pour `set_setting`.
    pub fn start(sessions: Arc<dyn Sessions>, platform: &Platform, stt: Option<Stt>, rt: Handle) -> Self {
        let a = platform.settings.agent.active();
        let link = Link::new(Launch {
            command: a.command.clone(),
            args: a.args.clone(),
            fallback_cwd: platform.settings.paths.data(),
            ready_within: crate::READY_WITHIN,
        });
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        Self {
            link,
            sessions,
            platform: platform.clone(),
            settings: Arc::new(AppSettings::new(platform, stt)),
            rt,
            table: Mutex::new(Table::default()),
            out_tx,
            out_rx: Mutex::new(Some(out_rx)),
            active: Mutex::new(platform.settings.agent.selected.clone()),
        }
    }

    /// Traite une commande de l'interface ; l'erreur est lue par l'interface.
    ///
    /// Peut bloquer (`select_agent` écrit `config.toml`) : à appeler hors des fils du runtime.
    pub fn handle(&self, cmd: AgentCommand) -> Result<(), String> {
        if self.is_closed(cmd.tab()) {
            tracing::debug!(tab = cmd.tab(), "commande pour un onglet fermé : ignorée");
            return Ok(());
        }
        match cmd {
            AgentCommand::Prompt { tab, scope, text, ui, answer } => {
                // Fermé entre-temps : ignorée, comme ci-dessus.
                let Some(t) = self.open(&tab) else { return Ok(()) };
                // Réponse à la question désignée, jamais à une autre. Si elle n'attend plus (répondue
                // ou annulée entre-temps), la consigne n'est pas perdue : elle part comme toute
                // consigne (nouveau tour ; pendant un tour, elle l'interrompt puis le remplace).
                if !answer.is_some_and(|id| t.hub.answer(&id, &text)) {
                    t.send(Command::Prompt { scope, text, ui });
                }
            }
            AgentCommand::Cancel { tab } => {
                if let Some(t) = self.tab(&tab) {
                    t.hub.cancel_asks();
                    t.send(Command::Cancel);
                }
            }
            // Un onglet sans agent n'a aucune surface de tour à fermer : rien à faire pour lui.
            AgentCommand::NewConversation { tab, scope } => {
                if let Some(t) = self.tab(&tab) {
                    t.send(Command::NewConversation { scope });
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
            // Démarre l'onglet (outils, gardien ; pas l'agent ACP) : un onglet rouvert retrouve ses fils.
            // L'état de l'agent d'abord, puis le fil de `scope` (montré par le gardien, après sa relecture).
            AgentCommand::Resync { tab, scope } => match self.open(&tab) {
                Some(t) => {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show(scope));
                }
                // Fermé entre-temps : rien à montrer.
                None => self.replay_closed(&tab),
            },
            AgentCommand::Follow { tab } => match self.tab(&tab) {
                Some(t) => {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Replay);
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
            AgentCommand::Picker { tab } => self.emit_picker(&tab),
            AgentCommand::SelectAgent { tab, id } => {
                if let Err(e) = self.select_agent(&id) {
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "choix d'agent refusé");
                    return Err(e.localize(&self.platform.live_localizer.current()));
                }
                self.emit_picker(&tab);
            }
        }
        Ok(())
    }

    /// Le deck de l'onglet a changé depuis l'interface (voir
    /// `Request::changes_deck`) : fermer les surfaces orphelines, et faire appliquer au
    /// gardien les renommages de widgets tout de suite (un nouveau widget de l'ancien nom
    /// a sa propre conversation).
    pub fn after_change(&self, tab: &str) {
        if let Some(t) = self.tab(tab) {
            t.hub.prune();
            t.keeper.send(Keep::Sync);
        }
    }

    /// L'onglet est fermé : son agent s'arrête (tour annulé, questions
    /// relâchées), ses outils aussi.
    pub fn tab_closed(&self, tab: &str) {
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

    /// Choisit l'agent `id` pour toute l'app : enregistré dans `config.toml`, puis bascule.
    ///
    /// Rien ne change si le choix est refusé (inconnu, imposé, non enregistré).
    fn select_agent(&self, id: &str) -> Result<(), CommandError> {
        let agent = &self.platform.settings.agent;
        ensure!(!agent.locked, AgentLockedSnafu);
        let preset = agent.presets.get(id).context(UnknownAgentSnafu { id })?;
        // Gardé jusqu'à la bascule : deux choix croisés s'appliquent l'un après l'autre.
        let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        if *active == id {
            return Ok(());
        }
        write_value(
            &self.platform.storage,
            &self.platform.settings.config_file(),
            "agent.selected",
            &serde_json::Value::from(id),
        )
        .context(AgentNotSavedSnafu)?;
        self.link.switch(Launch {
            command: preset.command.clone(),
            args: preset.args.clone(),
            fallback_cwd: self.platform.settings.paths.data(),
            ready_within: crate::READY_WITHIN,
        });
        id.clone_into(&mut active);
        Ok(())
    }

    /// Émet le panneau des agents aux vues de `tab` ; un tour ouvert dans un onglet l'annonce.
    fn emit_picker(&self, tab: &str) {
        let busy = self.table().tabs.values().any(|e| e.link.hub.turn().is_some());
        let active = self.active.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let agent = &self.platform.settings.agent;
        let surface =
            picker_surface(&agent.presets, &active, busy, agent.locked, &self.platform.live_localizer.current());
        emit(&self.out_tx, "agent", tab, &AgentEvent::Picker { surface });
    }

    /// Ce que voit une vue d'un onglet sans agent démarré.
    ///
    /// Repos, aucun brouillon, les slides, aucune surface.
    fn replay_closed(&self, tab: &str) {
        emit(&self.out_tx, "agent", tab, &AgentEvent::idle());
        emit(&self.out_tx, "agent", tab, &AgentEvent::Draft { text: None, origin: None });
        emit(&self.out_tx, "mcp", tab, &McpEvent::Viewing { scope: AgentScope::Slides });
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
        let entry = TabEntry::start(
            session,
            &self.platform,
            self.settings.clone(),
            self.link.clone(),
            &self.rt,
            self.out_tx.clone(),
        );
        let link = entry.link.clone();
        table.tabs.insert(tab.to_owned(), entry);
        Some(link)
    }
}
