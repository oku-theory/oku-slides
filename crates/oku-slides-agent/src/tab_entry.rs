//! Un onglet de la pile : son hub de surfaces, ses outils MCP, son agent et
//! son fil gardé, avec les relais de ses évènements vers l'interface.

use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_i18n::Localize;
use oku_slides_mcp::{McpEvent, SurfaceHub, ToolServer, Tools};
use oku_slides_session::Platform;
use oku_slides_session::agent::TabSession;
use serde::Serialize;
use tokio::runtime::Handle;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::settings_port::AppSettings;
use crate::tab_agent::Command;
use crate::thread_log::ThreadLog;
use crate::{AgentEvent, AgentHandle, Link, ToolsAccess};

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

/// Ce que la pile utilise d'un onglet, sans garder la table verrouillée.
#[derive(Debug, Clone)]
pub(crate) struct TabLink {
    tab: String,
    pub hub: Arc<SurfaceHub>,
    /// Vers la tâche qui démarre les outils puis l'agent, et lui relaie les commandes.
    cmds: mpsc::UnboundedSender<Command>,
    thread: Arc<Mutex<ThreadLog>>,
}

impl TabLink {
    pub fn send(&self, cmd: Command) {
        let _ = self.cmds.send(cmd);
    }

    /// Envoie le fil gardé. Sous le verrou du fil, comme le relais de
    /// l'agent : aucun évènement n'est à la fois dans le fil et après lui.
    pub fn replay(&self, out: &mpsc::UnboundedSender<Out>) {
        let thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner);
        emit(out, "agent", &self.tab, &AgentEvent::Thread { entries: thread.entries() });
    }
}

/// Un onglet démarré ; le lâcher arrête son agent et ses outils.
#[derive(Debug)]
pub(crate) struct TabEntry {
    pub link: TabLink,
    cancel: CancellationToken,
}

impl TabEntry {
    /// Démarre les relais et, dans `rt`, les outils puis l'agent ; les
    /// commandes attendent en file que l'agent soit là.
    pub fn start(
        session: TabSession,
        platform: &Platform,
        settings: Arc<AppSettings>,
        link: Link,
        rt: &Handle,
        out: mpsc::UnboundedSender<Out>,
    ) -> Self {
        let tab = session.tab().to_owned();
        let hub = SurfaceHub::new(session.clone(), platform.localizer.clone());
        let tools = Tools::new(session.clone(), hub.clone(), settings, platform);
        let cancel = CancellationToken::new();
        let (cmds, rx) = mpsc::unbounded_channel();
        let thread = Arc::new(Mutex::new(ThreadLog::default()));
        let closed = cancel.clone();
        rt.spawn(relay_mcp(tab.clone(), hub.subscribe(), Arc::downgrade(&hub), out.clone(), closed.clone()));
        let agent = Agent { session, hub: hub.clone(), link, localizer: platform.localizer.clone() };
        let relay = Relay { tab: tab.clone(), thread: thread.clone(), out, closed };
        rt.spawn(agent.run(tools, cancel.clone(), rx, relay));
        Self { link: TabLink { tab, hub, cmds, thread }, cancel }
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
                // risque, `surface` remplace à sa place et `close_surface` est idempotent.
                let Some(hub) = hub.upgrade() else { return };
                let list = hub.surfaces();
                emit(&out, "mcp", &tab, &McpEvent::Surfaces { list });
            }
            Err(RecvError::Closed) => return,
        }
    }
}

/// Le fil de l'onglet : gardé, puis relayé ; plus rien une fois l'onglet
/// fermé (`closed`), même si son agent n'a pas encore fini de s'arrêter.
struct Relay {
    tab: String,
    thread: Arc<Mutex<ThreadLog>>,
    out: mpsc::UnboundedSender<Out>,
    closed: CancellationToken,
}

impl Relay {
    fn emit(&self, e: &AgentEvent) {
        if self.closed.is_cancelled() {
            return;
        }
        let mut thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner);
        thread.push(e);
        emit(&self.out, "agent", &self.tab, e);
    }

    async fn run(self, mut rx: broadcast::Receiver<AgentEvent>) {
        loop {
            let received = tokio::select! {
                biased;
                () = self.closed.cancelled() => return,
                r = rx.recv() => r,
            };
            match received {
                Ok(e) => self.emit(&e),
                Err(RecvError::Lagged(_)) => {}
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
    localizer: Arc<oku_slides_i18n::Localizer>,
}

impl Agent {
    /// Démarre les outils puis l'agent, et lui relaie les commandes jusqu'à
    /// la fermeture de l'onglet (l'agent est alors lâché, donc arrêté).
    async fn run(
        self,
        tools: Tools,
        cancel: CancellationToken,
        mut rx: mpsc::UnboundedReceiver<Command>,
        relay: Relay,
    ) {
        let server = match ToolServer::start(tools, cancel.clone()).await {
            Ok(server) => server,
            Err(e) => {
                tracing::warn!(tab = %self.session.tab(), error = %e, "outils de l'onglet indisponibles");
                // Chaque consigne reçoit l'erreur dans le fil ; le reste est sans objet.
                let message = e.localize(&self.localizer);
                while let Some(cmd) = next(&cancel, &mut rx).await {
                    if let Command::Prompt { .. } = cmd {
                        relay.emit(&AgentEvent::Error { message: message.clone() });
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
        let agent = AgentHandle::spawn(self.session, self.hub, access, self.link, self.localizer);
        // Abonné avant la première commande : aucun évènement de l'agent n'est perdu.
        tokio::spawn(relay.run(agent.subscribe()));
        while let Some(cmd) = next(&cancel, &mut rx).await {
            match cmd {
                Command::Prompt { text, ui } => agent.prompt(text, ui),
                Command::Cancel => agent.cancel(),
                Command::NewConversation => agent.new_conversation(),
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
