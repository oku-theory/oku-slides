//! `oku-slides serve` : l'interface et l'API dans un navigateur, sans Tauri.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use color_eyre::eyre::{Result, eyre};
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_session::Localize;
use tiny_http::{Header, Method, Response, Server};

use crate::out::Out;

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

/// Hôte de bouclage, port facultatif : `localhost`, `127.0.0.1`, `[::1]`.
fn loopback(host: &str) -> bool {
    let bare = match host.rsplit_once(':') {
        Some((name, port)) if !name.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    matches!(bare, "localhost" | "127.0.0.1" | "[::1]")
}

/// Taille maximale du corps d'une requête `/api*` : au-delà, 413.
const MAX_BODY: u64 = 64 * 1024 * 1024;

/// La requête vient d'une page locale (ce serveur, ou Vite sur un autre port).
///
/// Une page web qui viserait `localhost` est refusée : son `Origin` n'est
/// pas locale (CSRF), ou son `Host` n'est pas un nom de bouclage (DNS
/// rebinding). Serveur de développement seulement.
///
/// Une requête sans `Origin` est acceptée volontairement : un client qui
/// n'est pas un navigateur (tests, `curl`) ne peut pas servir de véhicule à
/// un CSRF, et un navigateur envoie toujours `Origin` sur un `POST`.
fn same_origin(req: &tiny_http::Request) -> bool {
    let header =
        |name: &'static str| req.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str().to_owned());
    header("Host").is_some_and(|h| loopback(&h))
        && header("Origin").is_none_or(|o| o.strip_prefix("http://").is_some_and(loopback))
}

/// `req` ferme le dernier onglet de la seule fenêtre : plus rien à afficher.
fn would_empty(
    ws: &std::sync::Mutex<oku_slides_session::Workspace>,
    req: &oku_slides_session::WorkspaceRequest,
) -> bool {
    let oku_slides_session::WorkspaceRequest::Close { tab, .. } = req else { return false };
    match oku_slides_session::Workspace::lock(ws).view().windows.as_slice() {
        [only] => matches!(only.tabs.as_slice(), [last] if last.id == *tab),
        _ => false,
    }
}

/// Sert l'interface et l'API, pour le développement seulement.
///
/// Onglets : `POST /api` (`{ tab, req }`) ; espace de travail : `POST
/// /api/workspace` ; dictée : `POST /api/stt`. Une requête d'une page non
/// locale est refusée (403).
///
/// Ctrl-C, SIGTERM ou SIGHUP arrêtent le serveur proprement : la requête en
/// cours reçoit sa réponse, puis `serve` rend la main (code de sortie 0).
pub fn serve(
    out: &mut Out<'_>,
    input: Option<PathBuf>,
    port: Option<u16>,
    ui: &Path,
    fonts: Fonts,
    platform: &Platform,
) -> Result<()> {
    let (l, storage) = (&platform.localizer, &platform.storage);
    // Le fichier donné s'ouvre dans le premier onglet, `t-1`.
    let workspace = std::sync::Mutex::new(oku_slides_session::Workspace::new(fonts, platform, input.as_deref())?);
    let settings = &platform.settings;
    let (host, port) = (settings.serve.host.as_str(), port.unwrap_or(settings.serve.port));
    let server = Arc::new(Server::http((host, port)).map_err(|e| eyre!("{e}"))?);
    let stopper = Arc::clone(&server);
    ctrlc::set_handler(move || stopper.unblock())?;
    let stt = oku_slides_stt::Stt::new(platform);
    out.line("cli-serving", &[("host", host.into()), ("port", port.into())])?;
    out.line("cli-models-dir", &[("dir", stt.models_dir().display().to_string().into())])?;
    let header = |value: &str| {
        Header::from_bytes("Content-Type", value)
            .map_err(|()| eyre!(l.tr_with("cli-bad-header", &[("value", value.into())])))
    };
    let json = header("application/json")?;
    let bad_request = |e: serde_json::Error| {
        serde_json::json!({ "error": l.tr_with("cli-bad-request", &[("reason", e.to_string().into())]) }).to_string()
    };
    for mut req in server.incoming_requests() {
        let url = req.url().split('?').next().unwrap_or("/").to_owned();
        if req.method() == &Method::Post && (url == "/api" || url == "/api/workspace" || url == "/api/stt") {
            if !same_origin(&req) {
                tracing::warn!(url, "requête d'une autre origine refusée");
                if let Err(e) = req.respond(Response::from_string("").with_status_code(403)) {
                    tracing::debug!(error = %e, "réponse non envoyée");
                }
                continue;
            }
            let mut body = String::new();
            if let Err(e) = req.as_reader().take(MAX_BODY + 1).read_to_string(&mut body) {
                tracing::warn!(error = %e, "requête illisible");
            }
            if body.len() as u64 > MAX_BODY {
                tracing::warn!(url, "corps de requête trop long");
                if let Err(e) = req.respond(Response::from_string("").with_status_code(413)) {
                    tracing::debug!(error = %e, "réponse non envoyée");
                }
                continue;
            }
            let refused = |e: oku_slides_session::SessionError| {
                tracing::warn!(error = %e.one_line(), "requête refusée");
                (400, serde_json::json!({ "error": e.localize(l) }).to_string())
            };
            let (status, reply) = match url.as_str() {
                "/api" => match serde_json::from_str::<TabRequest>(&body) {
                    Ok(TabRequest { tab, req }) => {
                        // Le serveur de développement ignore le drapeau : l'interface relit
                        // l'espace de travail après chaque commande.
                        let (result, _changed) = oku_slides_session::Workspace::request(&workspace, &tab, req);
                        result.map_or_else(refused, |json| (200, json))
                    }
                    Err(e) => (400, bad_request(e)),
                },
                "/api/workspace" => match serde_json::from_str::<oku_slides_session::WorkspaceRequest>(&body) {
                    // Fermer la seule fenêtre, ou son dernier onglet : ce serveur
                    // n'a pas de fenêtre native à quitter, il refuse (409) sans
                    // rien fermer. Les requêtes passent une à une : la
                    // vérification ne peut pas être devancée.
                    Ok(r) if would_empty(&workspace, &r) => {
                        (409, serde_json::json!({ "error": l.tr("session-last-window") }).to_string())
                    }
                    Ok(r) => match oku_slides_session::Workspace::apply(&workspace, r) {
                        Ok(oku_slides_session::WorkspaceOutcome::Applied(json)) => (200, json),
                        Ok(oku_slides_session::WorkspaceOutcome::LastWindow(_)) => {
                            (409, serde_json::json!({ "error": l.tr("session-last-window") }).to_string())
                        }
                        Err(e) => refused(e),
                    },
                    Err(e) => (400, bad_request(e)),
                },
                _ => match serde_json::from_str::<oku_slides_stt::SttRequest>(&body) {
                    Ok(r) => (200, serde_json::to_string(&stt.handle(r)).unwrap_or_default()),
                    Err(e) => (400, bad_request(e)),
                },
            };
            if let Err(e) = req.respond(Response::from_string(reply).with_status_code(status).with_header(json.clone()))
            {
                tracing::debug!(error = %e, "réponse non envoyée");
            }
            continue;
        }
        let rel = url.trim_start_matches('/');
        let rel = if rel.is_empty() { "index.html" } else { rel };
        let path = ui.join(rel);
        let safe = !rel.split('/').any(|seg| seg == "..");
        // Un chemin qui remonte (`..`) n'est même pas lu.
        let response = match safe.then(|| storage.read(&path).ok()).flatten() {
            Some(bytes) => req.respond(Response::from_data(bytes).with_header(header(content_type(&path))?)),
            None => req.respond(Response::from_string(l.tr("cli-not-found")).with_status_code(404)),
        };
        if let Err(e) = response {
            tracing::debug!(error = %e, "réponse non envoyée");
        }
    }
    out.line("cli-stopped", &[])?;
    Ok(())
}
