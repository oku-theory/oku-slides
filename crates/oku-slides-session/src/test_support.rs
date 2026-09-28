//! Fabrique de session pour les tests, ouverte sur `fixtures/roadmap.json`.

use crate::{Fonts, Platform, Session, Settings};

/// Un dossier neuf, créé atomiquement sous un nom aléatoire (t-264).
///
/// Jamais celui d'un lancement précédent, même quand macOS recycle le PID
/// (nextest lance un processus par test). Supprimé au `drop`, sauf `keep()`.
pub(crate) fn fresh_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test impossible : {e}"))
}

/// Une session isolée, en français, ouverte sur `fixtures/roadmap.json`.
pub(crate) fn session() -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    // Sans reprise activée, la session n'écrit rien dans son dossier de
    // données : il peut disparaître une fois la session construite.
    let data = fresh_dir("oku-slides-session-test-support-");
    settings.paths.data = Some(data.path().to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    let mut s =
        Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()));
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap_or_else(|e| panic!("ouverture du fixture impossible : {}", e.one_line()));
    s
}
