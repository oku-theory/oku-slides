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

/// Un élément `frise` (id `fr`) aux items par défaut `d0`…`d3`, ajouté aux blocs de `s-constat`.
pub(crate) fn add_frise() -> oku_slides_core::Op {
    serde_json::from_value(serde_json::json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "defaults": true,
        "element": { "id": "fr", "kind": "frise" }
    }))
    .unwrap()
}
