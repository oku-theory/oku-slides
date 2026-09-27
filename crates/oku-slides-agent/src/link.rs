//! La connexion ACP partagée par tous les onglets : un seul sous-processus,
//! lancé à la première demande et relancé après un arrêt ; notifications et
//! demandes de permission routées vers l'onglet de leur `SessionId`.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex, PoisonError};

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    ClientCapabilities, FileSystemCapabilities, InitializeRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionId, SessionNotification,
};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use oku_slides_mcp::SurfaceHub;
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
}

/// Où livrer ce qui arrive pour une session : le fil et les surfaces de son onglet.
#[derive(Debug, Clone)]
pub(crate) struct Route {
    pub events: broadcast::Sender<AgentEvent>,
    pub hub: Arc<SurfaceHub>,
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
    launch: Launch,
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
                launch,
                state: Mutex::new(State::default()),
                routes: Routes::default(),
                shutdown: CancellationToken::new(),
            }),
        }
    }

    /// Dossier de travail d'une session sans document enregistré.
    pub(crate) fn fallback_cwd(&self) -> &Path {
        &self.inner.launch.fallback_cwd
    }

    /// La connexion courante, ou une nouvelle (sous-processus lancé et
    /// poignée de main faite) si la précédente est finie.
    pub(crate) async fn connect(&self) -> Result<Live, AgentError> {
        let mut state = self.inner.state.lock().await;
        if let Some(live) = &state.live
            && !live.closed.is_cancelled()
        {
            return Ok(live.clone());
        }
        state.live = None;
        state.generation += 1;
        let live = self.launch(state.generation).await?;
        state.live = Some(live.clone());
        Ok(live)
    }

    /// Livre à `route` ce qui arrive pour la session `id` du lancement `generation`.
    pub(crate) fn route(&self, generation: u64, id: SessionId, route: Route) {
        self.inner.routes.lock().unwrap_or_else(PoisonError::into_inner).insert((generation, id), route);
    }

    /// Oublie la session `id` du lancement `generation` : ce qui arrive encore
    /// pour elle est ignoré ; la même session d'un autre lancement reste routée.
    pub(crate) fn forget(&self, generation: u64, id: SessionId) {
        self.inner.routes.lock().unwrap_or_else(PoisonError::into_inner).remove(&(generation, id));
    }

    /// Lance l'agent et attend sa poignée de main, au plus `ready_within`.
    async fn launch(&self, generation: u64) -> Result<Live, AgentError> {
        let Launch { command, args, .. } = &self.inner.launch;
        let mut child = tokio::process::Command::new(command)
            .args(args)
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
        let serving = Serving {
            routes: self.inner.routes.clone(),
            shutdown: self.inner.shutdown.clone(),
            closed: CancellationToken::new(),
            generation,
        };
        let serving = tokio::spawn(serving.run(transport, child, ready_tx));
        match tokio::time::timeout(self.inner.launch.ready_within, ready_rx).await {
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
    shutdown: CancellationToken,
    closed: CancellationToken,
    generation: u64,
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
        let Self { routes, shutdown, closed, generation } = self;
        let result = Client
            .builder()
            .name(CLIENT_NAME)
            .on_receive_notification(
                {
                    let routes = routes.clone();
                    async move |n: SessionNotification, _cx: ConnectionTo<Agent>| {
                        tracing::trace!(?n, "acp");
                        let events = routes
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .get(&(generation, n.session_id.clone()))
                            .map(|r| r.events.clone());
                        match events {
                            Some(events) => {
                                if let Some(event) = from_update(n.update) {
                                    let _ = events.send(event);
                                }
                            }
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
                    match handshake(&cx).await {
                        Ok(true) => {}
                        Ok(false) => {
                            let _ = ready.send(Err(NoHttpSnafu.build().into()));
                            return Ok(());
                        }
                        Err(e) => {
                            let _ = ready.send(Err(StoppedSnafu { message: e.to_string() }.build().into()));
                            return Ok(());
                        }
                    }
                    let _ = ready.send(Ok(Live { cx: cx.clone(), generation, closed: closed.clone() }));
                    tokio::select! {
                        () = cx.incoming_closed() => {}
                        _ = child.wait() => {}
                        () = shutdown.cancelled() => {}
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

/// Poignée de main ; `false` si l'agent ne sait pas joindre les outils MCP en HTTP.
async fn handshake(cx: &ConnectionTo<Agent>) -> agent_client_protocol::Result<bool> {
    let caps = ClientCapabilities::new()
        .fs(FileSystemCapabilities::new().read_text_file(false).write_text_file(false))
        .terminal(false);
    let init =
        cx.send_request(InitializeRequest::new(ProtocolVersion::V1).client_capabilities(caps)).block_task().await?;
    Ok(init.agent_capabilities.mcp_capabilities.http)
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
