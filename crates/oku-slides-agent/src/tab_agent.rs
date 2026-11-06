//! A tab's agent: a tokio task that receives its commands.
//!
//! It opens or resumes the tab's ACP session on the shared connection and plays
//! its turns.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    HttpHeader, LoadSessionRequest, McpServer, McpServerHttp, NewSessionRequest, ResumeSessionRequest,
    SessionConfigOption, SessionId, SetSessionConfigOptionRequest,
};
use oku_slides_i18n::{LiveLocalizer, Localize};
use oku_slides_mcp::SurfaceHub;
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::{AgentUi, AttachmentRef, TabSession, TurnOrigin};
use oku_slides_session::{DocumentKind, Session};
use snafu::IntoError;
use tokio::sync::{broadcast, mpsc};

use crate::after_turn::AfterTx;
use crate::agent_switch::PromptGuard;
use crate::error::{AgentError, RequestSnafu, StoppedSnafu, WorkDirSnafu, WorkDirTaskSnafu};
use crate::keeper::{Keep, Keeper};
use crate::link::{Link, Live, Route};
use crate::session_model::{Offered, model_options};
use crate::voice_port::AppVoice;
use crate::{AgentEvent, AgentState};

#[path = "fallback_turn.rs"]
mod fallback_turn;

use fallback_turn::Connected;

/// Wait for the connection's end after a failed request: ACP fails pending requests before
/// signalling the end, and the next prompt must relaunch the agent, not reuse the old connection.
const CLOSE_GRACE: Duration = Duration::from_secs(1);

/// Beyond this, a model request goes unanswered: the turn goes on with the session's model.
const MODEL_WITHIN: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(crate) enum Command {
    Prompt(Instruction),
    Cancel,
    NewConversation,
}

/// Une consigne à jouer en un tour, dans la conversation de l'onglet.
#[derive(Debug)]
pub(crate) struct Instruction {
    /// Le texte montré dans le fil et envoyé en dernier bloc du prompt.
    pub text: String,
    pub ui: AgentUi,
    /// Tour de fine-tuning d'un import : les consignes d'import partent, cachées, avant le contexte.
    pub tune: bool,
    /// Les images collées envoyées avec la consigne.
    pub images: Vec<AttachmentRef>,
    /// How the turn began: typed, a hands-free utterance, or one after the wake phrase.
    pub origin: TurnOrigin,
}

/// Where to reach the tab's MCP tool server.
#[derive(Clone)]
pub struct ToolsAccess {
    /// URL of the tab's MCP endpoint.
    pub url: String,
    /// `Authorization` header for the tools; never logged.
    pub authorization: String,
}

impl std::fmt::Debug for ToolsAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolsAccess").field("url", &self.url).finish_non_exhaustive()
    }
}

/// What a tab and its turns share.
#[derive(Debug, Clone)]
pub(crate) struct Ctx {
    pub tab: TabSession,
    pub hub: Arc<SurfaceHub>,
    /// The interface's current language, read again for each text.
    pub localizer: LiveLocalizer,
    pub events: broadcast::Sender<AgentEvent>,
    pub keeper: Keeper,
    /// Counts the requests in progress for the settings box; `None` outside the stack.
    pub voice: Option<Arc<AppVoice>>,
    /// Bounds of the images sent as image blocks (`[paste] prompt_*`).
    pub prompt_limits: PromptImageLimits,
    /// The tab's document kind, read once when its agent started (its tools are that kind's).
    pub kind: DocumentKind,
    /// Where ended turns' quit or install go; `None` outside the stack (logged and dropped).
    pub after: Option<AfterTx>,
}

/// What a tab's actor starts from.
pub(crate) struct ActorParts {
    /// The tab.
    pub tab: TabSession,
    /// Its surfaces.
    pub hub: Arc<SurfaceHub>,
    /// Where to reach its MCP tools.
    pub tools: ToolsAccess,
    /// The shared ACP connection.
    pub link: Link,
    /// The interface's current language.
    pub localizer: LiveLocalizer,
    /// The keeper of its conversations.
    pub keeper: Keeper,
    /// Bounds of the images sent as image blocks.
    pub prompt_limits: PromptImageLimits,
    /// The tab's document kind, the one its tools were built for.
    pub kind: DocumentKind,
    /// Where its ended turns' quit or install go; `None` outside the stack.
    pub after: Option<AfterTx>,
}

impl Ctx {
    pub fn emit(&self, event: AgentEvent) {
        let _ = self.events.send(event);
    }

