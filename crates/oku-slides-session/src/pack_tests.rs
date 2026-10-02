use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::test_support::manifest;

#[test]
fn pack_manifest_round_trips_through_pack_files() {
    let m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let files = m.to_files(Some("#let registry = ()".into())).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.widgets()).is_equal_to(Some("#let registry = ()"));
}

#[test]
fn pack_manifest_without_uuid_id_is_refused() {
    let json = serde_json::json!({ "id": "essentiel", "name": "E", "parent": null, "created": "x", "templates": [] });
    let files = oku_slides_core::PackFiles::new(json.as_object().cloned().unwrap(), None);
    assert_that!(PackManifest::from_files(&files).is_err()).is_true();
}

#[test]
fn pack_manifest_info_marks_the_default_pack() {
    let m = manifest(DEFAULT_PACK, "Essentiel", None, "2026-10-01T00:00:00Z");
    let v = m.to_files(None).unwrap().version();
    let info = m.info(v.clone(), true);
    assert_that!((info.id, info.version, info.default, info.builtin)).is_equal_to((DEFAULT_PACK, v, true, true));
}

#[test]
fn now_created_is_utc_to_the_second() {
    let s = now_created();
    assert_that!(s.len()).is_equal_to(20);
    assert_that!(s.ends_with('Z') && s.as_bytes()[10] == b'T').is_true();
}
