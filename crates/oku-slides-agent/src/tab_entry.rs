//! A tab of the stack: its surface hub, its MCP tools, its agent and its
//! turns, with the relays of its events to the interface.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::{DraftPort, McpEvent, SettingsPort, SurfaceHub, ToolServer, Tools};
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::{AttachmentRef, TabSession};
use oku_slides_session::{DocumentKind, Platform};
use serde::Serialize;
use tokio::runtime::Handle;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::keeper::{Keep, Keeper};
use crate::settings_port::AppSettings;
use crate::tab_agent::{ActorParts, Command, Instruction};
use crate::turns::{TurnOut, Turns};
use crate::voice_port::AppVoice;
use crate::{AgentEvent, AgentHandle, AgentState, Link, ToolsAccess};

/// Un évènement relayé : canal (`"mcp"`, `"agent"`) et JSON enveloppé.
pub(crate) type Out = (&'static str, String);

/// Un évènement et son onglet, sérialisé tel quel (sans `Value` intermédiaire).
#[derive(Serialize)]
struct Envelope<'a, E> {
    tab: &'a str,
    event: &'a E,
}

/// Envoie sur `channel` le JSON `{"tab": …, "event": …}` ; un évènement
/// qui ne se sérialise pas est journalisé et sauté (jamais une chaîne vide).
pub(crate) fn emit(out: &mpsc::UnboundedSender<Out>, channel: &'static str, tab: &str, event: &impl Serialize) {
    match serde_json::to_string(&Envelope { tab, event }) {
        Ok(json) => {
            let _ = out.send((channel, json));
        }
        Err(e) => tracing::warn!(tab, channel, error = %e, "évènement non sérialisable : sauté"),
    }
}

/// Le brouillon d'un onglet : son texte (`None` : fermé) et ses images collées.
#[derive(Debug, Clone, Default)]
pub(crate) struct Draft {
    /// Le texte écrit ; `None` : brouillon fermé.
    pub text: Option<String>,
    /// Les images collées, dans l'ordre du collage.
    pub images: Vec<AttachmentRef>,
}

/// Ce que la pile utilise d'un onglet, sans garder la table verrouillée.
#[derive(Debug, Clone)]
pub(crate) struct TabLink {
    tab: String,
    pub hub: Arc<SurfaceHub>,
    /// Le gardien de l'onglet : seul passage vers sa conversation.
    pub keeper: Keeper,
    /// Vers la tâche qui démarre les outils puis l'agent, et lui relaie les commandes.
    cmds: mpsc::UnboundedSender<Command>,
    status: Arc<Mutex<AgentState>>,
    /// Le brouillon de l'onglet, tenu pour toutes ses vues.
    draft: Arc<Mutex<Draft>>,
    /// La session de l'onglet : magasin des images collées.
    pub session: TabSession,
    /// Sérialise les collages : le plafond du brouillon et la liste à garder du magasin se lisent et s'écrivent d'un bloc.
    ///
    /// Ordre des verrous : celui-ci, puis la session (`TabSession::with`), puis le brouillon ; rien d'autre ne le prend.
    pub paste: Arc<Mutex<()>>,
}

impl TabLink {
    pub fn send(&self, cmd: Command) {
        let _ = self.cmds.send(cmd);
    }

