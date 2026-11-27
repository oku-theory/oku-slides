//! A tab of the stack: its surface hub, its MCP tools, its agent and its
//! turns, with the relays of its events to the interface.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::{HostPorts, McpEvent, SettingsPort, SurfaceHub, TabPort, ToolServer, Tools};
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::{AgentWorkspace, AppPort, AttachmentRef, SpokenFailure, TabSession};
use oku_slides_session::{DocumentKind, Platform};
use serde::Serialize;
use tokio::runtime::Handle;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::after_turn::AfterTx;
use crate::keeper::{Keep, Keeper};
use crate::settings_port::AppSettings;
use crate::tab_agent::{ActorParts, Command, Instruction};
use crate::turns::{TurnOut, Turns};
use crate::voice_port::AppVoice;
use crate::{AgentEvent, AgentHandle, AgentState, Link, ToolsAccess};

/// A relayed event: its channel (`"mcp"`, `"agent"`) and wrapped JSON.
pub(crate) type Out = (&'static str, String);

/// An event and its tab, serialized as is (no intermediate `Value`).
#[derive(Serialize)]
struct Envelope<'a, E> {
    tab: &'a str,
    event: &'a E,
}

/// Sends the JSON `{"tab": …, "event": …}` on `channel`; an event that does not serialize
/// is logged and skipped (never an empty string).
pub(crate) fn emit(out: &mpsc::UnboundedSender<Out>, channel: &'static str, tab: &str, event: &impl Serialize) {
    match serde_json::to_string(&Envelope { tab, event }) {
        Ok(json) => {
            let _ = out.send((channel, json));
        }
        Err(e) => tracing::warn!(tab, channel, error = %e, "évènement non sérialisable : sauté"),
    }
}

/// A tab's draft: its text (`None`: closed) and its attachments.
#[derive(Debug, Clone, Default)]
pub(crate) struct Draft {
    /// The written text; `None`: the draft is closed.
    pub text: Option<String>,
    /// The attached images and documents, in the order they were attached.
    pub attachments: Vec<AttachmentRef>,
}

/// What the stack uses of a tab, without keeping the table locked.
#[derive(Debug, Clone)]
pub(crate) struct TabLink {
    tab: String,
    pub hub: Arc<SurfaceHub>,
    /// The tab's keeper: the only way to its conversation.
    pub keeper: Keeper,
    /// To the task that starts the tools then the agent, and relays the commands to it.
    cmds: mpsc::UnboundedSender<Command>,
    status: Arc<Mutex<AgentState>>,
    /// Why the tab's last turn ran on the local agent; `None`: on the selected one.
    pub fallback: Arc<Mutex<Option<String>>>,
    /// The tab's draft, held for all its views.
    draft: Arc<Mutex<Draft>>,
    /// The tab's session: the store of its attachments.
    pub session: TabSession,
    /// Serializes attaches: the draft's caps and the store's keep list are read and written in one step.
    ///
    /// Lock order: this one, then the session (`TabSession::with`), then the draft; nothing else takes it.
    pub paste: Arc<Mutex<()>>,
}

impl TabLink {
    pub fn send(&self, cmd: Command) {
        let _ = self.cmds.send(cmd);
    }

