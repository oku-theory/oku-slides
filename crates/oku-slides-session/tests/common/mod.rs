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
    req(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme_ref(theme)}]}))
}

pub fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"))
}

#[allow(dead_code, reason = "lu par certains binaires de test seulement")]
pub fn snapshot_json(s: &mut Session) -> serde_json::Value {
    let reply = s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("snapshot refusé : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("snapshot illisible : {e}"))
}

/// Le registre de base figé (`fixtures/widgets/base-registry.json`).
#[allow(dead_code, reason = "lu par certains binaires de test seulement")]
pub fn registry() -> std::sync::Arc<oku_slides_core::WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let probe = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    std::sync::Arc::new(oku_slides_core::WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}")))
}

pub fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// La référence de la version intégrée du thème nommé `name` (sans casse).
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn theme_ref(name: &str) -> Option<oku_slides_core::ThemeRef> {
    use oku_slides_session::{LibraryKind, Themes};
    Themes::builtins()
        .iter()
        .find(|b| b.manifest.name.eq_ignore_ascii_case(name))
        .map(|b| oku_slides_core::ThemeRef { id: b.manifest.id, version: b.version.clone() })
}

/// Le nom court (`papier`, `pro`) du thème intégré de même id que `r`.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn theme_name(r: &oku_slides_core::ThemeRef) -> Option<&'static str> {
    use oku_slides_session::{PAPIER_THEME, PRO_THEME};
    [(PAPIER_THEME, "papier"), (PRO_THEME, "pro")].into_iter().find(|(id, _)| *id == r.id).map(|(_, n)| n)
}
