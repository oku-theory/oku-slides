//! Aides partagées par les tests d'intégration de `oku-slides-session`
//! (`session.rs`, `merge.rs`, `agent.rs`, `errors.rs`, `regressions/`).

use oku_slides_session::{Fonts, Platform, Request, Session, Settings};
use serde_json::json;

/// Un dossier de données neuf, supprimé à la fin du test (t-145).
///
/// Créé atomiquement sous un nom aléatoire (t-264) : jamais celui d'un
/// lancement précédent, même quand macOS recycle le PID.
#[derive(Debug)]
pub struct Data(tempfile::TempDir);

impl std::ops::Deref for Data {
    type Target = std::path::Path;
    fn deref(&self) -> &std::path::Path {
        self.0.path()
    }
}

impl AsRef<std::path::Path> for Data {
    fn as_ref(&self) -> &std::path::Path {
        self.0.path()
    }
}

pub fn data_dir() -> Data {
    let dir = tempfile::Builder::new()
        .prefix("oku-slides-session-")
        .tempdir()
        .unwrap_or_else(|e| panic!("dossier de données impossible : {e}"));
    Data(dir)
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

/// Une session isolée : son propre dossier de données, vide, qui vit autant
/// qu'elle (t-264).
pub struct Isolated {
    // Déclarée avant `data` : la session se ferme avant que son dossier soit supprimé.
    session: Session,
    data: Data,
}

impl Isolated {
    /// Le dossier de données de la session.
    #[allow(dead_code, reason = "lu par les tests de non-régression seulement")]
    pub fn data(&self) -> &std::path::Path {
        &self.data
    }
}

impl std::ops::Deref for Isolated {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.session
    }
}

impl std::ops::DerefMut for Isolated {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

/// Une session isolée : son propre dossier de données, vide.
pub fn session() -> Isolated {
    let data = data_dir();
    Isolated { session: session_in(&data), data }
}

#[allow(dead_code, reason = "lu par certains binaires de test seulement")]
pub fn set_theme(theme: &str) -> Request {
    req(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme}]}))
}

pub fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"))
}

#[allow(dead_code, reason = "lu par certains binaires de test seulement")]
pub fn snapshot_json(s: &mut Session) -> serde_json::Value {
    let reply = s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("snapshot refusé : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("snapshot illisible : {e}"))
}

/// Le registre de base figé (`fixtures/widgets/builtin-registry.json`).
#[allow(dead_code, reason = "lu par certains binaires de test seulement")]
pub fn registry() -> std::sync::Arc<oku_slides_core::WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let probe = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    std::sync::Arc::new(oku_slides_core::WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}")))
}

pub fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Un espace de noms des tests.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn ns(s: &str) -> oku_slides_core::Namespace {
    oku_slides_core::Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"))
}

/// Ajoute à la bibliothèque de thèmes sous `data` un thème `namespace` qui étend `extends`, de jetons `tokens`.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn add_user_theme(data: &std::path::Path, namespace: &str, extends: &[&str], tokens: serde_json::Value) {
    let storage = oku_slides_session::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let themes = oku_slides_session::ThemeLibrary::new(Some(data.join("themes")), storage);
    let manifest = oku_slides_session::ThemeManifest {
        id: uuid::Uuid::new_v4(),
        name: namespace.to_owned(),
        namespace: ns(namespace),
        extends: extends.iter().map(|e| ns(e)).collect(),
        description: String::new(),
        parent: None,
        created: "2026-10-22T10:00:00Z".to_owned(),
        tokens: serde_json::from_value(tokens).unwrap_or_default(),
    };
    let files = manifest.to_files().unwrap_or_else(|e| panic!("theme.json : {e}"));
    themes.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("thème : {}", e.one_line()));
}

/// Adds a user widget pack named `name` (also its namespace) with one widget to the widget library under `data`: its id.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn add_user_pack(data: &std::path::Path, name: &str) -> uuid::Uuid {
    const WIDGET: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;
    let storage = oku_slides_session::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = oku_slides_session::PackLibrary::new(Some(data.join("widgets")), None, no_user_themes(), storage);
    let manifest = oku_slides_session::PackManifest {
        id: uuid::Uuid::new_v4(),
        name: name.to_owned(),
        namespace: ns(name),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-22T10:00:00Z".to_owned(),
        scope: oku_slides_session::PackScope::Library,
        profiles: vec![],
        examples: std::collections::BTreeMap::default(),
        styles: std::collections::BTreeMap::default(),
        tokens: vec![],
    };
    let quote = oku_slides_core::WidgetFile::parse("quote").unwrap_or_else(|| panic!("nom de widget invalide"));
    let widgets = std::collections::BTreeMap::from([(quote, WIDGET.to_owned())]);
    let files = manifest.to_files(oku_slides_core::PackKind::Widgets, widgets).unwrap_or_else(|e| panic!("pack : {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("pack : {}", e.one_line()));
    manifest.id
}

/// Une bibliothèque de thèmes sans dossier : les seuls intégrés.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn no_user_themes() -> oku_slides_session::ThemeLibrary {
    let storage = oku_slides_session::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    oku_slides_session::ThemeLibrary::new(None, storage)
}

/// Le nom court (`papier`, `pro`, `neutre`) d'un thème intégré, depuis son espace de noms.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn theme_name(ns: &oku_slides_core::Namespace) -> Option<&'static str> {
    use oku_slides_session::{NEUTRE_NAMESPACE, PAPIER_NAMESPACE, PRO_NAMESPACE};
    [PAPIER_NAMESPACE, PRO_NAMESPACE, NEUTRE_NAMESPACE].into_iter().find(|n| *n == ns.as_str())
}