    /// Keeps the draft's text and broadcasts it; closing it (`None`) drops its attachments.
    ///
    /// Broadcast to every view of the tab under the same lock: broadcasts follow the order of the writes.
    pub fn set_draft(&self, out: &mpsc::UnboundedSender<Out>, text: Option<String>, origin: Option<String>) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        if text.is_none() {
            draft.attachments.clear();
        }
        draft.text.clone_from(&text);
        let attachments = draft.attachments.clone();
        emit(out, "agent", &self.tab, &AgentEvent::Draft { text, origin, attachments });
    }

    /// Puts `text` in an empty draft and broadcasts it; returns whether it was set.
    ///
    /// A draft holding non-blank text or attachments is kept as is; a closed one (`None`) counts as
    /// empty and opens. Takes only the draft lock, once, so the check and the write are one step.
    pub fn prefill_draft(&self, out: &mpsc::UnboundedSender<Out>, text: String) -> bool {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        let held = draft.text.as_deref().is_some_and(|t| !t.trim().is_empty()) || !draft.attachments.is_empty();
        if held {
            return false;
        }
        draft.text = Some(text.clone());
        let event = AgentEvent::Draft { text: Some(text), origin: None, attachments: draft.attachments.clone() };
        emit(out, "agent", &self.tab, &event);
        true
    }

    /// Adds an attachment to the draft (once), opening it empty if it was closed.
    pub fn add_attachment(&self, out: &mpsc::UnboundedSender<Out>, attachment: AttachmentRef) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        if !draft.attachments.iter().any(|i| i.id() == attachment.id()) {
            draft.attachments.push(attachment);
        }
        let text = Some(draft.text.take().unwrap_or_default());
        draft.text.clone_from(&text);
        let event = AgentEvent::Draft { text, origin: None, attachments: draft.attachments.clone() };
        emit(out, "agent", &self.tab, &event);
    }

    /// Removes attachment `id` from the draft.
    pub fn remove_attachment(&self, out: &mpsc::UnboundedSender<Out>, id: &str) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        draft.attachments.retain(|i| i.id() != id);
        let event =
            AgentEvent::Draft { text: draft.text.clone(), origin: None, attachments: draft.attachments.clone() };
        emit(out, "agent", &self.tab, &event);
    }

    /// The draft's attachments.
    pub fn draft_attachments(&self) -> Vec<AttachmentRef> {
        self.draft.lock().unwrap_or_else(PoisonError::into_inner).attachments.clone()
    }

    /// Sends the agent's state and the draft; the hub replays the turns' surfaces.
    pub fn replay(&self, out: &mpsc::UnboundedSender<Out>) {
        let state = *self.status.lock().unwrap_or_else(PoisonError::into_inner);
        emit(out, "agent", &self.tab, &AgentEvent::Status { state });
        let reason = self.fallback.lock().unwrap_or_else(PoisonError::into_inner).clone();
        if reason.is_some() {
            emit(out, "agent", &self.tab, &AgentEvent::Fallback { reason });
        }
        // Under the lock, as in `set_draft`: a stale draft is never broadcast after a newer one.
        let draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        let event =
            AgentEvent::Draft { text: draft.text.clone(), origin: None, attachments: draft.attachments.clone() };
        emit(out, "agent", &self.tab, &event);
    }
}

/// What the stack shares with each tab: settings, voice and agent choice, other tabs and workspace.
#[derive(Debug, Clone)]
pub(crate) struct TabPorts {
    /// The settings `set_setting` reads and writes.
    pub settings: Arc<AppSettings>,
    /// The app's dictation and agent: the settings box and its guards.
    pub voice: Arc<AppVoice>,
    /// Other tabs: where `open_pack` and `open_theme` pass the user's request (a draft), `help`
    /// pages, and `import` starts the fine-tuning turn.
    pub tabs: Arc<dyn TabPort>,
    /// The host's workspace: the focused tab, an import's new tab.
    pub workspace: Arc<dyn AgentWorkspace>,
    /// The app itself: the `update` tool's checks.
    pub app: Arc<dyn AppPort>,
    /// Where the tab's actor sends its ended turns' quit or install.
    pub after: AfterTx,
    /// The read-aloud: the tab's relay hands it each surface that opens; `None` without a voice.
    pub reader: Option<crate::reader::ReaderTx>,
}

/// A started tab; dropping it stops its agent and its tools.
#[derive(Debug)]
pub(crate) struct TabEntry {
    pub link: TabLink,
    cancel: CancellationToken,
}

