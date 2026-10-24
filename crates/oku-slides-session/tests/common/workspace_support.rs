//! Aides des tests de non-régression qui passent par le `Workspace` : copier-coller
//! entre onglets et fenêtres (t-1178, t-1182).

use std::io::Cursor;
use std::sync::Mutex;

use oku_slides_core::Op;
use oku_slides_session::{Fonts, Platform, Session, Settings, Workspace, WorkspaceOutcome, WorkspaceRequest};

use crate::common::Data;

/// Le workspace repris de `data`, en français.
pub fn workspace(data: &Data) -> Mutex<Workspace> {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    let ws = Workspace::resume(Fonts::default(), &platform, data.join("resume"))
        .unwrap_or_else(|e| panic!("reprise impossible : {}", e.one_line()));
    Mutex::new(ws)
}

/// Envoie une requête de structure ; sa réponse JSON.
pub fn send(ws: &Mutex<Workspace>, v: serde_json::Value) -> serde_json::Value {
    let req: WorkspaceRequest = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"));
    match Workspace::apply(ws, req).unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line())) {
        WorkspaceOutcome::Applied(json) => serde_json::from_str(&json).unwrap_or_else(|e| panic!("réponse : {e}")),
        other => panic!("réponse inattendue : {other:?}"),
    }
}

/// `f` sur la session de l'onglet `tab`.
pub fn on<R>(ws: &Mutex<Workspace>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> R {
    Workspace::with_session(ws, tab, f).0.unwrap_or_else(|e| panic!("onglet {tab} : {}", e.one_line()))
}

/// Un PNG gris de 4 × 4 pixels, toujours le même.
pub fn png() -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, 4, 4);
    e.set_color(png::ColorType::Grayscale);
    let mut writer = e.write_header().unwrap_or_else(|e| panic!("png : {e}"));
    writer.write_image_data(&[7; 16]).unwrap_or_else(|e| panic!("png : {e}"));
    drop(writer);
    out.into_inner()
}

/// Une op lue de son JSON.
pub fn op(v: serde_json::Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}
