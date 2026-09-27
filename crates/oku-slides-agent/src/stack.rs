//! La pile de l'agent pour un hôte : un agent par onglet (outils MCP, fil
//! gardé) sur la connexion ACP partagée, et le relais des évènements vers
//! l'interface, enveloppés `{tab, event}`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_mcp::{McpEvent, SurfaceAction};
use oku_slides_session::Platform;
use oku_slides_session::agent::{AgentUi, Sessions, TabSession};
use oku_slides_stt::Stt;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::settings_port::AppSettings;
use crate::tab_agent::Command;
use crate::tab_entry::{Out, TabEntry, TabLink, emit};
use crate::{AgentEvent, Launch, Link};

/// Commande de l'interface vers l'agent d'un onglet (JSON `{"cmd": …, "tab": …}`),
/// la même pour Tauri et HTTP.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum AgentCommand {
    Prompt {
        tab: String,
        text: String,
        #[serde(default)]
        ui: AgentUi,
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
}

impl AgentCommand {
    /// L'onglet visé.
    pub fn tab(&self) -> &str {
        match self {
            Self::Prompt { tab, .. }
            | Self::Cancel { tab }
            | Self::NewConversation { tab }
            | Self::SurfaceAction { tab, .. }
            | Self::Resync { tab } => tab,
        }
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
    tabs: Mutex<HashMap<String, TabEntry>>,
    /// Onglets fermés : leurs identifiants ne reviennent jamais, une
    /// commande tardive pour l'un d'eux est ignorée (sinon elle relancerait
    /// un agent que plus rien n'arrêterait).
    closed: Mutex<HashSet<String>>,
    out_tx: mpsc::UnboundedSender<Out>,
    out_rx: Mutex<Option<mpsc::UnboundedReceiver<Out>>>,
}

impl Stack {
    /// Rien ne démarre ici : l'agent au premier prompt, les outils d'un
    /// onglet à sa première commande, dans `rt`. `stt` : la dictée de
    /// l'hôte, pour `set_setting`.
    pub fn start(sessions: Arc<dyn Sessions>, platform: &Platform, stt: Option<Stt>, rt: Handle) -> Self {
        let a = &platform.settings.agent;
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
            tabs: Mutex::new(HashMap::new()),
            closed: Mutex::new(HashSet::new()),
            out_tx,
            out_rx: Mutex::new(Some(out_rx)),
        }
    }

    /// Traite une commande de l'interface ; l'erreur est lue par l'interface.
    pub fn handle(&self, cmd: AgentCommand) -> Result<(), String> {
        if self.is_closed(cmd.tab()) {
            tracing::debug!(tab = cmd.tab(), "commande pour un onglet fermé : ignorée");
            return Ok(());
        }
        match cmd {
            AgentCommand::Prompt { tab, text, ui } => {
                // Consigne dictée : réponse à la question en attente, sinon nouveau tour.
                let t = self.open(&tab);
                if !t.hub.answer_pending(&text) {
                    t.send(Command::Prompt { text, ui });
                }
            }
            AgentCommand::Cancel { tab } => {
                if let Some(t) = self.tab(&tab) {
                    t.hub.cancel_asks();
                    t.send(Command::Cancel);
                }
            }
            AgentCommand::NewConversation { tab } => match self.tab(&tab) {
                Some(t) => t.send(Command::NewConversation),
                // Rien à oublier : l'interface vide son fil, sans démarrer l'onglet.
                None => emit(&self.out_tx, "agent", &tab, &AgentEvent::Conversation),
            },
            AgentCommand::SurfaceAction { tab, surface, name, context } => {
                // Un onglet sans agent n'a aucune surface ouverte.
                let t = self.tab(&tab).ok_or_else(|| "unknown surface".to_owned())?;
                return t.hub.act(SurfaceAction { surface, name, context });
            }
            AgentCommand::Resync { tab } => self.resync(&tab),
        }
        Ok(())
    }

    /// Le fil gardé puis les surfaces ouvertes ; vides pour un onglet sans agent.
    fn resync(&self, tab: &str) {
        match self.tab(tab) {
            Some(t) => {
                t.replay(&self.out_tx);
                t.hub.resync();
            }
            None => {
                emit(&self.out_tx, "agent", tab, &AgentEvent::Thread { entries: Vec::new() });
                emit(&self.out_tx, "mcp", tab, &McpEvent::Surfaces { list: Vec::new() });
            }
        }
    }

    /// Le deck de l'onglet a changé depuis l'interface : fermer les post-its orphelins.
    pub fn after_change(&self, tab: &str) {
        if let Some(t) = self.tab(tab) {
            t.hub.prune();
        }
    }

    /// L'onglet est fermé : son agent s'arrête (tour annulé, questions
    /// relâchées), ses outils aussi.
    pub fn tab_closed(&self, tab: &str) {
        self.closed.lock().unwrap_or_else(PoisonError::into_inner).insert(tab.to_owned());
        let gone = self.tabs().remove(tab);
        drop(gone);
    }

    fn is_closed(&self, tab: &str) -> bool {
        self.closed.lock().unwrap_or_else(PoisonError::into_inner).contains(tab)
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

    fn tabs(&self) -> std::sync::MutexGuard<'_, HashMap<String, TabEntry>> {
        self.tabs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// L'onglet s'il a déjà un agent ; le verrou n'est pas gardé.
    fn tab(&self, tab: &str) -> Option<TabLink> {
        self.tabs().get(tab).map(|e| e.link.clone())
    }

    /// L'onglet, démarré s'il n'a pas encore d'agent.
    fn open(&self, tab: &str) -> TabLink {
        let mut tabs = self.tabs();
        if let Some(e) = tabs.get(tab) {
            return e.link.clone();
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
        tabs.insert(tab.to_owned(), entry);
        link
    }
}