impl TabEntry {
    /// Starts, in `rt`, the keeper, the relays, then the tools and the agent;
    /// commands wait in a queue until the agent is there.
    ///
    /// The tools are built later, in [`Agent::run`]: they need the tab's kind, read off
    /// this thread, which holds the stack's table lock.
    pub fn start(
        session: TabSession,
        platform: &Platform,
        ports: TabPorts,
        link: Link,
        rt: &Handle,
        out: mpsc::UnboundedSender<Out>,
    ) -> Self {
        let tab = session.tab().to_owned();
        let hub = SurfaceHub::new(session.clone(), platform.live_localizer.clone(), ports.voice.clone());
        let cancel = CancellationToken::new();
        let (cmds, rx) = mpsc::unbounded_channel();
        let status = Arc::new(Mutex::new(AgentState::Idle));
        let fallback = Arc::new(Mutex::new(None));
        let closed = cancel.clone();
        let mcp = hub.subscribe();
        // After the relay subscribed: the list the thread replay sends is never lost.
        let keeper = {
            let _rt = rt.enter();
            Keeper::spawn(session.clone(), hub.clone())
        };
        let relay_to = McpRelay { tab: tab.clone(), hub: Arc::downgrade(&hub), out: out.clone(), reader: ports.reader };
        rt.spawn(relay_mcp(relay_to, mcp, closed.clone()));
        let tab_session = session.clone();
        let agent = Agent {
            session,
            hub: hub.clone(),
            link,
            localizer: platform.live_localizer.clone(),
            keeper: keeper.clone(),
            voice: ports.voice,
            tabs: ports.tabs,
            workspace: ports.workspace,
            app: ports.app,
            after: ports.after,
            prompt_limits: platform.settings.paste.prompt_limits(),
        };
        let relay = Relay {
            tab: tab.clone(),
            hub: hub.clone(),
            keeper: keeper.clone(),
            localizer: platform.live_localizer.clone(),
            turns: Turns::default(),
            status: status.clone(),
            fallback: fallback.clone(),
            out,
            closed,
        };
        rt.spawn(agent.run(ports.settings, platform.clone(), cancel.clone(), rx, relay));
        let draft = Arc::new(Mutex::new(Draft::default()));
        Self {
            link: TabLink {
                tab,
                hub,
                keeper,
                cmds,
                status,
                fallback,
                draft,
                session: tab_session,
                paste: Arc::default(),
            },
            cancel,
        }
    }
}

impl Drop for TabEntry {
    fn drop(&mut self) {
        // A pending answer must never block the tool server's shutdown, nor must a question
        // asked afterwards (a late ACP permission).
        self.link.hub.close();
        self.cancel.cancel();
    }
}

/// Where a tab's hub events go: the interface, and the read-aloud.
struct McpRelay {
    tab: String,
    hub: std::sync::Weak<SurfaceHub>,
    out: mpsc::UnboundedSender<Out>,
    reader: Option<crate::reader::ReaderTx>,
}

/// Relays the hub. When the relay falls behind, a `changed` makes the interface reread
/// everything and `surfaces` gives it the whole list again. The hub is held weakly: the relay
/// must not keep the hub (and its sender) alive, or `recv` would never return `Closed`. Stops
/// when the tab closes (`closed`).
async fn relay_mcp(to: McpRelay, mut rx: broadcast::Receiver<McpEvent>, closed: CancellationToken) {
    let McpRelay { tab, hub, out, reader } = to;
    loop {
        let received = tokio::select! {
            biased;
            () = closed.cancelled() => return,
            r = rx.recv() => r,
        };
        match received {
            Ok(e) => {
                if let Some(reader) = &reader
                    && let Some(heard) = crate::reader::heard_of(&tab, &hub, &e)
                {
                    reader.hear(heard);
                }
                emit(&out, "mcp", &tab, &e);
            }
            Err(RecvError::Lagged(_)) => {
                emit(&out, "mcp", &tab, &McpEvent::Changed);
                // The events still queued replay on top, safely: `surface` and
                // `surface_patch` write whole keys (the replay converges) and `close_surface` is
                // idempotent. A lost page command (`present`, `interface`, `open_history`) is not
                // replayed: its tool already answered (the history waits to be opened, and says
                // so otherwise).
                let Some(hub) = hub.upgrade() else { return };
                let list = hub.visible_surfaces();
                emit(&out, "mcp", &tab, &McpEvent::Surfaces { list });
            }
            Err(RecvError::Closed) => return,
        }
    }
}

/// The relay of a tab's agent events.
///
/// State and availability go to the interface, the rest updates the turns' surfaces; nothing
/// goes once the tab is closed (`closed`), even if its agent has not finished stopping.
struct Relay {
    tab: String,
    hub: Arc<SurfaceHub>,
    /// Writes the thread of an ended turn.
    keeper: Keeper,
    localizer: oku_slides_i18n::LiveLocalizer,
    turns: Turns,
    status: Arc<Mutex<AgentState>>,
    fallback: Arc<Mutex<Option<String>>>,
    out: mpsc::UnboundedSender<Out>,
    closed: CancellationToken,
}

