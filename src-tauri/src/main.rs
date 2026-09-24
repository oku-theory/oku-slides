//! Écho : application de bureau. Toute la logique est dans `slider-session` ;
//! l'interface appelle une seule commande, `request`, avec une requête JSON.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::Value;
use slider_session::{Request, Session};
use slider_stt::{Status, Stt, SttRequest};
use tauri::Manager;

struct AppState(Mutex<Session>);

#[tauri::command]
fn request(state: tauri::State<'_, AppState>, req: Request) -> Result<Value, String> {
    let mut session = state.0.lock().map_err(|_| "session indisponible".to_owned())?;
    session.handle(req).map_err(|e| e.to_string())
}

/// Dictée : jamais bloquant (téléchargement, micro et Whisper tournent dans leurs fils).
#[tauri::command]
fn stt(engine: tauri::State<'_, Stt>, req: SttRequest) -> Status {
    engine.handle(req)
}

/// Polices livrées avec l'app (ressource `fonts/`), plus `SLIDER_FONTS` si défini.
fn app_fonts(dirs: &[PathBuf]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if matches!(ext.as_str(), "ttf" | "otf" | "ttc")
                && let Ok(bytes) = std::fs::read(&path)
            {
                out.push(bytes);
            }
        }
    }
    out
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let mut dirs = Vec::new();
            if let Ok(res) = app.path().resource_dir() {
                dirs.push(res.join("fonts"));
            }
            if let Some(extra) = std::env::var_os("SLIDER_FONTS") {
                dirs.push(PathBuf::from(extra));
            }
            // En développement, les polices du dépôt.
            dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts")));
            let mut session = Session::new(app_fonts(&dirs));
            // Un fichier passé en argument (ouvrir avec…) est ouvert au démarrage.
            if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from)
                && path.exists()
            {
                session.open(&path).ok();
            }
            app.manage(AppState(Mutex::new(session)));
            let models = match std::env::var_os("ECHO_MODELS") {
                Some(dir) => PathBuf::from(dir),
                None => app.path().app_data_dir().map(|d| d.join("models")).unwrap_or_else(|_| slider_stt::default_models_dir()),
            };
            app.manage(Stt::new(models));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![request, stt])
        .run(tauri::generate_context!())
        .expect("échec du lancement d'Écho");
}
