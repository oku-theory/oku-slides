//! Un onglet de la pile : son hub de surfaces, ses outils MCP, son agent et
//! ses tours, avec les relais de ses évènements vers l'interface.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use oku_slides_i18n::Localize;
use oku_slides_mcp::{McpEvent, SurfaceHub, ToolServer, Tools};
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::Platform;
use oku_slides_session::agent::{AgentScope, AttachmentRef, TabSession};
use serde::Serialize;
use tokio::runtime::Handle;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::keeper::{Keep, Keeper};
use crate::settings_port::AppSettings;
use crate::tab_agent::{Command, Instruction};
use crate::turns::{TurnOut, Turns};
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
    /// Le gardien de l'onglet : seul passage vers ses conversations.
    pub keeper: Keeper,
    /// Vers la tâche qui démarre les outils puis l'agent, et lui relaie les commandes.
    cmds: mpsc::UnboundedSender<Command>,
    status: Arc<Mutex<(AgentState, Option<AgentScope>)>>,
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
        let (state, scope) = self.status.lock().unwrap_or_else(PoisonError::into_inner).clone();
        emit(out, "agent", &self.tab, &AgentEvent::Status { state, scope });
        // Sous le verrou, comme `set_draft` : jamais un brouillon périmé diffusé après un plus récent.
        let draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        let event = AgentEvent::Draft { text: draft.text.clone(), origin: None, images: draft.images.clone() };
        emit(out, "agent", &self.tab, &event);
    }
}

/// Un onglet démarré ; le lâcher arrête son agent et ses outils.
#[derive(Debug)]
pub(crate) struct TabEntry {
    pub link: TabLink,
    cancel: CancellationToken,
}

impl TabEntry {
    /// Démarre, dans `rt`, le gardien, les relais, puis les outils et l'agent ;
    /// les commandes attendent en file que l'agent soit là.
    pub fn start(
        session: TabSession,
        platform: &Platform,
        settings: Arc<AppSettings>,
        link: Link,
        rt: &Handle,
        out: mpsc::UnboundedSender<Out>,
    ) -> Self {
        let tab = session.tab().to_owned();
        let hub = SurfaceHub::new(session.clone(), platform.live_localizer.clone());
        let tools = Tools::new(session.clone(), hub.clone(), settings, platform);
        let cancel = CancellationToken::new();
        let (cmds, rx) = mpsc::unbounded_channel();
        let status = Arc::new(Mutex::new((AgentState::Idle, None)));
        let closed = cancel.clone();
        let mcp = hub.subscribe();
        // Après l'abonnement du relais : la liste qu'envoie la relecture des fils n'est jamais perdue.
        let keeper = {
            let _rt = rt.enter();
            Keeper::spawn(session.clone(), hub.clone())
        };
        rt.spawn(relay_mcp(tab.clone(), mcp, Arc::downgrade(&hub), keeper.clone(), out.clone(), closed.clone()));
        let tab_session = session.clone();
        let agent = Agent {
            session,
            hub: hub.clone(),
            link,
            localizer: platform.live_localizer.clone(),
            keeper: keeper.clone(),
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
        rt.spawn(agent.run(tools, cancel.clone(), rx, relay));
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
///
/// Un widget écrit par les outils de l'agent fait appliquer au gardien les renommages
/// notés ([`Keep::Sync`]), comme un changement du deck venu de l'interface.
async fn relay_mcp(
    tab: String,
    mut rx: broadcast::Receiver<McpEvent>,
    hub: std::sync::Weak<SurfaceHub>,
    keeper: Keeper,
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
            Ok(e) => {
                if matches!(e, McpEvent::WidgetTouched { .. }) {
                    keeper.send(Keep::Sync);
                }
                emit(&out, "mcp", &tab, &e);
            }
            Err(RecvError::Lagged(_)) => {
                emit(&out, "mcp", &tab, &McpEvent::Changed);
                // Les évènements encore en file rejouent ensuite par-dessus : sans
                // risque : `surface` et `surface_patch` écrivent des clés entières (le rejeu
                // converge) et `close_surface` est idempotent. Une commande de la page perdue
                // (`present`, `interface`, `open_history`) ne se rejoue pas : son outil a déjà
                // répondu (l'historique, lui, attend d'être ouvert, et le dit sinon).
                let Some(hub) = hub.upgrade() else { return };
                emit(&out, "mcp", &tab, &McpEvent::Viewing { scope: hub.viewing() });
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
    status: Arc<Mutex<(AgentState, Option<AgentScope>)>>,
    out: mpsc::UnboundedSender<Out>,
    closed: CancellationToken,
}

impl Relay {
    fn emit(&mut self, e: &AgentEvent) {
        if self.closed.is_cancelled() {
            return;
        }
        match e {
            AgentEvent::Status { state, scope } => {
                *self.status.lock().unwrap_or_else(PoisonError::into_inner) = (*state, scope.clone());
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

/// De quoi démarrer l'agent de l'onglet une fois ses outils prêts.
struct Agent {
    session: TabSession,
    hub: Arc<SurfaceHub>,
    link: Link,
    localizer: oku_slides_i18n::LiveLocalizer,
    keeper: Keeper,
    /// Bornes des images envoyées en bloc image à l'agent.
    prompt_limits: PromptImageLimits,
}

impl Agent {
    /// Démarre les outils puis l'agent, et lui relaie les commandes jusqu'à
    /// la fermeture de l'onglet (l'agent est alors lâché, donc arrêté).
    async fn run(
        self,
        tools: Tools,
        cancel: CancellationToken,
        mut rx: mpsc::UnboundedReceiver<Command>,
        mut relay: Relay,
    ) {
        let server = match ToolServer::start(tools, cancel.clone()).await {
            Ok(server) => server,
            Err(e) => {
                tracing::warn!(tab = %self.session.tab(), error = %e, "outils de l'onglet indisponibles");
                // Les fils relus d'abord : les tours de l'erreur sont numérotés après les leurs.
                self.keeper.restored().await;
                // Chaque consigne reçoit l'erreur dans le fil, dans la langue du moment ; le reste est sans objet.
                while let Some(cmd) = next(&cancel, &mut rx).await {
                    if let Command::Prompt(Instruction { scope, text, images, .. }) = cmd {
                        let n = self.hub.begin_turn(scope.clone());
                        relay.emit(&AgentEvent::UserText {
                            turn: n,
                            scope,
                            text,
                            images: images.iter().map(AttachmentRef::shown).collect(),
                        });
                        let message = e.localize(&self.localizer.current());
                        relay.emit(&AgentEvent::Error { turn: n, message });
                    }
                }
                return;
            }
        };
        // Onglet fermé pendant le démarrage : les commandes en file sont sans objet.
        if cancel.is_cancelled() {
            return;
        }
        let access = ToolsAccess { url: server.url().to_owned(), authorization: server.authorization() };
        let agent = AgentHandle::spawn(
            self.session,
            self.hub,
            access,
            self.link,
            self.localizer,
            self.keeper,
            self.prompt_limits,
        );
        // Abonné avant la première commande : aucun évènement de l'agent n'est perdu.
        tokio::spawn(relay.run(agent.subscribe()));
        while let Some(cmd) = next(&cancel, &mut rx).await {
            match cmd {
                Command::Prompt(instruction) => agent.instruct(instruction),
                Command::Cancel => agent.cancel(),
                Command::NewConversation { scope } => agent.new_conversation(scope),
            }
        }
    }
}

/// La commande suivante ; `None` une fois l'onglet fermé, même s'il en reste
/// en file (l'émetteur lâché ne fait rendre `None` qu'après la file).
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