impl Relay {
    fn emit(&mut self, e: &AgentEvent) {
        if self.closed.is_cancelled() {
            return;
        }
        match e {
            AgentEvent::Status { state } => {
                *self.status.lock().unwrap_or_else(PoisonError::into_inner) = *state;
                emit(&self.out, "agent", &self.tab, e);
            }
            AgentEvent::Availability { .. } => emit(&self.out, "agent", &self.tab, e),
            AgentEvent::Fallback { reason } => {
                reason.clone_into(&mut *self.fallback.lock().unwrap_or_else(PoisonError::into_inner));
                emit(&self.out, "agent", &self.tab, e);
            }
            _ => {}
        }
        // Every event, `Status` included, goes to the turns (idle ends a turn whose end was lost).
        for out in self.turns.apply(e, Instant::now(), &self.localizer.current()) {
            match out {
                TurnOut::Show(surface) => self.hub.emit(McpEvent::surface(surface)),
                TurnOut::ShowAloud(surface, spoken) => self.hub.emit(McpEvent::surface_aloud(surface, Some(spoken))),
                TurnOut::Patch { id, messages } => self.hub.emit(McpEvent::SurfacePatch { id, messages }),
                TurnOut::Evict(n) => self.hub.close_turn(n),
                TurnOut::Ended { turn, narration } => {
                    self.hub.end_turn(turn, &narration);
                    self.keeper.send(Keep::Thread(turn));
                }
            }
        }
    }

    async fn run(mut self, mut rx: broadcast::Receiver<AgentEvent>) {
        loop {
            let received = tokio::select! {
                biased;
                () = self.closed.cancelled() => return,
                r = rx.recv() => r,
            };
            match received {
                Ok(e) => self.emit(&e),
                Err(RecvError::Lagged(n)) => {
                    // Events lost: the turn ends when the agent is idle again (see `Turns`).
                    tracing::debug!(tab = %self.tab, lost = n, "relais de l'agent en retard");
                }
                Err(RecvError::Closed) => return,
            }
        }
    }
}

/// What starts the tab's agent once its tools are ready.
struct Agent {
    session: TabSession,
    hub: Arc<SurfaceHub>,
    link: Link,
    localizer: oku_slides_i18n::LiveLocalizer,
    keeper: Keeper,
    /// Counts the tab's requests for the settings box.
    voice: Arc<AppVoice>,
    /// The other tabs' draft, page and fine-tuning turn, for the tools.
    tabs: Arc<dyn TabPort>,
    /// The host's workspace, for the tools.
    workspace: Arc<dyn AgentWorkspace>,
    /// The host's app port, for the tools.
    app: Arc<dyn AppPort>,
    /// Where the actor sends its ended turns' quit or install.
    after: AfterTx,
    /// Bounds of the images sent to the agent as image blocks.
    prompt_limits: PromptImageLimits,
}

impl Agent {
    /// Reads the tab's kind, starts its tools then its agent, and relays the commands to it
    /// until the tab closes (the agent is then dropped, so stopped).
    ///
    /// Without the kind (tab gone, task lost) or a tool server, each prompt gets the
    /// tools-unavailable error instead ([`Agent::serve_without_tools`]).
    async fn run(
        self,
        settings: Arc<dyn SettingsPort>,
        platform: Platform,
        cancel: CancellationToken,
        rx: mpsc::UnboundedReceiver<Command>,
        relay: Relay,
    ) {
        let (server, kind) = match self.start_tools(settings, &platform, &cancel).await {
            Ok(started) => started,
            Err(say) => return self.serve_without_tools(say, cancel, rx, relay).await,
        };
        // Tab closed while starting: the queued commands are moot.
        if cancel.is_cancelled() {
            return;
        }
        self.serve(&server, kind, cancel, rx, relay).await;
    }

