//! Sessions de test sur un dossier de données neuf, partagées par
//! `lib_tests.rs` et `fields_tests.rs`.

use std::path::{Path, PathBuf};

use crate::{Fonts, Platform, Session, Settings};

/// Un dossier neuf, supprimé à la fin du test.
pub(crate) struct Data(pub(crate) PathBuf);

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

pub(crate) fn data_dir() -> Data {
    Data(crate::test_support::fresh_dir("oku-slides-lib-").keep())
}

pub(crate) fn session_in(data: &Path) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    Session::new(Fonts::default(), &Platform::with_settings(settings).unwrap()).unwrap()
}

pub(crate) fn fixture_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

pub(crate) fn session_with_fixture() -> Session {
    let mut s = session_in(&data_dir().0);
    s.open(&fixture_path()).unwrap();
    s
}

/// Un élément `deck/frise` (widget du pack du deck des tests, id `fr`) aux items par défaut
/// `d0`…`d3`, ajouté aux blocs de `s-constat`.
fn add_frise() -> oku_slides_core::Op {
    serde_json::from_value(serde_json::json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "defaults": true,
        "element": { "id": "fr", "kind": "deck/frise" }
    }))
    .unwrap()
}

/// Le fixture, avec le widget de pack `frise` des tests et un élément `fr` sur `s-constat`.
pub(crate) fn session_with_frise() -> Session {
    let mut s = session_with_fixture();
    crate::test_support::set_frise_widget(&mut s);
    add_frise_to(&mut s);
    s
}

/// Ajoute l'élément `fr` (widget `frise`) sur `s-constat`.
pub(crate) fn add_frise_to(s: &mut Session) {
    s.apply("frise", vec![add_frise()], None).unwrap();
}

/// Le fixture, avec le widget de pack `plan` des tests et un élément `pl` sur `s-constat`,
/// amorcé de son défaut : étapes `d0` (tâches `d0`, `d1`) et `d1` (tâche `d0`).
pub(crate) fn session_with_plan() -> Session {
    let mut s = session_with_fixture();
    crate::test_support::set_plan_widget(&mut s);
    let add = serde_json::from_value(serde_json::json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "defaults": true,
        "element": { "id": "pl", "kind": "deck/plan" }
    }))
    .unwrap();
    s.apply("plan", vec![add], None).unwrap();
    s
}
