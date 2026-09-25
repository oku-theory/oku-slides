//! Écho : application de bureau. Toute la logique est dans `slider-session` ;
//! l'interface appelle une seule commande, `request`, avec une requête JSON.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use slider_session::{Fonts, Request, Session, Settings, read_font_dir};
use slider_stt::{Status, Stt, SttRequest};
use tauri::Manager;
use tauri::ipc::Response;

struct AppState(Mutex<Session>);

/// La réponse est déjà du JSON : elle passe telle quelle à l'interface.
#[tauri::command]
fn request(state: tauri::State<'_, AppState>, req: Request) -> Result<Response, String> {
    let mut session = state.0.lock().map_err(|_| "session indisponible".to_owned())?;
    session.handle(req).map(Response::new).map_err(|e| e.one_line())
}

/// Dictée : jamais bloquant (téléchargement, micro et Whisper tournent dans leurs fils).
#[tauri::command]
fn stt(engine: tauri::State<'_, Stt>, req: SttRequest) -> Status {
    engine.handle(req)
}

/// Polices livrées avec l'app (ressource `fonts/`), plus `paths.fonts` des
/// réglages ; un dossier absent est ignoré.
fn app_fonts(dirs: &[PathBuf]) -> Fonts {
    Fonts::new(dirs.iter().flat_map(|dir| read_font_dir(dir).unwrap_or_default().into_values()))
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    slider_platform::telemetry::init();
    let settings = Settings::load()?;
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let mut dirs = Vec::new();
            if let Ok(res) = app.path().resource_dir() {
                dirs.push(res.join("fonts"));
            }
            dirs.extend(settings.paths.fonts.clone());
            // En développement, les polices du dépôt.
            dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts")));
            let mut session = Session::new(app_fonts(&dirs), &settings)?;
            // Un fichier passé en argument (ouvrir avec…) est ouvert au démarrage.
            if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from)
                && path.exists()
                && let Err(e) = session.open(&path)
            {
                tracing::warn!(path = %path.display(), error = %e, "ouverture au démarrage");
            }
            app.manage(AppState(Mutex::new(session)));
            app.manage(Stt::new(settings.paths.models(), settings.stt.clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![request, stt])
        .run(tauri::generate_context!())?;
    Ok(())
}
