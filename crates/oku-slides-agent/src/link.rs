//! La connexion ACP partagée par tous les onglets : un seul sous-processus,
//! lancé à la première demande et relancé après un arrêt ou un changement
//! d'agent ([`Link::switch`]) ; notifications et
//! demandes de permission routées vers l'onglet de leur `SessionId`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, PoisonError};
use std::time::Duration;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    ClientCapabilities, FileSystemCapabilities, InitializeRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionId, SessionNotification, SessionUpdate,
};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use oku_slides_mcp::SurfaceHub;
use sha2::{Digest, Sha256};
use snafu::IntoError;
use tokio::process::Child;
use tokio::sync::{Mutex, broadcast, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tokio_util::sync::CancellationToken;

use crate::error::{AgentError, NoHttpSnafu, NotFoundSnafu, SpawnSnafu, StoppedSnafu};
use crate::events::from_update;
use crate::permission::choice;
use crate::{AgentEvent, Launch};

/// Nom du client annoncé à l'agent.
const CLIENT_NAME: &str = "oku-slides";

/// La connexion vivante : à utiliser tant que `closed` n'est pas annulé.
#[derive(Debug, Clone)]
pub(crate) struct Live {
    pub cx: ConnectionTo<Agent>,
    /// Numéro du lancement : une session ACP ne vaut que pour le sien.
    pub generation: u64,
    /// Annulé quand la connexion est finie (arrêt de l'agent, pile lâchée).
    pub closed: CancellationToken,
    /// Ce que l'agent sait faire d'une session existante.
    pub caps: SessionCaps,
    /// Époque du lancement (voir [`Link::switch`]) : d'une autre, la connexion est périmée.
    pub epoch: u64,
    /// Arrête cette connexion (agent changé) ; enfant de l'arrêt de l'app.
    pub stop: CancellationToken,
}

/// Ce que l'agent sait faire d'une session existante.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SessionCaps {
    /// `session/load` (rejoue l'historique).
    pub load: bool,
    /// `session/resume` (sans rejeu).
    pub resume: bool,
}

/// Empreinte d'un lancement d'agent : ses arguments ne sont jamais écrits en clair.
pub fn agent_fingerprint(command: &str, args: &[String]) -> String {
    let mut h = Sha256::new();
    h.update(command.as_bytes());
    for a in args {
        h.update([0u8]);
        h.update(a.as_bytes());
    }
    hex::encode(h.finalize())
}

/// Où livrer ce qui arrive pour une session : le fil et les surfaces de son onglet.
#[derive(Debug, Clone)]
pub(crate) struct Route {
    pub events: broadcast::Sender<AgentEvent>,
    pub hub: Arc<SurfaceHub>,
    /// Session reprise par `session/load` dont le premier `session/prompt` n'est pas parti :
    /// ce qui rejoue l'historique est ignoré (voir [`is_replay`]).
    pub replaying: Arc<AtomicBool>,
}

/// Une mise à jour qui rejoue l'historique d'une session (`session/load`).
///
/// ACP envoie le rejeu avant la réponse à `session/load`, quand la route n'est pas encore
/// posée ; un agent non conforme qui l'envoie après la ferait entrer dans le tour (t-612).
/// Les autres mises à jour (commandes disponibles, mode…) décrivent l'état de la session.
fn is_replay(update: &SessionUpdate) -> bool {
    matches!(
        update,
        SessionUpdate::UserMessageChunk(_)
            | SessionUpdate::AgentMessageChunk(_)
            | SessionUpdate::AgentThoughtChunk(_)
            | SessionUpdate::ToolCall(_)
            | SessionUpdate::ToolCallUpdate(_)
            | SessionUpdate::Plan(_)
    )
}

/// Routes par lancement et session : un agent relancé peut réutiliser un
/// identifiant de session, qui ne doit pas atteindre l'onglet d'un autre lancement.
type Routes = Arc<StdMutex<HashMap<(u64, SessionId), Route>>>;

/// La connexion partagée, vue par les onglets ; lâcher le dernier clone
/// arrête l'agent.
#[derive(Debug, Clone)]
pub struct Link {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    /// Le lancement courant et son époque ; `switch` change les deux ensemble.
    launch: StdMutex<(Launch, u64)>,
    /// Le jeton `stop` de la connexion vivante ou en cours de lancement : `switch` l'annule.
    current_stop: StdMutex<Option<CancellationToken>>,
    state: Mutex<State>,
    routes: Routes,
    /// Arrête la connexion courante quand `Inner` est lâché.
    shutdown: CancellationToken,
}

