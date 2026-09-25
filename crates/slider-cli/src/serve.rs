//! `slider serve` : l'interface et l'API dans un navigateur, sans Tauri.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use color_eyre::eyre::{Result, eyre};
use slider_platform::Platform;
use slider_render::Fonts;
use slider_session::Localize;
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

/// Sert l'interface et l'API `/api` (une requête JSON par appel, comme `invoke` dans Tauri).
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
    let mut session = slider_session::Session::new(fonts, platform)?;
    if let Some(path) = &input {
        session.open(path)?;
    }
    let settings = &platform.settings;
    let (host, port) = (settings.serve.host.as_str(), port.unwrap_or(settings.serve.port));
    let server = Arc::new(Server::http((host, port)).map_err(|e| eyre!("{e}"))?);
    let stopper = Arc::clone(&server);
    ctrlc::set_handler(move || stopper.unblock())?;
    let stt = slider_stt::Stt::new(platform);
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
        if req.method() == &Method::Post && (url == "/api" || url == "/api/stt") {
            let mut body = String::new();
            if let Err(e) = req.as_reader().read_to_string(&mut body) {
                tracing::warn!(error = %e, "requête illisible");
            }
            let (status, reply) = if url == "/api" {
                match serde_json::from_str::<slider_session::Request>(&body) {
                    Ok(r) => match session.handle(r) {
                        Ok(json) => (200, json),
                        Err(e) => {
                            tracing::warn!(error = %e.one_line(), "requête refusée");
                            (400, serde_json::json!({ "error": e.localize(l) }).to_string())
                        }
                    },
                    Err(e) => (400, bad_request(e)),
                }
            } else {
                match serde_json::from_str::<slider_stt::SttRequest>(&body) {
                    Ok(r) => (200, serde_json::to_string(&stt.handle(r)).unwrap_or_default()),
                    Err(e) => (400, bad_request(e)),
                }
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