    /// Closes undo turn `turn`, whatever happened (synchronous lock: off the async thread).
    ///
    /// Its traces go to the hub before `TurnEnded` is emitted: its fallback receipt lists them.
    pub async fn end_turn(&self, turn: u32) {
        // The received-guide note belongs to the turn: every exit path ends here.
        self.hub.set_permission_note(None);
        let tab = self.tab.clone();
        match tokio::task::spawn_blocking(move || tab.with(Session::end_turn)).await {
            Ok(Ok(traces)) => self.hub.note_turn_traces(turn, traces),
            Ok(Err(e)) => {
                tracing::warn!(tab = %self.tab.tab(), error = %e.one_line(), "fin de tour : session introuvable");
            }
            Err(e) => tracing::warn!(tab = %self.tab.tab(), error = %e, "fin de tour : tâche interrompue"),
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
    /// Starts the agent of tab `tab`, a tab of `kind`, in the current tokio runtime.
    ///
    /// The shared agent (`link`) is started on the first prompt. Its conversation
    /// goes through `keeper`. Dropping every clone stops the actor (current turn
    /// cancelled, routes of its sessions forgotten).
    #[expect(
        clippy::too_many_arguments,
        reason = "each parameter is a distinct dependency of the actor; grouping them would add a type with no other use"
    )]
    pub fn spawn(
        tab: TabSession,
        hub: Arc<SurfaceHub>,
        tools: ToolsAccess,
        link: Link,
        localizer: LiveLocalizer,
        keeper: Keeper,
        prompt_limits: PromptImageLimits,
        kind: DocumentKind,
    ) -> Self {
        Self::start(ActorParts { tab, hub, tools, link, localizer, keeper, prompt_limits, kind, after: None }, None)
    }

    /// As [`AgentHandle::spawn`]; each request to the agent holds a guard of `voice`.
    pub(crate) fn spawn_counted(parts: ActorParts, voice: Arc<AppVoice>) -> Self {
        Self::start(parts, Some(voice))
    }

    fn start(parts: ActorParts, voice: Option<Arc<AppVoice>>) -> Self {
        let ActorParts { tab, hub, tools, link, localizer, keeper, prompt_limits, kind, after } = parts;
        let (tx, rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(256);
        let ctx = Ctx { tab, hub, localizer, events: events.clone(), keeper, voice, prompt_limits, kind, after };
        tokio::spawn(TabAgent { ctx, tools, link, live: HashSet::new(), on_local: false }.run(rx));
        Self { tx, events }
    }

    /// Une consigne pour la conversation de l'onglet.
    pub fn prompt(&self, text: String, ui: AgentUi) {
        self.instruct(Instruction { text, ui, tune: false, images: Vec::new(), origin: TurnOrigin::Typed });
    }

    /// Une consigne, ordinaire ou de fine-tuning.
    pub(crate) fn instruct(&self, instruction: Instruction) {
        // Acteur arrêté (onglet fermé) : la consigne est sans objet.
        let _ = self.tx.send(Command::Prompt(instruction));
    }

    /// Arrête le tour en cours (`session/cancel`).
    pub fn cancel(&self) {
        // Acteur arrêté : plus rien à annuler.
        let _ = self.tx.send(Command::Cancel);
    }

    /// Nouvelle conversation : nouvelle session ACP au prochain prompt, fil effacé.
    pub fn new_conversation(&self) {
        // Acteur arrêté (onglet fermé) : plus de conversation à recommencer.
        let _ = self.tx.send(Command::NewConversation);
    }

    /// One more subscriber to the tab's agent events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.events.subscribe()
    }
}

/// Ce qu'est devenue une session connue au premier prompt après le lancement de l'agent.
#[derive(Debug)]
enum Reopened {
    /// Reprise (`session/resume` ou `session/load`) : la conversation continue.
    Resumed(SessionId),
    /// Reprise non tentée : autre agent, ou agent qui ne sait rien reprendre.
    NotTried,
    /// Reprise tentée et refusée par l'agent (session inconnue de lui).
    Refused,
}

/// The session a turn opens with, looked up for the agent the link ran then.
struct Known {
    /// That agent's fingerprint.
    agent: String,
    /// The session and its agent's fingerprint (see `Keeper::lookup`).
    session: Option<(String, Option<String>)>,
}

/// What a tab's ACP session is opened with: its working folder and tool server.
#[derive(Debug, Clone)]
struct Setup {
    /// The tab's working folder, the session's `cwd`.
    cwd: PathBuf,
    /// The tab's MCP tool server.
    tools: McpServer,
}

