//! `oku-slides serve` : l'interface et l'API dans un navigateur, sans Tauri.

use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::pin::{Pin, pin};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, HttpBody};
use axum::extract::{Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use color_eyre::eyre::Result;
use oku_slides_agent::{AgentCommand, Stack};
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_session::{Localize, Workspace, WorkspaceOutcome, WorkspaceRequest};
use oku_slides_stt::Stt;

use crate::guard;
use crate::listen::{Limits, serve_connections};
use crate::out::Out;
use crate::sse::Clients;

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

/// Une requête d'onglet : le corps de `POST /api`.
#[derive(serde::Deserialize)]
struct TabRequest {
    tab: String,
    req: oku_slides_session::Request,
}

/// Taille maximale du corps d'une requête `/api*` : au-delà, 413.
const MAX_BODY: u64 = 64 * 1024 * 1024;

/// Délai laissé aux requêtes en cours à l'arrêt, puis au runtime pour finir
/// ses tâches : au-delà, elles sont abandonnées.
const GRACE: Duration = Duration::from_secs(5);

/// Les routes de l'API servies en POST.
const API: [&str; 4] = ["/api", "/api/workspace", "/api/stt", "/api/agent"];

/// `req` ferme le dernier onglet de la seule fenêtre : plus rien à afficher.
fn would_empty(ws: &Mutex<Workspace>, req: &WorkspaceRequest) -> bool {
    let WorkspaceRequest::Close { tab, .. } = req else { return false };
    match Workspace::lock(ws).view().windows.as_slice() {
        [only] => matches!(only.tabs.as_slice(), [last] if last.id == *tab),
        _ => false,
    }
}

/// 413 : corps de requête trop long.
fn too_large() -> Response {
    (StatusCode::PAYLOAD_TOO_LARGE, [(CONTENT_TYPE, "text/plain; charset=utf-8")], "").into_response()
}

/// Une réponse JSON.
fn json(status: StatusCode, body: String) -> Response {
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// Ce que partagent les requêtes du serveur.
struct Shared {
    /// L'espace de travail, partagé avec l'agent (`Sessions`).
    workspace: Arc<Mutex<Workspace>>,
    stack: Stack,
    stt: Stt,
    clients: Clients,
    platform: Platform,
    ui: PathBuf,
    /// L'adresse d'écoute (`[serve] host`), pour la garde d'origine.
    host: String,
    /// Une requête `/api*` à la fois, dans l'ordre d'arrivée (verrou FIFO,
    /// pris côté async : aucun fil bloquant n'attend) : la vérification de
    /// `would_empty` ne peut pas être devancée entre elle et `apply`.
    one_at_a_time: Arc<tokio::sync::Mutex<()>>,
}

impl Shared {
    /// Traite une requête `/api*` : statut et corps JSON. Bloquant.
    fn api(&self, url: &str, body: &[u8]) -> (StatusCode, String) {
        let l = &self.platform.localizer;
        let bad_request = |e: serde_json::Error| {
            let reason = e.to_string();
            let error = l.tr_with("cli-bad-request", &[("reason", reason.into())]);
            (StatusCode::BAD_REQUEST, serde_json::json!({ "error": error }).to_string())
        };
        let refused = |e: oku_slides_session::SessionError| {
            tracing::warn!(error = %e.one_line(), "requête refusée");
            (StatusCode::BAD_REQUEST, serde_json::json!({ "error": e.localize(l) }).to_string())
        };
        let last_window =
            || (StatusCode::CONFLICT, serde_json::json!({ "error": l.tr("session-last-window") }).to_string());
        match url {
            "/api" => match serde_json::from_slice::<TabRequest>(body) {
                Ok(TabRequest { tab, req }) => {
                    // Le serveur de développement ignore le drapeau : l'interface relit
                    // l'espace de travail après chaque commande.
                    let changes_deck = req.changes_deck();
                    let (result, _changed) = Workspace::request(&self.workspace, &tab, req);
                    match result {
                        Ok(json) => {
                            if changes_deck {
                                self.stack.after_change(&tab);
                            }
                            (StatusCode::OK, json)
                        }
                        Err(e) => refused(e),
                    }
                }
                Err(e) => bad_request(e),
            },
            "/api/workspace" => match serde_json::from_slice::<WorkspaceRequest>(body) {
                // Fermer la seule fenêtre, ou son dernier onglet : ce serveur
                // n'a pas de fenêtre native à quitter, il refuse (409) sans
                // rien fermer. Les requêtes passent une à une : la
                // vérification ne peut pas être devancée.
                Ok(r) if would_empty(&self.workspace, &r) => last_window(),
                Ok(r) => {
                    let (outcome, closed) = Workspace::closed_by(&self.workspace, |ws| Workspace::apply(ws, r));
                    // Onglets fermés (ou vierges remplacés) : leur agent s'arrête.
                    for gone in &closed {
                        self.stack.tab_closed(gone);
                    }
                    match outcome {
                        Ok(WorkspaceOutcome::Applied(json)) => (StatusCode::OK, json),
                        Ok(WorkspaceOutcome::LastWindow(_)) => last_window(),
                        Err(e) => refused(e),
                    }
                }
                Err(e) => bad_request(e),
            },
            "/api/agent" => match serde_json::from_slice::<AgentCommand>(body) {
                Ok(cmd) => match self.stack.handle(cmd) {
                    Ok(()) => (StatusCode::OK, "{}".to_owned()),
                    Err(e) => (StatusCode::BAD_REQUEST, serde_json::json!({ "error": e }).to_string()),
                },
                Err(e) => bad_request(e),
            },
            _ => match serde_json::from_slice::<oku_slides_stt::SttRequest>(body) {
                Ok(r) => (StatusCode::OK, serde_json::to_string(&self.stt.handle(r)).unwrap_or_default()),
                Err(e) => bad_request(e),
            },
        }
    }

    /// Un fichier de l'interface (`index.html` pour `/`). Bloquant.
    fn static_file(&self, url: &str) -> Response {
        let rel = url.trim_start_matches('/');
        let rel = if rel.is_empty() { "index.html" } else { rel };
        let path = self.ui.join(rel);
        // Un chemin qui remonte (`..`) ou sort du dossier n'est même pas lu.
        match stays_inside(rel).then(|| self.platform.storage.read(&path).ok()).flatten() {
            Some(bytes) => ([(CONTENT_TYPE, content_type(&path))], bytes).into_response(),
            None => (StatusCode::NOT_FOUND, self.platform.localizer.tr("cli-not-found")).into_response(),
        }
    }
}

/// Une écoute sur `ip` est joignable depuis le réseau : ni la boucle locale,
/// ni une adresse IPv6 qui la représente (`::ffff:127.0.0.1`). La garde
/// d'origine n'authentifie pas (ADR-0003) : un client du réseau qui forge
/// `Host` serait servi.
fn reachable_from_network(ip: IpAddr) -> bool {
    !ip.to_canonical().is_loopback()
}

/// `rel` (chemin d'URL non décodé, sans `/` de tête) reste sous le dossier de
/// l'interface : que des noms, ni `..`, ni `.` de tête, ni racine, ni préfixe
/// de lecteur ou de partage (Windows, où `\` sépare aussi).
fn stays_inside(rel: &str) -> bool {
    Path::new(rel).components().all(|c| matches!(c, Component::Normal(_)))
}

/// Lit le corps d'une requête ; `None` s'il dépasse `MAX_BODY`.
async fn read_body(mut body: Body) -> Result<Option<Vec<u8>>, axum::Error> {
    let mut bytes = Vec::new();
    while let Some(frame) = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await {
        if let Ok(data) = frame?.into_data() {
            bytes.extend_from_slice(&data);
            if bytes.len() as u64 > MAX_BODY {
                return Ok(None);
            }
        }
    }
    Ok(Some(bytes))
}

/// Toute requête passe ici : la garde d'origine d'abord, puis la route.
async fn handle(State(shared): State<Arc<Shared>>, req: Request) -> Response {
    let (parts, body) = req.into_parts();
    let l = &shared.platform.localizer;
    if let Err(reason) = guard::check(&parts.method, &parts.headers, &shared.host) {
        // `debug` : une page hostile pourrait sinon remplir le journal.
        tracing::debug!(method = %parts.method, ?reason, "requête d'une autre origine refusée");
        return json(StatusCode::FORBIDDEN, serde_json::json!({ "error": l.tr("cli-forbidden") }).to_string());
    }
    let url = parts.uri.path().to_owned();
    if parts.method == Method::GET && url == "/api/events" {
        return match shared.clients.subscribe() {
            Some(events) => {
                let headers = [(CONTENT_TYPE, "text/event-stream"), (CACHE_CONTROL, "no-cache")];
                (headers, Body::new(events)).into_response()
            }
            // `MAX_SSE_CLIENTS` sont déjà servis.
            None => {
                json(StatusCode::SERVICE_UNAVAILABLE, serde_json::json!({ "error": l.tr("cli-sse-full") }).to_string())
            }
        };
    }
    let worker = Arc::clone(&shared);
    let done = if parts.method == Method::POST && API.contains(&url.as_str()) {
        // `Content-Length` annonce déjà trop : refusé sans attendre le corps.
        if body.size_hint().lower() > MAX_BODY {
            tracing::warn!(url, "corps de requête trop long");
            return too_large();
        }
        let body = match read_body(body).await {
            Ok(Some(body)) => body,
            Ok(None) => {
                tracing::warn!(url, "corps de requête trop long");
                return too_large();
            }
            Err(e) => {
                // Client parti, ou corps mal encodé : rien à traiter.
                tracing::debug!(error = %e, "requête illisible");
                let error = l.tr_with("cli-bad-request", &[("reason", e.to_string().into())]);
                return json(StatusCode::BAD_REQUEST, serde_json::json!({ "error": error }).to_string());
            }
        };
        let one = Arc::clone(&shared.one_at_a_time).lock_owned().await;
        // Le traitement bloque (rendu, disque) : hors des fils du runtime.
        tokio::task::spawn_blocking(move || {
            let _one = one;
            let (status, reply) = worker.api(&url, &body);
            json(status, reply)
        })
        .await
    } else {
        tokio::task::spawn_blocking(move || worker.static_file(&url)).await
    };
    done.unwrap_or_else(|e| {
        tracing::error!(error = %e, "requête interrompue");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })
}

/// Sert l'interface et l'API, pour le développement seulement.
///
/// Onglets : `POST /api` (`{ tab, req }`) ; espace de travail : `POST
/// /api/workspace` ; dictée : `POST /api/stt` ; agent : `POST /api/agent`
/// (chaque commande nomme son onglet) et le flux `GET /api/events`. Une
/// requête qui ne vient pas de l'interface locale est refusée (403, voir
/// `guard`).
///
/// Chaque connexion a `[serve] header_timeout_ms` pour envoyer ses en-têtes ;
/// au-delà de `[serve] max_connections` connexions simultanées, une nouvelle
/// est fermée aussitôt (voir `listen`).
///
/// Ctrl-C, SIGTERM ou SIGHUP arrêtent le serveur proprement : les requêtes en
/// cours reçoivent leur réponse (`GRACE` au plus), les flux d'évènements se
/// terminent, la pile de l'agent s'arrête, puis `serve` rend la main (code de
/// sortie 0).
pub fn serve(
    out: &mut Out<'_>,
    input: Option<PathBuf>,
    port: Option<u16>,
    ui: &Path,
    fonts: Fonts,
    platform: &Platform,
) -> Result<()> {
    // Le fichier donné s'ouvre dans le premier onglet, `t-1`. L'espace de
    // travail est partagé par le serveur et l'agent (`Sessions`).
    let workspace = Arc::new(Mutex::new(Workspace::new(fonts, platform, input.as_deref())?));
    let settings = &platform.settings;
    let (host, port) = (settings.serve.host.as_str(), port.unwrap_or(settings.serve.port));
    // Le serveur, les outils MCP et l'agent : un seul runtime tokio.
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let listener = runtime.block_on(tokio::net::TcpListener::bind((host, port)))?;
    if reachable_from_network(listener.local_addr()?.ip()) {
        tracing::warn!("{}", platform.localizer.tr_with("cli-exposed", &[("host", host.into())]));
    }
    let stop = Arc::new(tokio::sync::Notify::new());
    {
        let stop = Arc::clone(&stop);
        ctrlc::set_handler(move || stop.notify_one())?;
    }
    let stt = Stt::new(platform);
    out.line("cli-serving", &[("host", host.into()), ("port", port.into())])?;
    out.line("cli-models-dir", &[("dir", stt.models_dir().display().to_string().into())])?;
    let clients = Clients::default();
    let sessions: Arc<dyn oku_slides_session::agent::Sessions> = workspace.clone();
    let stack = Stack::start(sessions, platform, Some(stt.clone()), runtime.handle().clone());
    {
        let c = clients.clone();
        stack.forward(move |name, json| c.send(name, &json));
    }
    let shared = Arc::new(Shared {
        workspace,
        stack,
        stt,
        clients: clients.clone(),
        platform: platform.clone(),
        ui: ui.to_owned(),
        host: host.to_owned(),
        one_at_a_time: Arc::new(tokio::sync::Mutex::new(())),
    });
    let app = Router::new().fallback(handle).with_state(Arc::clone(&shared));
    // Les flux d'évènements ne finissent jamais d'eux-mêmes : l'arrêt les clôt,
    // sinon l'arrêt gracieux les attendrait sans fin.
    let stopping = Arc::new(tokio::sync::Notify::new());
    let shutdown = {
        let stopping = Arc::clone(&stopping);
        async move {
            stop.notified().await;
            clients.close();
            stopping.notify_one();
        }
    };
    let limits = Limits::new(settings.serve.header_timeout(), settings.serve.max_connections.get());
    runtime.block_on(async {
        let mut server = pin!(serve_connections(listener, app, limits, shutdown));
        tokio::select! {
            () = &mut server => {}
            () = stopping.notified() => {
                // Une requête longue ne retient pas l'arrêt plus de `GRACE`.
                if tokio::time::timeout(GRACE, server).await.is_err() {
                    tracing::warn!("arrêt : requêtes en cours abandonnées");
                }
            }
        }
    });
    // La pile de l'agent s'arrête explicitement, pendant que le runtime qui la
    // porte tourne encore ; `block_on` est permis ici, hors contexte async.
    runtime.block_on(shared.stack.shutdown());
    drop(shared);
    runtime.shutdown_timeout(GRACE);
    out.line("cli-stopped", &[])?;
    Ok(())
}

#[cfg(test)]
#[path = "serve_tests.rs"]
mod serve_tests;
