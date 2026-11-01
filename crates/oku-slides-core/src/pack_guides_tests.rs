use std::collections::BTreeMap;

use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::{Namespace, PackKind, PackManifest, PackScope, ThemeManifest};

fn guides(edit: &str, use_: &str) -> Guides {
    Guides { edit: edit.to_owned(), use_: use_.to_owned() }
}

fn pack() -> PackManifest {
    PackManifest {
        id: Uuid::new_v4(),
        name: "Mien".into(),
        namespace: Namespace::parse("test").unwrap_or_else(|| panic!("namespace")),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::new(),
        styles: BTreeMap::new(),
        tokens: Vec::new(),
        order: Vec::new(),
        guides: Guides::default(),
    }
}

fn theme() -> ThemeManifest {
    ThemeManifest {
        id: Uuid::new_v4(),
        name: "Importé".into(),
        namespace: Namespace::parse("importe").unwrap_or_else(|| panic!("namespace")),
        extends: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: serde_json::Map::new(),
        guides: Guides::default(),
    }
}

#[test]
fn guides_empty_is_not_serialized() {
    let json = serde_json::to_value(guides("", "")).unwrap();
    assert_that!(json).is_equal_to(serde_json::json!({}));
}

#[rstest]
#[case::edit_only("e", "", serde_json::json!({"edit": "e"}))]
#[case::use_only("", "u", serde_json::json!({"use": "u"}))]
fn guides_serialize_skips_the_empty_one(#[case] edit: &str, #[case] use_: &str, #[case] expected: serde_json::Value) {
    assert_that!(serde_json::to_value(guides(edit, use_)).unwrap()).is_equal_to(expected);
}

#[test]
fn pack_manifest_without_guides_keeps_its_digest() {
    let files = pack().to_files(PackKind::Widgets, BTreeMap::new()).unwrap();
    assert_that!(files.manifest().contains_key("guides")).is_false();
    let again = PackManifest::from_files(&files).unwrap().to_files(PackKind::Widgets, BTreeMap::new()).unwrap();
    assert_that!(again.version()).is_equal_to(files.version());
}

#[test]
fn pack_manifest_guides_round_trip() {
    let mut manifest = pack();
    manifest.guides = guides("edit me", "use me");
    let files = manifest.to_files(PackKind::Widgets, BTreeMap::new()).unwrap();
    let read = PackManifest::from_files(&files).unwrap();
    assert_that!(read.guides).is_equal_to(guides("edit me", "use me"));
}

#[test]
fn theme_manifest_guides_round_trip() {
    let mut manifest = theme();
    manifest.guides = guides("", "use me");
    let read = ThemeManifest::from_files(&manifest.to_files().unwrap()).unwrap();
    assert_that!(read.guides).is_equal_to(guides("", "use me"));
}

#[rstest]
#[case::edit(GuideKind::Edit, "a")]
#[case::use_(GuideKind::Use, "b")]
fn guides_get_reads_the_one_set(#[case] which: GuideKind, #[case] expected: &str) {
    let mut g = Guides::default();
    g.set(GuideKind::Edit, "a".into());
    g.set(GuideKind::Use, "b".into());
    assert_that!(g.get(which)).is_equal_to(expected);
}

#[rstest]
#[case::edit(GuideKind::Edit)]
#[case::use_(GuideKind::Use)]
fn guide_kind_as_str_matches_its_serde_name(#[case] which: GuideKind) {
    let serde = serde_json::to_value(which).unwrap();
    assert_that!(serde.as_str()).is_equal_to(Some(which.as_str()));
}
