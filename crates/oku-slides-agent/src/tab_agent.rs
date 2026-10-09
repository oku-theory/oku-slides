//! L'agent d'un onglet : une tâche tokio qui reçoit ses commandes, ouvre ou
//! reprend une session ACP par portée sur la connexion partagée et joue ses tours.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    HttpHeader, LoadSessionRequest, McpServer, McpServerHttp, NewSessionRequest, ResumeSessionRequest, SessionId,
};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::SurfaceHub;
use oku_slides_session::agent::{AgentScope, AgentUi, TabSession};
use tokio::sync::{broadcast, mpsc};

use crate::error::{AgentError, RequestSnafu, StoppedSnafu};
use crate::keeper::{Keep, Keeper};
use crate::link::{Link, Live, Route};
use crate::{AgentEvent, AgentState};

/// Attente de la fin de la connexion après l'échec d'une requête : la
/// bibliothèque ACP fait échouer les requêtes en cours avant de signaler la
/// fin, et le prompt suivant doit relancer l'agent plutôt que reprendre
/// l'ancienne connexion.
const CLOSE_GRACE: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(crate) enum Command {
    Prompt { scope: AgentScope, text: String, ui: AgentUi },
    Cancel,
    NewConversation { scope: AgentScope },
}

/// Où joindre le serveur d'outils MCP de l'onglet.
#[derive(Clone)]
pub struct ToolsAccess {
    pub url: String,
    /// En-tête `Authorization` des outils ; jamais journalisé.
    pub authorization: String,
}

impl std::fmt::Debug for ToolsAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolsAccess").field("url", &self.url).finish_non_exhaustive()
    }
}

/// Ce que l'onglet et ses tours partagent.
#[derive(Debug, Clone)]
pub(crate) struct Ctx {
    pub tab: TabSession,
    pub hub: Arc<SurfaceHub>,
    pub localizer: Arc<Localizer>,
    pub events: broadcast::Sender<AgentEvent>,
    pub keeper: Keeper,
}

impl Ctx {
    pub fn emit(&self, event: AgentEvent) {
        let _ = self.events.send(event);
    }

    /// Ferme le tour d'annulation, quoi qu'il soit arrivé (verrou synchrone : hors thread async).
    pub async fn end_turn(&self) {
        let tab = self.tab.clone();
        let closed = tokio::task::spawn_blocking(move || tab.with(|s| s.end_turn())).await;
        if let Ok(Err(e)) = closed {
            tracing::warn!(tab = %self.tab.tab(), error = %e.one_line(), "fin de tour : session introuvable");
        }
    }
}

/// L'agent d'un onglet vu par l'hôte ; chaque méthode rend la main tout de suite.
#[derive(Debug, Clone)]
pub struct AgentHandle {
    tx: mpsc::UnboundedSender<Command>,
    events: broadcast::Sender<AgentEvent>,
}

impl AgentHandle {
    /// Démarre l'agent de l'onglet `tab` dans le runtime tokio courant ;
    /// l'agent partagé (`link`) est lancé au premier prompt. Ses conversations
    /// passent par `keeper`. Lâcher tous les clones arrête l'acteur (tour en
    /// cours annulé, routes de ses sessions oubliées).
    pub fn spawn(
        tab: TabSession,
        hub: Arc<SurfaceHub>,
        tools: ToolsAccess,
        link: Link,
        localizer: Arc<Localizer>,
        keeper: Keeper,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(256);
        let ctx = Ctx { tab, hub, localizer, events: events.clone(), keeper };
        tokio::spawn(TabAgent { ctx, tools, link, live: HashSet::new() }.run(rx));
        Self { tx, events }
    }

    /// Une consigne pour la conversation des slides.
    pub fn prompt(&self, text: String, ui: AgentUi) {
        self.prompt_in(AgentScope::Slides, text, ui);
    }

    /// Une consigne pour la conversation de `scope`.
    pub fn prompt_in(&self, scope: AgentScope, text: String, ui: AgentUi) {
        // Acteur arrêté (onglet fermé) : la consigne est sans objet.
        let _ = self.tx.send(Command::Prompt { scope, text, ui });
    }

    /// Arrête le tour en cours (`session/cancel`).
    pub fn cancel(&self) {
        // Acteur arrêté : plus rien à annuler.
        let _ = self.tx.send(Command::Cancel);
    }

    /// Nouvelle conversation dans `scope` : nouvelle session ACP au prochain prompt, fil effacé.
    pub fn new_conversation(&self, scope: AgentScope) {
        // Acteur arrêté (onglet fermé) : plus de conversation à recommencer.
        let _ = self.tx.send(Command::NewConversation { scope });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.events.subscribe()
    }
}