/// L'acteur de l'onglet.
struct TabAgent {
    ctx: Ctx,
    tools: ToolsAccess,
    link: Link,
    /// Sessions ACP vivantes : (lancement de l'agent, id) ; un agent relancé peut réutiliser un id.
    live: HashSet<(u64, String)>,
    /// The tab was last told its turns run on the local agent (`AgentEvent::Fallback`).
    on_local: bool,
}

impl TabAgent {
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Command>) {
        // Le fil relu est au hub avant le premier tour (numéros de tour après les leurs).
        self.ctx.keeper.restored().await;
        let mut next = None;
        // La garde d'une requête suivie d'une consigne en file : gardée jusqu'à la fin de celle-ci.
        let mut busy: Option<PromptGuard> = None;
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
                Command::NewConversation => self.new_conversation().await,
                Command::Prompt(instruction) => {
                    // Une requête ACP tourne : un changement d'agent attend. Gardé pour une consigne en file.
                    let guard =
                        busy.take().or_else(|| self.ctx.voice.as_ref().map(|v| v.prompt_started(self.ctx.tab.tab())));
                    next = self.prompt(&mut rx, instruction).await;
                    if matches!(next, Some(Command::Prompt(_))) {
                        busy = guard;
                    }
                }
            }
        }
        // `AgentHandle` lâché : un tour en cours a déjà reçu `session/cancel` (voir `turn`).
        for (generation, id) in self.live.drain() {
            self.link.forget(generation, SessionId::new(id));
        }
        self.ctx.hub.cancel_asks();
    }

    /// Nouvelle conversation : oubliée par le gardien (son fil aussi), sa route aussi.
    async fn new_conversation(&mut self) {
        if let Some(id) = self.ctx.keeper.forget().await {
            let gone: Vec<(u64, String)> = self.live.iter().filter(|(_, i)| *i == id).cloned().collect();
            for key in gone {
                self.live.remove(&key);
                self.link.forget(key.0, SessionId::new(key.1));
            }
        }
        self.ctx.emit(AgentEvent::Conversation);
    }

    /// Joue un tour ; rend la consigne arrivée pendant le tour, à jouer ensuite.
    async fn prompt(&mut self, rx: &mut mpsc::UnboundedReceiver<Command>, instruction: Instruction) -> Option<Command> {
        // Le tour existe avant toute étape qui peut échouer (connexion, session) : son erreur s'y affiche.
        let n = self.ctx.hub.begin_turn_from(instruction.origin);
        self.ctx.emit(AgentEvent::UserText {
            turn: n,
            text: instruction.text.clone(),
            images: instruction.images.iter().map(AttachmentRef::shown).collect(),
        });
        let mut known = self.known_session().await;
        // Routed first: the local agent may stand in for an unusable selected one.
        let live = match self.connect_routed(rx, n, &mut known).await {
            Connected::Live(live) => live,
            Connected::Failed => return None,
            Connected::Interrupted(next) => return next,
        };
        let opened = match known.session {
            Some((id, _)) if self.live.contains(&(live.generation, id.clone())) => Ok((SessionId::new(id), false)),
            known => {
                // No working folder: the turn fails, never another folder in its place.
                let setup = match self.setup().await {
                    Ok(setup) => setup,
                    Err(e) => {
                        self.failed(n, &e);
                        return None;
                    }
                };
                self.open_or_reopen(&live, n, known, &setup).await
            }
        };
        let (id, fresh) = match opened {
            Ok(x) => x,
            Err(e) => {
                self.request_failed(n, &live, e).await;
                return None;
            }
        };
        self.ensure_model(&live, &id).await;
        let prompting = || self.link.prompting(live.generation, &id);
        match crate::turn::run(&self.ctx, &live.cx, rx, n, id.clone(), instruction, fresh, live.caps.image, prompting)
            .await
        {
            Ok((mut pending, usage)) => {
                crate::usage_board::record_turn(&self.ctx, &self.link, live.generation, &id, usage.as_ref()).await;
                // The one drain point of the turn's after-turn list: its reply is out. A tool's new
                // conversation joins the turn's own `Pending`, as the user's does.
                let (hub, after, tab) = (&self.ctx.hub, self.ctx.after.as_ref(), self.ctx.tab.tab());
                pending.new_conversation |=
                    crate::after_turn::drain_after_reply(hub, after, tab, n, !pending.cut).await;
                if pending.new_conversation {
                    self.new_conversation().await;
                }
                pending.prompt.map(Command::Prompt)
            }
            Err(e) => {
                self.request_failed(n, &live, e).await;
                None
            }
        }
    }

    /// The session known for the agent the link runs now; `Starting` shows unless it is live.
    async fn known_session(&self) -> Known {
        let agent = self.link.agent_id();
        let session = self.ctx.keeper.lookup(&agent).await;
        let current = self.link.current_generation();
        let alive = session.as_ref().zip(current).is_some_and(|((id, _), g)| self.live.contains(&(g, id.clone())));
        if !alive {
            self.ctx.emit(AgentEvent::Status { state: AgentState::Starting });
        }
        Known { agent, session }
    }

    /// The tab's ACP session for turn `turn`: resumed if `known` and the agent can, else a new one.
    ///
    /// Returns it and whether it is new.
    async fn open_or_reopen(
        &mut self,
        live: &Live,
        turn: u32,
        known: Option<(String, Option<String>)>,
        setup: &Setup,
    ) -> agent_client_protocol::Result<(SessionId, bool)> {
        let Some((id, agent)) = known else { return self.open(live, turn, setup).await.map(|id| (id, true)) };
        match self.reopen(live, turn, &id, agent.as_deref(), setup).await? {
            Reopened::Resumed(id) => Ok((id, false)),
            Reopened::NotTried => self.open(live, turn, setup).await.map(|id| (id, true)),
            Reopened::Refused => {
                self.memory_lost(turn);
                self.open(live, turn, setup).await.map(|id| (id, true))
            }
        }
    }

    /// The conversation's working folder (the session's `cwd`) and tool server.
    ///
    /// The folder is the conversation's own, empty at first (`Session::agent_work_dir`), never the
    /// deck's folder nor the data folder: what sits next to a deck never configures the agent.
    /// Before each new, resumed or loaded session, the config names an earlier turn may have
    /// written there are removed (`AgentWorkDir::prepare`), once the session lock is released.
    async fn setup(&self) -> Result<Setup, AgentError> {
        let tab = self.ctx.tab.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            tab.with(Session::agent_work_dir).and_then(|work| work).and_then(|work| work.prepare())
        });
        let cwd = match prepared.await {
            Ok(Ok(dir)) => dir,
            Ok(Err(source)) => return Err(WorkDirSnafu.into_error(source).into()),
            Err(source) => return Err(WorkDirTaskSnafu.into_error(source).into()),
        };
        let tools = McpServer::Http(
            McpServerHttp::new("oku-slides", self.tools.url.clone())
                .headers(vec![HttpHeader::new("Authorization", self.tools.authorization.clone())]),
        );
        Ok(Setup { cwd, tools })
    }

    /// Nouvelle session ACP de l'onglet, ouverte pour le tour `turn`.
    async fn open(&mut self, live: &Live, turn: u32, setup: &Setup) -> agent_client_protocol::Result<SessionId> {
        let request = NewSessionRequest::new(setup.cwd.clone()).mcp_servers(vec![setup.tools.clone()]);
        let r = live.cx.send_request(request).block_task().await?;
        self.adopt(live, turn, &r.session_id, false, r.config_options.as_deref());
        Ok(r.session_id)
    }

    /// Asks session `id` for the chosen model, if the agent offers it and the session uses another.
    ///
    /// A refusal is logged and the turn goes on with the session's model; the box shows it.
    async fn ensure_model(&self, live: &Live, id: &SessionId) {
        let Some(wanted) = self.link.wanted_model() else { return };
        let board = self.link.models();
        let config_id = match board.session(live.generation, id) {
            Some(o) if o.current != wanted && o.offers(&wanted) => o.config_id,
            Some(_) => return,
            // Its reply told no options: the offer of the agent's other sessions stands in.
            None => match board.offered(live.epoch) {
                Offered::Models(o) if o.offers(&wanted) => o.config_id,
                Offered::Models(_) | Offered::NoChoice | Offered::Unknown => return,
            },
        };
        let asked = SetSessionConfigOptionRequest::new(id.clone(), config_id, wanted.as_str());
        match tokio::time::timeout(MODEL_WITHIN, live.cx.send_request(asked).block_task()).await {
            Ok(Ok(r)) => board.told(live.generation, live.epoch, id, model_options(&r.config_options)),
            Ok(Err(e)) => {
                tracing::warn!(tab = %self.ctx.tab.tab(), model = %wanted, error = %e, "modèle de l'agent refusé");
            }
            Err(_) => tracing::warn!(tab = %self.ctx.tab.tab(), model = %wanted, "modèle de l'agent sans réponse"),
        }
    }

    /// Reprend la session `id`, si l'agent le peut.
    ///
    /// Autre agent ou aucune capacité : reprise non tentée ; session inconnue de l'agent :
    /// reprise refusée. Dans les deux cas, le tour en ouvre une autre.
    async fn reopen(
        &mut self,
        live: &Live,
        turn: u32,
        id: &str,
        agent: Option<&str>,
        setup: &Setup,
    ) -> agent_client_protocol::Result<Reopened> {
        if agent != Some(self.link.agent_id().as_str()) || !(live.caps.resume || live.caps.load) {
            return Ok(Reopened::NotTried);
        }
        let id = SessionId::new(id.to_owned());
        let Setup { cwd, tools } = setup.clone();
        // La route n'est posée qu'après la réponse : le rejeu de `session/load` est ignoré
        // (et, s'il arrive après, par la garde de la route jusqu'au premier prompt).
        let asked = if live.caps.resume {
            live.cx
                .send_request(ResumeSessionRequest::new(id.clone(), cwd).mcp_servers(vec![tools]))
                .block_task()
                .await
                .map(|r| r.config_options)
        } else {
            live.cx
                .send_request(LoadSessionRequest::new(id.clone(), cwd).mcp_servers(vec![tools]))
                .block_task()
                .await
                .map(|r| r.config_options)
        };
        match asked {
            Ok(options) => {
                // Un rejeu tardif (agent non conforme) reste hors du tour jusqu'au premier prompt (t-612).
                self.adopt(live, turn, &id, !live.caps.resume, options.as_deref());
                self.link.usage().resumed(live.generation, &id);
                Ok(Reopened::Resumed(id))
            }
            Err(e) if agent_client_protocol::is_incoming_transport_closed(&e) || live.closed.is_cancelled() => Err(e),
            Err(e) => {
                tracing::info!(tab = %self.ctx.tab.tab(), error = %e, "session de l'agent non reprise");
                Ok(Reopened::Refused)
            }
        }
    }

    /// Session `id` serves the tab from turn `turn`: routed, live, handed to the keeper.
    ///
    /// Its config `options` tell the models it offers. The keeper keeps it only if the turn is
    /// still in the thread (no new conversation meanwhile).
    fn adopt(
        &mut self,
        live: &Live,
        turn: u32,
        id: &SessionId,
        replaying: bool,
        options: Option<&[SessionConfigOption]>,
    ) {
        // A reply without options tells nothing: optional in ACP, it does not mean no choice.
        if let Some(options) = options {
            self.link.models().told(live.generation, live.epoch, id, model_options(options));
        }
        self.link.route(
            live.generation,
            id.clone(),
            Route {
                events: self.ctx.events.clone(),
                hub: self.ctx.hub.clone(),
                replaying: Arc::new(AtomicBool::new(replaying)),
            },
        );
        let key = id.0.to_string();
        self.live.insert((live.generation, key.clone()));
        self.ctx.keeper.send(Keep::Session { turn, id: key, agent: self.link.agent_id() });
    }

    /// L'agent a refusé de reprendre la conversation : une carte le dit dans le tour.
    ///
    /// Seulement après un refus (t-611) : un agent qui ne sait rien reprendre repart de zéro
    /// après chaque arrêt, la carte n'apprendrait rien de plus à chaque fois.
    fn memory_lost(&self, turn: u32) {
        let id = self.ctx.hub.next_id("note");
        let l = self.ctx.localizer.current();
        let text = l.tr("agent-memory-lost");
        if let Some(card) = oku_slides_session::agent::receipt::fallback(&id, &text, &[], &[], &l) {
            if !self.ctx.hub.show_static(turn, card) {
                tracing::debug!(turn, "carte « mémoire perdue » refusée : tour déjà clos");
            }
        } else {
            tracing::debug!("carte « mémoire perdue » vide : non montrée");
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
        // A failed turn runs nothing it recorded.
        self.ctx.hub.drop_after_turn();
        self.ctx.hub.cancel_asks();
        let message = e.localize(&self.ctx.localizer.current());
        self.ctx.emit(AgentEvent::Error { turn, message, spoken: e.spoken() });
        self.ctx.emit(AgentEvent::idle());
    }
}