    /// Garde le texte du brouillon et le diffuse ; le fermer (`None`) en retire les images.
    ///
    /// Diffusé à toutes les vues de l'onglet, sous le même verrou : l'ordre des diffusions est celui des écritures.
    pub fn set_draft(&self, out: &mpsc::UnboundedSender<Out>, text: Option<String>, origin: Option<String>) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        if text.is_none() {
            draft.images.clear();
        }
        draft.text.clone_from(&text);
        let images = draft.images.clone();
        emit(out, "agent", &self.tab, &AgentEvent::Draft { text, origin, images });
    }

    /// Puts `text` in an empty draft and broadcasts it; returns whether it was set.
    ///
    /// A draft holding non-blank text or images is kept as is; a closed one (`None`) counts as
    /// empty and opens. Takes only the draft lock, once, so the check and the write are one step.
    pub fn prefill_draft(&self, out: &mpsc::UnboundedSender<Out>, text: String) -> bool {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        let held = draft.text.as_deref().is_some_and(|t| !t.trim().is_empty()) || !draft.images.is_empty();
        if held {
            return false;
        }
        draft.text = Some(text.clone());
        let event = AgentEvent::Draft { text: Some(text), origin: None, images: draft.images.clone() };
        emit(out, "agent", &self.tab, &event);
        true
    }

    /// Ajoute une image au brouillon (une fois), en l'ouvrant vide s'il était fermé.
    pub fn add_image(&self, out: &mpsc::UnboundedSender<Out>, image: AttachmentRef) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        if !draft.images.iter().any(|i| i.id == image.id) {
            draft.images.push(image);
        }
        let text = Some(draft.text.take().unwrap_or_default());
        draft.text.clone_from(&text);
        emit(out, "agent", &self.tab, &AgentEvent::Draft { text, origin: None, images: draft.images.clone() });
    }

    /// Retire l'image `id` du brouillon.
    pub fn remove_image(&self, out: &mpsc::UnboundedSender<Out>, id: &str) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        draft.images.retain(|i| i.id != id);
        let event = AgentEvent::Draft { text: draft.text.clone(), origin: None, images: draft.images.clone() };
        emit(out, "agent", &self.tab, &event);
    }

    /// Les images du brouillon.
    pub fn draft_images(&self) -> Vec<AttachmentRef> {
        self.draft.lock().unwrap_or_else(PoisonError::into_inner).images.clone()
    }

    /// Envoie l'état de l'agent et le brouillon ; les surfaces des tours sont rejouées par le hub.
    pub fn replay(&self, out: &mpsc::UnboundedSender<Out>) {
        let state = *self.status.lock().unwrap_or_else(PoisonError::into_inner);
        emit(out, "agent", &self.tab, &AgentEvent::Status { state });
        // Sous le verrou, comme `set_draft` : jamais un brouillon périmé diffusé après un plus récent.
        let draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        let event = AgentEvent::Draft { text: draft.text.clone(), origin: None, images: draft.images.clone() };
        emit(out, "agent", &self.tab, &event);
    }
}

/// What the stack shares with each tab: settings, voice and agent choice, and the drafts port.
#[derive(Debug, Clone)]
pub(crate) struct TabPorts {
    /// The settings `set_setting` reads and writes.
    pub settings: Arc<AppSettings>,
    /// The app's dictation and agent: the settings box and its guards.
    pub voice: Arc<AppVoice>,
    /// Where `open_pack` and `open_theme` pass the user's request: another tab's draft.
    pub drafts: Arc<dyn DraftPort>,
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
        let closed = cancel.clone();
        let mcp = hub.subscribe();
        // Après l'abonnement du relais : la liste qu'envoie la relecture des fils n'est jamais perdue.
        let keeper = {
            let _rt = rt.enter();
            Keeper::spawn(session.clone(), hub.clone())
        };
        rt.spawn(relay_mcp(tab.clone(), mcp, Arc::downgrade(&hub), out.clone(), closed.clone()));
        let tab_session = session.clone();
        let agent = Agent {
            session,
            hub: hub.clone(),
            link,
            localizer: platform.live_localizer.clone(),
            keeper: keeper.clone(),
            voice: ports.voice,
            drafts: ports.drafts,
            prompt_limits: platform.settings.paste.prompt_limits(),
        };
        let relay = Relay {
            tab: tab.clone(),
            hub: hub.clone(),
            keeper: keeper.clone(),
            localizer: platform.live_localizer.clone(),
            turns: Turns::default(),
            status: status.clone(),
            out,
            closed,
        };
        rt.spawn(agent.run(ports.settings, platform.clone(), cancel.clone(), rx, relay));
        let draft = Arc::new(Mutex::new(Draft::default()));
        Self {
            link: TabLink { tab, hub, keeper, cmds, status, draft, session: tab_session, paste: Arc::default() },
            cancel,
        }
    }
}