/// L'acteur de l'onglet.
struct TabAgent {
    ctx: Ctx,
    tools: ToolsAccess,
    link: Link,
    /// Sessions ACP vivantes : (lancement de l'agent, id) ; un agent relancé peut réutiliser un id.
    live: HashSet<(u64, String)>,
}

impl TabAgent {
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Command>) {
        // Les fils relus sont au hub avant le premier tour (numéros de tour après les leurs).
        self.ctx.keeper.restored().await;
        let mut next = None;
        loop {
            let cmd = match next.take() {
                Some(c) => c,
                None => match rx.recv().await {
                    Some(c) => c,
                    None => break,
                },
            };
            match cmd {
                Command::Cancel => self.ctx.hub.cancel_asks(),
                Command::NewConversation { scope } => self.new_conversation(scope).await,
                Command::Prompt { scope, text, ui } => next = self.prompt(&mut rx, scope, text, ui).await,
            }
        }
        // `AgentHandle` lâché : un tour en cours a déjà reçu `session/cancel` (voir `turn`).
        for (generation, id) in self.live.drain() {
            self.link.forget(generation, SessionId::new(id));
        }
        self.ctx.hub.cancel_asks();
    }

    /// Nouvelle conversation dans `scope` : oubliée par le gardien (son fil aussi), sa route aussi.
    async fn new_conversation(&mut self, scope: AgentScope) {
        if let Some(id) = self.ctx.keeper.forget(scope.clone()).await {
            let gone: Vec<(u64, String)> = self.live.iter().filter(|(_, i)| *i == id).cloned().collect();
            for key in gone {
                self.live.remove(&key);
                self.link.forget(key.0, SessionId::new(key.1));
            }
        }
        self.ctx.emit(AgentEvent::Conversation { scope });
    }

    /// Joue un tour dans `scope` ; rend la consigne arrivée pendant le tour, à jouer ensuite.
    async fn prompt(
        &mut self,
        rx: &mut mpsc::UnboundedReceiver<Command>,
        scope: AgentScope,
        text: String,
        ui: AgentUi,
    ) -> Option<Command> {
        // Le tour existe avant toute étape qui peut échouer (connexion, session) : son erreur s'y affiche.
        let n = self.ctx.hub.begin_turn(scope.clone());
        self.ctx.emit(AgentEvent::UserText { turn: n, scope: scope.clone(), text: text.clone() });
        let known = self.ctx.keeper.lookup(scope.clone()).await;
        let current = self.link.current_generation();
        let alive = known.as_ref().zip(current).is_some_and(|((id, _), g)| self.live.contains(&(g, id.clone())));
        if !alive {
            self.ctx.emit(AgentEvent::Status { state: AgentState::Starting, scope: Some(scope.clone()) });
        }
        let live = match self.link.connect().await {
            Ok(live) => live,
            Err(e) => {
                self.failed(n, &e);
                return None;
            }
        };
        let opened = match known {
            Some((id, _)) if self.live.contains(&(live.generation, id.clone())) => Ok((SessionId::new(id), false)),
            Some((id, agent)) => match self.reopen(&live, n, &id, agent.as_deref()).await {
                Ok(Some(id)) => Ok((id, false)),
                Ok(None) => {
                    self.memory_lost(n);
                    self.open(&live, n).await.map(|id| (id, true))
                }
                Err(e) => Err(e),
            },
            None => self.open(&live, n).await.map(|id| (id, true)),
        };
        let (id, fresh) = match opened {
            Ok(x) => x,
            Err(e) => {
                self.request_failed(n, &live, e).await;
                return None;
            }
        };
        match crate::turn::run(&self.ctx, &live.cx, rx, n, &scope, id, text, ui, fresh).await {
            Ok(pending) => {
                for scope in pending.new_conversations {
                    self.new_conversation(scope).await;
                }
                pending.prompt.map(|(scope, text, ui)| Command::Prompt { scope, text, ui })
            }
            Err(e) => {
                self.request_failed(n, &live, e).await;
                None
            }
        }
    }

    /// Dossier du document et serveur d'outils de l'onglet.
    async fn setup(&self) -> (PathBuf, McpServer) {
        let tab = self.ctx.tab.clone();
        let dir = match tokio::task::spawn_blocking(move || tab.with(|s| s.document_dir())).await {
            Ok(Ok(dir)) => dir,
            Ok(Err(e)) => {
                tracing::warn!(error = %e.one_line(), "dossier du document illisible : dossier par défaut");
                None
            }
            Err(e) => {
                tracing::warn!(error = %e, "dossier du document perdu : dossier par défaut");
                None
            }
        };
        let cwd = dir.unwrap_or_else(|| self.link.fallback_cwd().to_path_buf());
        let tools = McpServer::Http(
            McpServerHttp::new("oku-slides", self.tools.url.clone())
                .headers(vec![HttpHeader::new("Authorization", self.tools.authorization.clone())]),
        );
        (cwd, tools)
    }

    /// Nouvelle session ACP pour la portée du tour `turn`.
    async fn open(&mut self, live: &Live, turn: u32) -> agent_client_protocol::Result<SessionId> {
        let (cwd, tools) = self.setup().await;
        let r = live.cx.send_request(NewSessionRequest::new(cwd).mcp_servers(vec![tools])).block_task().await?;
        self.adopt(live, turn, r.session_id.clone());
        Ok(r.session_id)
    }

    /// Reprend la session `id` ; `None` si l'agent ne le peut pas.
    ///
    /// Autre agent, aucune capacité, ou session inconnue de l'agent : le tour en ouvre une autre.
    async fn reopen(
        &mut self,
        live: &Live,
        turn: u32,
        id: &str,
        agent: Option<&str>,
    ) -> agent_client_protocol::Result<Option<SessionId>> {
        if agent != Some(self.link.agent_id().as_str()) || !(live.caps.resume || live.caps.load) {
            return Ok(None);
        }
        let id = SessionId::new(id.to_owned());
        let (cwd, tools) = self.setup().await;
        // La route n'est posée qu'après la réponse : le rejeu de `session/load` est ignoré.
        let asked = if live.caps.resume {
            live.cx
                .send_request(ResumeSessionRequest::new(id.clone(), cwd).mcp_servers(vec![tools]))
                .block_task()
                .await
                .map(drop)
        } else {
            live.cx
                .send_request(LoadSessionRequest::new(id.clone(), cwd).mcp_servers(vec![tools]))
                .block_task()
                .await
                .map(drop)
        };
        match asked {
            Ok(()) => {
                self.adopt(live, turn, id.clone());
                Ok(Some(id))
            }
            Err(e) if agent_client_protocol::is_incoming_transport_closed(&e) || live.closed.is_cancelled() => Err(e),
            Err(e) => {
                tracing::info!(tab = %self.ctx.tab.tab(), error = %e, "session de l'agent non reprise");
                Ok(None)
            }
        }
    }

    /// La session `id` sert la portée du tour `turn` : routée, vivante, transmise au gardien.
    ///
    /// Le gardien lit la portée du tour au moment d'écrire : un widget renommé pendant
    /// l'ouverture de la session la garde.
    fn adopt(&mut self, live: &Live, turn: u32, id: SessionId) {
        self.link.route(
            live.generation,
            id.clone(),
            Route { events: self.ctx.events.clone(), hub: self.ctx.hub.clone() },
        );
        let key = id.0.to_string();
        self.live.insert((live.generation, key.clone()));
        self.ctx.keeper.send(Keep::Session { turn, id: key, agent: self.link.agent_id() });
    }

    /// L'agent a oublié la conversation : une carte le dit dans le tour.
    fn memory_lost(&self, turn: u32) {
        let id = self.ctx.hub.next_id("note");
        let text = self.ctx.localizer.tr("agent-memory-lost");
        match oku_slides_session::agent::receipt::fallback(&id, &text, &self.ctx.localizer) {
            Some(card) => {
                if !self.ctx.hub.show_static(turn, card) {
                    tracing::debug!(turn, "carte « mémoire perdue » refusée : tour déjà clos");
                }
            }
            None => tracing::debug!("carte « mémoire perdue » vide : non montrée"),
        }
    }

    /// Une requête du tour `turn` a échoué. Connexion finie : l'agent est arrêté ; la session
    /// reste connue du gardien (ses routes partent avec le lancement), elle sera reprise.
    /// Sinon l'agent a refusé la requête : erreur montrée, session gardée.
    async fn request_failed(&mut self, turn: u32, live: &Live, e: agent_client_protocol::Error) {
        if agent_client_protocol::is_incoming_transport_closed(&e) || live.closed.is_cancelled() {
            let _ = tokio::time::timeout(CLOSE_GRACE, live.closed.cancelled()).await;
            self.failed(turn, &StoppedSnafu { message: e.to_string() }.build().into());
        } else {
            self.failed(turn, &RequestSnafu { message: e.to_string() }.build().into());
        }
    }

    /// Le tour `turn` a échoué : l'erreur s'affiche dans ce tour.
    fn failed(&self, turn: u32, e: &AgentError) {
        tracing::warn!(tab = %self.ctx.tab.tab(), error = %e.one_line(), "agent : échec");
        self.ctx.hub.cancel_asks();
        self.ctx.emit(AgentEvent::Error { turn, message: e.localize(&self.ctx.localizer) });
        self.ctx.emit(AgentEvent::idle());
    }
}