    /// The tab's kind, read once off the async thread, and a tool server for that kind.
    ///
    /// # Errors
    ///
    /// The localized message of the failure, in the language of each use: the session's
    /// error for a gone tab, `agent-tools-unavailable` for a lost task or a server that
    /// did not start (the details go to the log).
    async fn start_tools(
        &self,
        settings: Arc<dyn SettingsPort>,
        platform: &Platform,
        cancel: &CancellationToken,
    ) -> Result<(ToolServer, DocumentKind), Say> {
        let read = {
            let session = self.session.clone();
            tokio::task::spawn_blocking(move || session.document_kind()).await
        };
        let kind = match read {
            Ok(Ok(kind)) => kind,
            Ok(Err(e)) => {
                tracing::warn!(tab = %self.session.tab(), error = %e.one_line(), "tab document kind unreadable");
                return Err(Box::new(move |l: &Localizer| e.localize(l)));
            }
            Err(e) => {
                tracing::warn!(tab = %self.session.tab(), error = %e, "tab document kind read: task lost");
                return Err(Box::new(|l: &Localizer| l.tr("agent-tools-unavailable")));
            }
        };
        let tools = Tools::new(
            self.session.clone(),
            self.hub.clone(),
            HostPorts { settings, tabs: self.tabs.clone(), workspace: self.workspace.clone(), app: self.app.clone() },
            platform,
            kind,
        );
        match ToolServer::start(tools, cancel.clone()).await {
            Ok(server) => Ok((server, kind)),
            Err(e) => {
                tracing::warn!(tab = %self.session.tab(), error = %e, "tab tools unavailable");
                Err(Box::new(move |l: &Localizer| e.localize(l)))
            }
        }
    }

    /// Without tools: each prompt gets `say`'s error in the thread, the rest is moot.
    async fn serve_without_tools(
        self,
        say: Say,
        cancel: CancellationToken,
        mut rx: mpsc::UnboundedReceiver<Command>,
        mut relay: Relay,
    ) {
        // The thread read first: the error's turns are numbered after its own.
        self.keeper.restored().await;
        // Each prompt gets the error in the thread, in the language of the moment.
        while let Some(cmd) = next(&cancel, &mut rx).await {
            if let Command::Prompt(Instruction { text, attachments, origin, .. }) = cmd {
                let n = self.hub.begin_turn_from(origin);
                relay.emit(&AgentEvent::UserText {
                    turn: n,
                    text,
                    images: attachments.iter().filter_map(AttachmentRef::shown).collect(),
                });
                let message = say(&self.localizer.current());
                relay.emit(&AgentEvent::Error { turn: n, message, spoken: SpokenFailure::NoTools });
            }
        }
    }

    /// Starts the agent on `server`'s tools, for a tab of `kind`, and relays the commands to it.
    async fn serve(
        self,
        server: &ToolServer,
        kind: DocumentKind,
        cancel: CancellationToken,
        mut rx: mpsc::UnboundedReceiver<Command>,
        relay: Relay,
    ) {
        let access = ToolsAccess { url: server.url().to_owned(), authorization: server.authorization() };
        let parts = ActorParts {
            tab: self.session,
            hub: self.hub,
            tools: access,
            link: self.link,
            localizer: self.localizer,
            keeper: self.keeper,
            prompt_limits: self.prompt_limits,
            kind,
            after: Some(self.after),
        };
        let agent = AgentHandle::spawn_counted(parts, self.voice);
        // Subscribed before the first command: no event of the agent is lost.
        tokio::spawn(relay.run(agent.subscribe()));
        while let Some(cmd) = next(&cancel, &mut rx).await {
            match cmd {
                Command::Prompt(instruction) => agent.instruct(instruction),
                Command::Cancel => agent.cancel(),
                Command::NewConversation => agent.new_conversation(),
            }
        }
    }
}

/// A failure's message, worded in the language current at each use.
type Say = Box<dyn Fn(&Localizer) -> String + Send>;

/// The next command; `None` once the tab closed, even with commands still
/// queued (a dropped sender gives `None` only after the queue).
async fn next(closed: &CancellationToken, rx: &mut mpsc::UnboundedReceiver<Command>) -> Option<Command> {
    tokio::select! {
        biased;
        () = closed.cancelled() => None,
        cmd = rx.recv() => cmd,
    }
}

#[cfg(test)]
#[path = "tab_entry_tests.rs"]
mod tab_entry_tests;
