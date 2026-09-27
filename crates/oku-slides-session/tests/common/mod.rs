//! Aides partagées par les tests d'intégration de `oku-slides-session`
//! (`session.rs` et `merge.rs`).

use oku_slides_session::{Fonts, Platform, Request, Session, Settings};
use serde_json::json;

/// Un dossier de données neuf, supprimé à la fin du test (t-145).
pub struct Data(std::path::PathBuf);

impl std::ops::Deref for Data {
    type Target = std::path::Path;
    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl AsRef<std::path::Path> for Data {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub fn data_dir() -> Data {
    Data(std::env::temp_dir().join(format!("oku-slides-session-{}-{}", std::process::id(), unique())))
}

/// Une session sur le dossier de données `data`.
pub fn session_in(data: &std::path::Path) -> Session {
    let settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    session_with_settings(data, settings)
}

/// Une session sur `data`, avec ces réglages (`paths.data` et `i18n.locale`
/// sont forcés : dossier isolé, messages en français quelle que soit la
/// langue du système).
pub fn session_with_settings(data: &std::path::Path, mut settings: Settings) -> Session {
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()))
}

/// Une session isolée : son propre dossier de données, vide.
pub fn session() -> Session {
    session_in(&data_dir())
}

pub fn set_theme(theme: &str) -> Request {
    req(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme}]}))
}

pub fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"))
}

pub fn snapshot_json(s: &mut Session) -> serde_json::Value {
    let reply = s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("snapshot refusé : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("snapshot illisible : {e}"))
}

pub fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}
