//! Fabrique de session pour les tests, ouverte sur `fixtures/roadmap.json`.

use std::sync::Arc;

use oku_slides_core::WidgetRegistry;

use crate::{Fonts, Platform, Session, Settings};

/// Le registre de base figé (`fixtures/widgets/base-registry.json`).
pub(crate) fn registry() -> Arc<WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let probe = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}")))
}

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

/// Un `pack.json` sans modèles : pack `id`, nom `name`, parente `parent`, créé à `created`.
pub(crate) fn manifest(
    id: uuid::Uuid,
    name: &str,
    parent: Option<&oku_slides_core::PackVersion>,
    created: &str,
) -> crate::pack::PackManifest {
    crate::pack::PackManifest {
        id,
        name: name.into(),
        description: String::new(),
        parent: parent.cloned(),
        created: created.into(),
        templates: Vec::new(),
    }
}

/// Ajoute `manifest` (et `widgets`) à `library` ; sa référence.
pub(crate) fn add_version(
    library: &crate::library::Library,
    manifest: &crate::pack::PackManifest,
    widgets: Option<&str>,
) -> oku_slides_core::PackRef {
    let files = manifest.to_files(widgets.map(str::to_owned)).unwrap_or_else(|e| panic!("pack.json : {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("version : {}", e.one_line()));
    oku_slides_core::PackRef { id: manifest.id, version: files.version() }
}

/// Une session isolée sur `data`, en français, sans deck ouvert.
pub(crate) fn session_on(data: &std::path::Path) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {}", e.one_line()))
}

/// La bibliothèque de la session `s`.
pub(crate) fn library_of(s: &Session) -> &crate::library::Library {
    &s.library
}
