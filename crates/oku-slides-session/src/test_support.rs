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
    // Sans reprise activée, la session n'écrit rien dans son dossier de
    // données : il peut disparaître une fois la session construite.
    let data = fresh_dir("oku-slides-session-test-support-");
    opened_on(data.path())
}

/// Une session isolée sur `data`, en français, ouverte sur `fixtures/roadmap.json`.
///
/// Pour les tests qui écrivent dans la bibliothèque : `data` reste à l'appelant.
pub(crate) fn opened_on(data: &std::path::Path) -> Session {
    let mut s = session_on(data);
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
        examples: Default::default(),
    }
}

/// Des fichiers de widgets, par nom.
pub(crate) fn widget_files(
    entries: &[(&str, &str)],
) -> std::collections::BTreeMap<oku_slides_core::WidgetFile, String> {
    entries
        .iter()
        .map(|(n, s)| {
            let name = oku_slides_core::WidgetFile::parse(n).unwrap_or_else(|| panic!("nom invalide : {n}"));
            (name, (*s).to_owned())
        })
        .collect()
}

/// Ajoute `manifest` (et ses fichiers de widgets) à `library` ; sa référence.
pub(crate) fn add_version(
    library: &crate::library::PackLibrary,
    manifest: &crate::pack::PackManifest,
    widgets: &[(&str, &str)],
) -> oku_slides_core::PackRef {
    let files = manifest.to_files(widget_files(widgets)).unwrap_or_else(|e| panic!("pack.json : {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("version : {}", e.one_line()));
    oku_slides_core::PackRef { id: manifest.id, version: files.version() }
}

/// Source du widget de pack `frise` des tests (`fixtures/widgets/frise-test.typ`) :
/// liste `etapes` (date, texte), quatre étapes par défaut, `max: 8`.
pub(crate) const FRISE_TEST: &str = include_str!("../../../fixtures/widgets/frise-test.typ");

/// Écrit le widget `frise` des tests dans le pack en cours de `s` (celui du deck,
/// ou celui des modèles en cours d'édition).
pub(crate) fn set_frise_widget(s: &mut Session) {
    s.set_widget("frise", FRISE_TEST.into()).unwrap_or_else(|e| panic!("frise : {}", e.one_line()));
}

/// Source du widget de pack `plan` des tests (`fixtures/widgets/plan-test.typ`) : étapes
/// (`titre`) à liste imbriquée `taches` (`texte`, `fait`) ; deux étapes par défaut.
pub(crate) const PLAN_TEST: &str = include_str!("../../../fixtures/widgets/plan-test.typ");

/// Écrit le widget `plan` des tests dans le pack en cours de `s`.
pub(crate) fn set_plan_widget(s: &mut Session) {
    s.set_widget("plan", PLAN_TEST.into()).unwrap_or_else(|e| panic!("plan : {}", e.one_line()));
}

/// Les réglages de test sur `data` : en français, decks dans `data/decks`.
pub(crate) fn settings_on(data: &std::path::Path) -> Settings {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings
}

/// Une session sur `settings`, sans deck ouvert.
pub(crate) fn session_with(settings: Settings) -> Session {
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {}", e.one_line()))
}

/// Une session isolée sur `data`, en français, sans deck ouvert.
pub(crate) fn session_on(data: &std::path::Path) -> Session {
    session_with(settings_on(data))
}

/// La bibliothèque de la session `s`.
pub(crate) fn library_of(s: &Session) -> &crate::library::PackLibrary {
    &s.library
}

/// Le fichier touché par un autre outil : même deck, autre empreinte.
///
/// La date `/ModDate` change, à longueur égale.
pub(crate) fn touch_elsewhere(path: &std::path::Path) {
    let mut bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
    let at = bytes.windows(8).position(|w| w == b"/ModDate").unwrap_or_else(|| panic!("sans /ModDate"));
    let date = at + bytes[at..].windows(2).position(|w| w == b"D:").unwrap_or_else(|| panic!("date illisible")) + 2;
    bytes[date..date + 4].copy_from_slice(b"2099");
    std::fs::write(path, bytes).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
}

/// La référence de la version intégrée du thème nommé `name` (sans casse).
pub(crate) fn theme_ref(name: &str) -> Option<oku_slides_core::ThemeRef> {
    use crate::library_kind::LibraryKind;
    crate::theme::Themes::builtins()
        .iter()
        .find(|b| b.manifest.name.eq_ignore_ascii_case(name))
        .map(|b| oku_slides_core::ThemeRef { id: b.manifest.id, version: b.version.clone() })
}

/// Le nom court (`papier`, `pro`) du thème intégré de même id que `r`.
pub(crate) fn theme_name(r: &oku_slides_core::ThemeRef) -> Option<&'static str> {
    [(crate::PAPIER_THEME, "papier"), (crate::PRO_THEME, "pro")].into_iter().find(|(id, _)| *id == r.id).map(|(_, n)| n)
}

/// Les réglages d'un espace de travail de test : données et decks sous `data`, interface en français.
pub(crate) fn workspace_platform(data: &std::path::Path) -> crate::Platform {
    let mut settings = crate::Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    crate::Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

/// Un espace de travail neuf sur `data`, sans reprise.
pub(crate) fn workspace_on(data: &std::path::Path) -> std::sync::Mutex<crate::Workspace> {
    let ws = crate::Workspace::new(crate::Fonts::default(), &workspace_platform(data), None);
    std::sync::Mutex::new(ws.unwrap_or_else(|e| panic!("espace de travail : {e}")))
}

/// L'espace de travail repris de `dir` (neuf s'il n'y a rien), sur `data`.
pub(crate) fn resumed_on(data: &std::path::Path, dir: std::path::PathBuf) -> std::sync::Mutex<crate::Workspace> {
    let ws = crate::Workspace::resume(crate::Fonts::default(), &workspace_platform(data), dir);
    std::sync::Mutex::new(ws.unwrap_or_else(|e| panic!("reprise : {e}")))
}

/// Applique une requête JSON de l'espace de travail.
pub(crate) fn apply_ws(
    ws: &std::sync::Mutex<crate::Workspace>,
    v: serde_json::Value,
) -> crate::Result<crate::WorkspaceOutcome> {
    crate::Workspace::apply(ws, serde_json::from_value(v).unwrap_or_else(|e| panic!("requête : {e}")))
}
