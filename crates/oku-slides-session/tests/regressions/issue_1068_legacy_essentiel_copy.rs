//! t-1068 : une copie d'Essentiel enregistrée aux lots 1-2 (pack de modèles `6f3c…`, espace de
//! noms `essentiel`, rangé sous `<data>/widgets/`) bloquait toute écriture sur l'Essentiel intégré
//! (`NamespaceTaken`) et le masquait dans les recherches par nom et par espace de noms.

use std::collections::BTreeMap;
use std::path::Path;

use oku_slides_core::{Namespace, PackFiles, PackKind, PackScope};
use oku_slides_session::{ESSENTIEL_PACK, PackLibrary, PackManifest, Storage, library_pack};
use serde_json::{Map, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, session_in};

/// L'id de l'ancien Essentiel, pack de modèles des lots 1-2.
const LEGACY_ID: Uuid = Uuid::from_u128(0x6f3c1e2a_8b4d_4c1e_9a57_2d0e5b7c9f10);

/// Les fichiers de l'ancienne copie d'Essentiel : un pack de widgets sans widgets.
fn legacy_files() -> PackFiles {
    let m = PackManifest {
        id: LEGACY_ID,
        name: "Essentiel".into(),
        namespace: Namespace::parse("essentiel").unwrap_or_else(|| panic!("espace de noms")),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: Default::default(),
        tokens: Vec::new(),
    };
    m.to_files(PackKind::Widgets, BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Range la copie dans `<data>/widgets/`, comme l'écrivait le mode modèles des lots 1-2.
fn store_legacy_copy(data: &Path) {
    let files = legacy_files();
    let dir = data.join("widgets").join(LEGACY_ID.to_string());
    let at = dir.join("versions").join(files.version().hex());
    std::fs::create_dir_all(&at).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(at.join("widgets.json"), files.pack_json().unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(dir.join("current"), files.version().as_str()).unwrap_or_else(|e| panic!("{e}"));
}

fn library(data: &Path) -> PackLibrary {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    PackLibrary::new(Some(data.join("widgets")), Some(data.join("layouts")), crate::common::no_user_themes(), storage)
}

#[test]
fn issue_1068_legacy_copy_lets_pack_mode_edit_an_essentiel_example() {
    let data = data_dir();
    store_legacy_copy(&data);
    let mut s = session_in(&data);
    s.edit_pack(ESSENTIEL_PACK).unwrap_or_else(|e| panic!("{e}"));
    let edited = s.set_example_values("content", Map::from_iter([("title".into(), json!("Neuf"))]));
    assert_that!(edited.map(|v| v.get("title").cloned())).is_ok().is_equal_to(Some(json!("Neuf")));
}

#[test]
fn issue_1068_legacy_copy_lets_save_pack_save_essentiel() {
    let data = data_dir();
    store_legacy_copy(&data);
    let mut s = session_in(&data);
    s.edit_pack(ESSENTIEL_PACK).unwrap_or_else(|e| panic!("{e}"));
    s.set_example_values("content", Map::from_iter([("title".into(), json!("Neuf"))]))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(s.save_pack(false)).is_ok();
}

#[rstest::rstest]
#[case::by_name("Essentiel")]
#[case::by_namespace("essentiel")]
fn issue_1068_legacy_copy_lookup_resolves_the_builtin_layout_pack(#[case] wanted: &str) {
    let data = data_dir();
    store_legacy_copy(&data);
    let files = library_pack(&library(&data), wanted).unwrap_or_else(|e| panic!("{e}"));
    let id = PackManifest::from_files(&files).map(|m| m.id).ok();
    assert_that!((files.kind(), id)).is_equal_to((PackKind::Layouts, Some(ESSENTIEL_PACK)));
}

#[test]
fn issue_1068_legacy_copy_stays_readable_by_id() {
    let data = data_dir();
    store_legacy_copy(&data);
    let current = library(&data).current_version(LEGACY_ID).map(|s| s.manifest.name);
    assert_that!(current).is_equal_to(Some("Essentiel".to_owned()));
}

#[test]
fn issue_1068_legacy_copy_gets_no_new_version() {
    let data = data_dir();
    store_legacy_copy(&data);
    let mut m = PackManifest::from_files(&legacy_files()).unwrap_or_else(|e| panic!("{e}"));
    m.description = "Changée".into();
    let files = m.to_files(PackKind::Widgets, BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(library(&data).add_version(LEGACY_ID, &files)).is_err();
}