#[derive(Debug, Default)]
struct State {
    live: Option<Live>,
    generation: u64,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

impl Link {
    /// Rien n'est lancé avant le premier [`Link::connect`].
    pub fn new(launch: Launch) -> Self {
        Self {
            inner: Arc::new(Inner {
                launch: StdMutex::new((launch, 0)),
                current_stop: StdMutex::default(),
                state: Mutex::new(State::default()),
                routes: Routes::default(),
                shutdown: CancellationToken::new(),
            }),
        }
    }

    /// L'empreinte de l'agent lancé : une session ne se reprend qu'avec le même.
    pub fn agent_id(&self) -> String {
        let (launch, _) = self.launch_now();
        agent_fingerprint(&launch.command, &launch.args)
    }

    /// Le lancement courant et son époque, copiés sous le verrou.
    fn launch_now(&self) -> (Launch, u64) {
        self.inner.launch.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Change d'agent : le prochain [`Link::connect`] lance `launch`.
    ///
    /// La connexion vivante est arrêtée tout de suite (ses tours échouent comme pour tout arrêt) ;
    /// un lancement en cours est interrompu, et `connect` relance aussitôt avec `launch`.
    /// Aucun lancement après `shutdown`.
    pub fn switch(&self, launch: Launch) {
        {
            let mut current = self.inner.launch.lock().unwrap_or_else(PoisonError::into_inner);
            current.0 = launch;
            current.1 += 1;
        }
        if let Some(stop) = self.inner.current_stop.lock().unwrap_or_else(PoisonError::into_inner).take() {
            stop.cancel();
        }
        tracing::info!("agent changé");
    }

    /// Le lancement vivant, s'il y en a un ; `None` aussi pendant un lancement (verrou pris).
    pub(crate) fn current_generation(&self) -> Option<u64> {
        let state = self.inner.state.try_lock().ok()?;
        state.live.as_ref().filter(|l| !l.closed.is_cancelled()).map(|l| l.generation)
    }

    /// Dossier de travail d'une session sans document enregistré.
    pub(crate) fn fallback_cwd(&self) -> PathBuf {
        self.launch_now().0.fallback_cwd
    }

    /// La connexion courante, ou une nouvelle (sous-processus lancé et
    /// poignée de main faite) si la précédente est finie ou d'un autre agent.
    pub(crate) async fn connect(&self) -> Result<Live, AgentError> {
        let mut state = self.inner.state.lock().await;
        loop {
            // Arrêtée pour de bon (`shutdown`) : plus aucun lancement.
            if self.inner.shutdown.is_cancelled() {
                return Err(StoppedSnafu { message: "shut down".to_owned() }.build().into());
            }
            let (launch, epoch) = self.launch_now();
            if let Some(live) = &state.live {
                if !live.closed.is_cancelled() && live.epoch == epoch {
                    return Ok(live.clone());
                }
                // D'une autre époque : l'agent a été changé, celui-ci s'arrête.
                live.stop.cancel();
            }
            state.live = None;
            state.generation += 1;
            // Posé avant la poignée de main : `switch` interrompt aussi un lancement en cours.
            let stop = self.inner.shutdown.child_token();
            *self.inner.current_stop.lock().unwrap_or_else(PoisonError::into_inner) = Some(stop.clone());
            // Un `switch` entre `launch_now` et la ligne ci-dessus a arrêté le jeton précédent, pas
            // celui-ci : l'époque relue le voit (`switch` la change avant de prendre le jeton).
            if self.launch_now().1 != epoch {
                stop.cancel();
                continue;
            }
            match self.launch(state.generation, launch, epoch, stop).await {
                // Changé pendant la poignée de main : on repasse par le haut de la boucle.
                Ok(live) => state.live = Some(live),
                // Lancement interrompu ou raté après une bascule : on tente le nouvel agent.
                Err(e) if self.launch_now().1 != epoch => {
                    tracing::debug!(error = %e, "lancement abandonné : agent changé");
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Arrête l'agent pour de bon (fin de l'app) : le sous-processus est tué,
    /// aucun autre ne sera lancé. Attend la fin de la connexion au plus
    /// `within` ; un lancement en cours n'est pas attendu (sa poignée de main
    /// s'interrompt, et son sous-processus avec).
    pub async fn shutdown(&self, within: Duration) {
        self.inner.shutdown.cancel();
        let live = self.inner.state.try_lock().ok().and_then(|s| s.live.clone());
        if let Some(live) = live
            && tokio::time::timeout(within, live.closed.cancelled()).await.is_err()
        {
            tracing::warn!(generation = live.generation, "agent pas arrêté à temps");
        }
    }

    /// Livre à `route` ce qui arrive pour la session `id` du lancement `generation`.
    pub(crate) fn route(&self, generation: u64, id: SessionId, route: Route) {
        self.inner.routes.lock().unwrap_or_else(PoisonError::into_inner).insert((generation, id), route);
    }

    /// Le premier `session/prompt` de la session `id` part : fin de son rejeu.
    pub(crate) fn prompting(&self, generation: u64, id: &SessionId) {
        if let Some(route) =
            self.inner.routes.lock().unwrap_or_else(PoisonError::into_inner).get(&(generation, id.clone()))
        {
            route.replaying.store(false, Ordering::Release);
        }
    }

    /// Oublie la session `id` du lancement `generation` : ce qui arrive encore
    /// pour elle est ignoré ; la même session d'un autre lancement reste routée.
    pub(crate) fn forget(&self, generation: u64, id: SessionId) {
        self.inner.routes.lock().unwrap_or_else(PoisonError::into_inner).remove(&(generation, id));
    }

    /// Lance l'agent `launch` (de l'époque `epoch`) et attend sa poignée de main, au plus
    /// `ready_within` ; `stop` l'interrompt et arrêtera la connexion.
    async fn launch(
        &self,
        generation: u64,
        launch: Launch,
        epoch: u64,
        stop: CancellationToken,
    ) -> Result<Live, AgentError> {
        let Launch { command, args, ready_within, .. } = launch;
        let mut child = tokio::process::Command::new(&command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|source| match source.kind() {
                std::io::ErrorKind::NotFound => NotFoundSnafu { command: command.clone() }.into_error(source),
                _ => SpawnSnafu { command: command.clone() }.into_error(source),
            })?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(StoppedSnafu { message: "no stdio".to_owned() }.build().into());
        };
        tracing::info!(command = %command, generation, "agent lancé");
        let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
        let (ready_tx, ready_rx) = oneshot::channel();
        let serving =
            Serving { routes: self.inner.routes.clone(), stop, closed: CancellationToken::new(), generation, epoch };
        let serving = tokio::spawn(serving.run(transport, child, ready_tx));
        match tokio::time::timeout(ready_within, ready_rx).await {
            Ok(Ok(ready)) => ready,
            Ok(Err(_)) => Err(StoppedSnafu { message: "connection ended before handshake".to_owned() }.build().into()),
            Err(_) => {
                // Lâcher la tâche lâche le sous-processus (`kill_on_drop`) et libère `connect`.
                serving.abort();
                tracing::warn!(command = %command, generation, "agent pas prêt à temps");
                Err(StoppedSnafu { message: "not ready in time".to_owned() }.build().into())
            }
        }
    }
}

/// Ce que la tâche de la connexion garde.
struct Serving {
    routes: Routes,
    /// Arrête la connexion : fin de l'app ou agent changé.
    stop: CancellationToken,
    closed: CancellationToken,
    generation: u64,
    epoch: u64,
}

impl Serving {
    /// Sert la connexion jusqu'à l'arrêt de l'agent ou de la pile ; rend la
    /// connexion prête (ou l'échec de la poignée de main) par `ready`.
    async fn run(
        self,
        transport: impl agent_client_protocol::ConnectTo<Client> + 'static,
        mut child: Child,
        ready: oneshot::Sender<Result<Live, AgentError>>,
    ) {
        let Self { routes, stop, closed, generation, epoch } = self;
        let result = Client
            .builder()
            .name(CLIENT_NAME)
            .on_receive_notification(
                {
                    let routes = routes.clone();
                    async move |n: SessionNotification, _cx: ConnectionTo<Agent>| {
                        tracing::trace!(?n, "acp");
                        let route = routes
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .get(&(generation, n.session_id.clone()))
                            .map(|r| (r.events.clone(), r.hub.clone(), r.replaying.load(Ordering::Acquire)));
                        match route {
                            Some((_, _, true)) if is_replay(&n.update) => {
                                tracing::debug!(session = ?n.session_id, "rejeu après session/load : ignoré");
                            }
                            // Rangée dans le tour ouvert : l'agent envoie les mises à jour d'un tour
                            // avant d'en rendre la réponse, sur la même connexion. Hors tour : ignorée.
                            Some((events, hub, _)) => match hub.turn() {
                                Some(turn) => {
                                    if let Some(event) = from_update(turn, n.update) {
                                        let _ = events.send(event);
                                    }
                                }
                                None => tracing::debug!(session = ?n.session_id, "mise à jour hors tour : ignorée"),
                            },
                            None => tracing::debug!(session = ?n.session_id, "notification d'une session inconnue"),
                        }
                        Ok(())
                    }
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                {
                    let routes = routes.clone();
                    async move |req: RequestPermissionRequest, responder, cx: ConnectionTo<Agent>| {
                        let hub = routes
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .get(&(generation, req.session_id.clone()))
                            .map(|r| r.hub.clone());
                        // Attendre l'utilisateur hors de la boucle de réception.
                        cx.spawn(async move {
                            let outcome = match hub {
                                Some(hub) => permission(&hub, &req).await,
                                None => {
                                    tracing::debug!(session = ?req.session_id, "permission d'une session inconnue");
                                    RequestPermissionOutcome::Cancelled
                                }
                            };
                            responder.respond(RequestPermissionResponse::new(outcome))
                        })
                    }
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(transport, {
                let closed = closed.clone();
                async move |cx: ConnectionTo<Agent>| {
                    let shaken = tokio::select! {
                        r = handshake(&cx) => r,
                        // Fin de l'app ou agent changé pendant la poignée de main :
                        // le sous-processus part avec la tâche.
                        () = stop.cancelled() => {
                            let _ = ready.send(Err(StoppedSnafu { message: "stopped".to_owned() }.build().into()));
                            return Ok(());
                        }
                    };
                    let caps = match shaken {
                        Ok(Some(caps)) => caps,
                        Ok(None) => {
                            let _ = ready.send(Err(NoHttpSnafu.build().into()));
                            return Ok(());
                        }
                        Err(e) => {
                            let _ = ready.send(Err(StoppedSnafu { message: e.to_string() }.build().into()));
                            return Ok(());
                        }
                    };
                    let _ = ready.send(Ok(Live {
                        cx: cx.clone(),
                        generation,
                        closed: closed.clone(),
                        caps,
                        epoch,
                        stop: stop.clone(),
                    }));
                    tokio::select! {
                        () = cx.incoming_closed() => {}
                        _ = child.wait() => {}
                        // Fin de l'app ou agent changé. Tué tout de suite, avant `closed` :
                        // `Link::shutdown` n'attend que lui.
                        () = stop.cancelled() => {
                            if let Err(e) = child.start_kill() {
                                tracing::debug!(generation, error = %e, "agent déjà arrêté");
                            }
                        }
                    }
                    // Avant la fin du pilote : les requêtes encore en cours
                    // échoueront avec `closed` déjà annulé (arrêt, pas refus).
                    closed.cancel();
                    Ok(())
                }
            })
            .await;
        match result {
            Ok(()) => tracing::info!(generation, "agent arrêté"),
            Err(e) => tracing::warn!(generation, error = %e, "agent arrêté"),
        }
        closed.cancel();
        routes.lock().unwrap_or_else(PoisonError::into_inner).retain(|(g, _), _| *g != generation);
    }
}

/// Poignée de main ; `None` si l'agent ne sait pas joindre les outils MCP en HTTP.
async fn handshake(cx: &ConnectionTo<Agent>) -> agent_client_protocol::Result<Option<SessionCaps>> {
    let caps = ClientCapabilities::new()
        .fs(FileSystemCapabilities::new().read_text_file(false).write_text_file(false))
        .terminal(false);
    let init =
        cx.send_request(InitializeRequest::new(ProtocolVersion::V1).client_capabilities(caps)).block_task().await?;
    let caps = &init.agent_capabilities;
    Ok(caps
        .mcp_capabilities
        .http
        .then(|| SessionCaps { load: caps.load_session, resume: caps.session_capabilities.resume.is_some() }))
}

/// La réponse de l'utilisateur, demandée dans les surfaces de l'onglet.
async fn permission(hub: &SurfaceHub, req: &RequestPermissionRequest) -> RequestPermissionOutcome {
    let title = req.tool_call.fields.title.clone().unwrap_or_default();
    let choices: Vec<_> = req.options.iter().map(choice).collect();
    match hub.permission(&title, &choices).await.and_then(|i| req.options.get(i)) {
        Some(o) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(o.option_id.clone())),
        None => RequestPermissionOutcome::Cancelled,
    }
}

#[cfg(test)]
#[path = "link_tests.rs"]
mod link_tests;