impl Drop for TabEntry {
    fn drop(&mut self) {
        // Une réponse en attente ne doit jamais bloquer l'arrêt du serveur
        // d'outils ; une question posée ensuite (permission ACP tardive) non plus.
        self.link.hub.close();
        self.cancel.cancel();
    }
}

/// Relaie le hub. Si le relais a pris du retard, un `changed` fait tout
/// relire à l'interface et `surfaces` lui redonne la liste entière. Faible :
/// le relais ne doit pas garder le hub (et son émetteur) en vie, sinon `recv`
/// ne rendrait jamais `Closed`. S'arrête à la fermeture de l'onglet (`closed`).
async fn relay_mcp(
    tab: String,
    mut rx: broadcast::Receiver<McpEvent>,
    hub: std::sync::Weak<SurfaceHub>,
    out: mpsc::UnboundedSender<Out>,
    closed: CancellationToken,
) {
    loop {
        let received = tokio::select! {
            biased;
            () = closed.cancelled() => return,
            r = rx.recv() => r,
        };
        match received {
            Ok(e) => emit(&out, "mcp", &tab, &e),
            Err(RecvError::Lagged(_)) => {
                emit(&out, "mcp", &tab, &McpEvent::Changed);
                // Les évènements encore en file rejouent ensuite par-dessus : sans
                // risque : `surface` et `surface_patch` écrivent des clés entières (le rejeu
                // converge) et `close_surface` est idempotent. Une commande de la page perdue
                // (`present`, `interface`, `open_history`) ne se rejoue pas : son outil a déjà
                // répondu (l'historique, lui, attend d'être ouvert, et le dit sinon).
                let Some(hub) = hub.upgrade() else { return };
                let list = hub.visible_surfaces();
                emit(&out, "mcp", &tab, &McpEvent::Surfaces { list });
            }
            Err(RecvError::Closed) => return,
        }
    }
}

/// Le relais des évènements de l'agent d'un onglet.
///
/// L'état et la disponibilité partent à l'interface, le reste met à jour les surfaces des tours ;
/// plus rien une fois l'onglet fermé (`closed`), même si son agent n'a pas encore fini de s'arrêter.
struct Relay {
    tab: String,
    hub: Arc<SurfaceHub>,
    /// Écrit le fil d'un tour fini.
    keeper: Keeper,
    localizer: oku_slides_i18n::LiveLocalizer,
    turns: Turns,
    status: Arc<Mutex<AgentState>>,
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
            _ => {}
        }
        // Tous les évènements, `Status` compris, passent aux tours (le repos clôt un tour dont la fin s'est perdue).
        for out in self.turns.apply(e, Instant::now(), &self.localizer.current()) {
            match out {
                TurnOut::Show(surface) => self.hub.emit(McpEvent::Surface { surface }),
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
                    // Évènements perdus : le tour se clôt au retour de l'agent au repos (voir `Turns`).
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
    /// The other tabs' drafts, for the tools' `open_pack` and `open_theme`.
    drafts: Arc<dyn DraftPort>,
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
        let tools = Tools::new(self.session.clone(), self.hub.clone(), settings, self.drafts.clone(), platform, kind);
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
            if let Command::Prompt(Instruction { text, images, .. }) = cmd {
                let n = self.hub.begin_turn();
                relay.emit(&AgentEvent::UserText {
                    turn: n,
                    text,
                    images: images.iter().map(AttachmentRef::shown).collect(),
                });
                let message = say(&self.localizer.current());
                relay.emit(&AgentEvent::Error { turn: n, message });
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
