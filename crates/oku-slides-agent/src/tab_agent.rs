//! L'agent d'un onglet : une tâche tokio qui reçoit ses commandes, ouvre sa
//! session ACP sur la connexion partagée et joue ses tours.

use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol::schema::v1::{HttpHeader, McpServer, McpServerHttp, NewSessionRequest, SessionId};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::SurfaceHub;
use oku_slides_session::agent::{AgentUi, TabSession};
use tokio::sync::{broadcast, mpsc};

use crate::error::{AgentError, RequestSnafu, StoppedSnafu};
use crate::link::{Link, Live, Route};
use crate::{AgentEvent, AgentState};

/// Attente de la fin de la connexion après l'échec d'une requête : la
/// bibliothèque ACP fait échouer les requêtes en cours avant de signaler la
/// fin, et le prompt suivant doit relancer l'agent plutôt que reprendre
/// l'ancienne connexion.
const CLOSE_GRACE: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(crate) enum Command {
    Prompt { text: String, ui: AgentUi },
    Cancel,
    NewConversation,
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
    /// l'agent partagé (`link`) est lancé au premier prompt. Lâcher tous les
    /// clones arrête l'acteur (tour en cours annulé, session oubliée).
    pub fn spawn(
        tab: TabSession,
        hub: Arc<SurfaceHub>,
        tools: ToolsAccess,
        link: Link,
        localizer: Arc<Localizer>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(256);
        let ctx = Ctx { tab, hub, localizer, events: events.clone() };
        tokio::spawn(TabAgent { ctx, tools, link, conversation: None }.run(rx));
        Self { tx, events }
    }

    pub fn prompt(&self, text: String, ui: AgentUi) {
        let _ = self.tx.send(Command::Prompt { text, ui });
    }

    /// Arrête le tour en cours (`session/cancel`).
    pub fn cancel(&self) {
        let _ = self.tx.send(Command::Cancel);
    }

    /// Nouvelle session ACP au prochain prompt ; le fil repart de zéro.
    pub fn new_conversation(&self) {
        let _ = self.tx.send(Command::NewConversation);
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
    /// La session ACP de l'onglet et le lancement de l'agent auquel elle appartient.
    conversation: Option<(u64, SessionId)>,
}

impl TabAgent {
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Command>) {
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
                Command::NewConversation => self.new_conversation(),
                Command::Prompt { text, ui } => next = self.prompt(&mut rx, text, ui).await,
            }
        }
        // `AgentHandle` lâché : un tour en cours a déjà reçu `session/cancel` (voir `turn`).
        self.forget();
        self.ctx.hub.cancel_asks();
    }

    fn new_conversation(&mut self) {
        self.forget();
        // Questions relâchées, plus aucun tour, surfaces du fil fermées : avant toute nouvelle consigne.
        self.ctx.hub.clear_thread();
        self.ctx.emit(AgentEvent::Conversation);
    }

    fn forget(&mut self) {
        if let Some((generation, id)) = self.conversation.take() {
            self.link.forget(generation, id);
        }
    }

    /// Joue un tour ; rend la consigne arrivée pendant le tour, à jouer ensuite.
    async fn prompt(
        &mut self,
        rx: &mut mpsc::UnboundedReceiver<Command>,
        text: String,
        ui: AgentUi,
    ) -> Option<Command> {
        // Le tour existe avant toute étape qui peut échouer (connexion, session) : son erreur s'y affiche.
        let n = self.ctx.hub.begin_turn();
        self.ctx.emit(AgentEvent::UserText { turn: n, text: text.clone() });
        if self.conversation.is_none() {
            self.ctx.emit(AgentEvent::Status { state: AgentState::Starting });
        }
        let live = match self.link.connect().await {
            Ok(live) => live,
            Err(e) => {
                self.failed(n, &e);
                return None;
            }
        };
        let (id, fresh) = match &self.conversation {
            Some((generation, id)) if *generation == live.generation => (id.clone(), false),
            _ => {
                // Pas de session, ou une session d'un agent arrêté depuis.
                self.forget();
                match self.open(&live).await {
                    Ok(id) => (id, true),
                    Err(e) => {
                        self.request_failed(n, &live, e).await;
                        return None;
                    }
                }
            }
        };
        match crate::turn::run(&self.ctx, &live.cx, rx, n, id, text, ui, fresh).await {
            Ok(pending) => {
                if pending.new_conversation {
                    self.new_conversation();
                }
                pending.prompt.map(|(text, ui)| Command::Prompt { text, ui })
            }
            Err(e) => {
                self.request_failed(n, &live, e).await;
                None
            }
        }
    }

    /// Nouvelle session ACP, dans le dossier du document, avec les outils de l'onglet.
    async fn open(&mut self, live: &Live) -> agent_client_protocol::Result<SessionId> {
        let tab = self.ctx.tab.clone();
        let cwd = tokio::task::spawn_blocking(move || tab.with(|s| s.document_dir()))
            .await
            .ok()
            .and_then(Result::ok)
            .flatten()
            .unwrap_or_else(|| self.link.fallback_cwd().to_path_buf());
        let tools = McpServer::Http(
            McpServerHttp::new("oku-slides", self.tools.url.clone())
                .headers(vec![HttpHeader::new("Authorization", self.tools.authorization.clone())]),
        );
        let r = live.cx.send_request(NewSessionRequest::new(cwd).mcp_servers(vec![tools])).block_task().await?;
        self.link.route(
            live.generation,
            r.session_id.clone(),
            Route { events: self.ctx.events.clone(), hub: self.ctx.hub.clone() },
        );
        self.conversation = Some((live.generation, r.session_id.clone()));
        Ok(r.session_id)
    }

    /// Une requête du tour `turn` a échoué. Connexion finie : l'agent est arrêté, la session
    /// oubliée. Sinon l'agent a refusé la requête : erreur montrée, session gardée.
    async fn request_failed(&mut self, turn: u32, live: &Live, e: agent_client_protocol::Error) {
        if agent_client_protocol::is_incoming_transport_closed(&e) || live.closed.is_cancelled() {
            let _ = tokio::time::timeout(CLOSE_GRACE, live.closed.cancelled()).await;
            self.forget();
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
        self.ctx.emit(AgentEvent::Status { state: AgentState::Idle });
    }
}
