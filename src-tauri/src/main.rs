//! Oku Slides : application de bureau. Toute la logique est dans `oku-slides-session` ;
//! l'interface appelle une seule commande, `request`, avec une requête JSON.
//!
//! Démarrage : journaux → plateforme (réglages, fichiers, langue) → session
//! et dictée.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use oku_slides_session::{Fonts, Localize, Localizer, Platform, Request, Session, Storage};
use oku_slides_stt::{Status, Stt, SttRequest};
use tauri::Manager;
use tauri::ipc::Response;

struct AppState {
    session: Mutex<Session>,
    localizer: Arc<Localizer>,
}

/// La réponse est déjà du JSON : elle passe telle quelle à l'interface ; une
/// erreur y arrive dans la langue de l'utilisateur.
#[tauri::command]
fn request(state: tauri::State<'_, AppState>, req: Request) -> Result<Response, String> {
    let mut session = state.session.lock().map_err(|_| state.localizer.tr("session-busy"))?;
    session.handle(req).map(Response::new).map_err(|e| {
        tracing::warn!(error = %e.one_line(), "requête refusée");
        e.localize(&state.localizer)
    })
}

/// Dictée : jamais bloquant (téléchargement, micro et Whisper tournent dans leurs fils).
#[tauri::command]
fn stt(engine: tauri::State<'_, Stt>, req: SttRequest) -> Status {
    engine.handle(req)
}

/// Polices livrées avec l'app (ressource `fonts/`), plus `paths.fonts` des
/// réglages ; un dossier illisible est signalé et ignoré.
fn app_fonts(storage: &Storage, dirs: &[PathBuf]) -> Fonts {
    Fonts::new(
        dirs.iter()
            .flat_map(|dir| {
                storage.read_fonts(dir).unwrap_or_else(|e| {
                    tracing::warn!(dir = %dir.display(), error = %oku_slides_platform::errors::one_line(&e), "polices");
                    Default::default()
                })
            })
            .map(|(_, bytes)| bytes),
    )
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    let platform = Platform::load()?;
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let mut dirs = Vec::new();
            if let Ok(res) = app.path().resource_dir() {
                dirs.push(res.join("fonts"));
            }
            dirs.extend(platform.settings.paths.fonts.clone());
            // En développement, les polices du dépôt.
            dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts")));
            let mut session = Session::new(app_fonts(&platform.storage, &dirs), &platform)?;
            // Un fichier passé en argument (ouvrir avec…) est ouvert au démarrage.
            if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from)
                && path.exists()
                && let Err(e) = session.open(&path)
            {
                tracing::warn!(path = %path.display(), error = %e.one_line(), "ouverture au démarrage");
            }
            app.manage(AppState { session: Mutex::new(session), localizer: platform.localizer.clone() });
            app.manage(Stt::new(&platform));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![request, stt])
        .run(tauri::generate_context!())?;
    Ok(())
}
