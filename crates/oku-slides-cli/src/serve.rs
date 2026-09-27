//! `oku-slides serve` : l'interface et l'API dans un navigateur, sans Tauri.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use color_eyre::eyre::{Result, eyre};
use oku_slides_agent::{AgentCommand, Stack};
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_session::{Localize, Workspace, WorkspaceOutcome, WorkspaceRequest};
use tiny_http::{Header, Method, Response, Server};

use crate::guard;
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

/// `req` ferme le dernier onglet de la seule fenêtre : plus rien à afficher.
fn would_empty(ws: &Mutex<Workspace>, req: &WorkspaceRequest) -> bool {
    let WorkspaceRequest::Close { tab, .. } = req else { return false };
    match Workspace::lock(ws).view().windows.as_slice() {
        [only] => matches!(only.tabs.as_slice(), [last] if last.id == *tab),
        _ => false,
    }
}

/// Sert l'interface et l'API, pour le développement seulement.
///
/// Onglets : `POST /api` (`{ tab, req }`) ; espace de travail : `POST
/// /api/workspace` ; dictée : `POST /api/stt` ; agent : `POST /api/agent`
/// (chaque commande nomme son onglet) et le flux `GET /api/events`. Une
/// requête qui ne vient pas de l'interface locale est refusée (403, voir
/// `guard`).
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
    // Le fichier donné s'ouvre dans le premier onglet, `t-1`. L'espace de
    // travail est partagé par la boucle HTTP et l'agent (`Sessions`).
    let workspace = Arc::new(Mutex::new(Workspace::new(fonts, platform, input.as_deref())?));
    let settings = &platform.settings;
    let (host, port) = (settings.serve.host.as_str(), port.unwrap_or(settings.serve.port));
    let server = Arc::new(Server::http((host, port)).map_err(|e| eyre!("{e}"))?);
    let stopper = Arc::clone(&server);
    ctrlc::set_handler(move || stopper.unblock())?;
    let stt = oku_slides_stt::Stt::new(platform);
    out.line("cli-serving", &[("host", host.into()), ("port", port.into())])?;
    out.line("cli-models-dir", &[("dir", stt.models_dir().display().to_string().into())])?;
    // Outils MCP et agent : un runtime tokio à côté de la boucle HTTP.
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let clients = Clients::default();
    let sessions: Arc<dyn oku_slides_session::agent::Sessions> = workspace.clone();
    let stack = Stack::start(sessions, platform, Some(stt.clone()), runtime.handle().clone());
    {
        let c = clients.clone();
        stack.forward(move |name, json| c.send(name, &json));
    }
    let header = |value: &str| {
        Header::from_bytes("Content-Type", value)
            .map_err(|()| eyre!(l.tr_with("cli-bad-header", &[("value", value.into())])))
    };
    let json = header("application/json")?;
    let bad_request = |e: serde_json::Error| {
        serde_json::json!({ "error": l.tr_with("cli-bad-request", &[("reason", e.to_string().into())]) }).to_string()
    };
    for mut req in server.incoming_requests() {
        if let Err(reason) = guard::check(&req, host) {
            // `debug` : une page hostile pourrait sinon remplir le journal.
            tracing::debug!(method = %req.method(), ?reason, "requête d'une autre origine refusée");
            let body = serde_json::json!({ "error": l.tr("cli-forbidden") }).to_string();
            if let Err(e) = req.respond(Response::from_string(body).with_status_code(403).with_header(json.clone())) {
                tracing::debug!(error = %e, "réponse non envoyée");
            }
            continue;
        }
        let url = req.url().split('?').next().unwrap_or("/").to_owned();
        if req.method() == &Method::Get && url == "/api/events" {
            let Some(mut reader) = clients.subscribe() else {
                // `MAX_SSE_CLIENTS` sont déjà servis.
                let body = serde_json::json!({ "error": l.tr("cli-sse-full") }).to_string();
                if let Err(e) = req.respond(Response::from_string(body).with_status_code(503).with_header(json.clone()))
                {
                    tracing::debug!(error = %e, "réponse non envoyée");
                }
                continue;
            };
            let mut writer = req.into_writer();
            // Un flux sans fin : sa propre tâche, pour ne pas bloquer la boucle.
            // En HTTP « chunked » roulé à la main : `tiny_http` ne vide son
            // codeur qu'après 8 Ko écrits (ou la fin du flux), ce qui
            // retarderait chaque évènement de plusieurs tours ; ici chacun
            // est écrit et vidé (`flush`) aussitôt reçu.
            std::thread::spawn(move || {
                let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nTransfer-Encoding: chunked\r\n\r\n";
                if writer.write_all(head).and_then(|()| writer.flush()).is_err() {
                    return;
                }
                let mut buf = [0u8; 8192];
                loop {
                    let n = match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    let sent = write!(writer, "{n:x}\r\n")
                        .and_then(|()| writer.write_all(&buf[..n]))
                        .and_then(|()| writer.write_all(b"\r\n"))
                        .and_then(|()| writer.flush());
                    if sent.is_err() {
                        break;
                    }
                }
                let _ = writer.write_all(b"0\r\n\r\n");
                let _ = writer.flush();
            });
            continue;
        }
        let api = ["/api", "/api/workspace", "/api/stt", "/api/agent"].contains(&url.as_str());
        if req.method() == &Method::Post && api {
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
                        let (result, _changed) = Workspace::request(&workspace, &tab, req);
                        match result {
                            Ok(json) => {
                                stack.after_change(&tab);
                                (200, json)
                            }
                            Err(e) => refused(e),
                        }
                    }
                    Err(e) => (400, bad_request(e)),
                },
                "/api/workspace" => match serde_json::from_str::<WorkspaceRequest>(&body) {
                    // Fermer la seule fenêtre, ou son dernier onglet : ce serveur
                    // n'a pas de fenêtre native à quitter, il refuse (409) sans
                    // rien fermer. Les requêtes passent une à une : la
                    // vérification ne peut pas être devancée.
                    Ok(r) if would_empty(&workspace, &r) => {
                        (409, serde_json::json!({ "error": l.tr("session-last-window") }).to_string())
                    }
                    Ok(r) => {
                        let (outcome, closed) = Workspace::closed_by(&workspace, |ws| Workspace::apply(ws, r));
                        // Onglets fermés (ou vierges remplacés) : leur agent s'arrête.
                        for gone in &closed {
                            stack.tab_closed(gone);
                        }
                        match outcome {
                            Ok(WorkspaceOutcome::Applied(json)) => (200, json),
                            Ok(WorkspaceOutcome::LastWindow(_)) => {
                                (409, serde_json::json!({ "error": l.tr("session-last-window") }).to_string())
                            }
                            Err(e) => refused(e),
                        }
                    }
                    Err(e) => (400, bad_request(e)),
                },
                "/api/agent" => match serde_json::from_str::<AgentCommand>(&body) {
                    Ok(cmd) => match stack.handle(cmd) {
                        Ok(()) => (200, "{}".to_owned()),
                        Err(e) => (400, serde_json::json!({ "error": e }).to_string()),
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
    // La pile de l'agent s'arrête avant le runtime tokio qui la porte.
    drop(stack);
    drop(runtime);
    out.line("cli-stopped", &[])?;
    Ok(())
}
