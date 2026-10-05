use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use oku_slides_core::WidgetFile;

use crate::test_support::{manifest, widget_files};

#[test]
fn pack_manifest_round_trips_through_pack_files() {
    let m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let files = m.to_files(widget_files(&[("kpi", "#let widget = none")])).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.widget(&WidgetFile::parse("kpi").unwrap())).is_equal_to(Some("#let widget = none"));
}

#[test]
fn pack_manifest_without_uuid_id_is_refused() {
    let json = serde_json::json!({ "id": "essentiel", "name": "E", "parent": null, "created": "x", "templates": [] });
    let files = oku_slides_core::PackFiles::new(json.as_object().cloned().unwrap(), std::collections::BTreeMap::new());
    assert_that!(PackManifest::from_files(&files).is_err()).is_true();
}

#[test]
fn pack_manifest_info_marks_the_default_pack() {
    let m = manifest(DEFAULT_PACK, "Essentiel", None, "2026-10-01T00:00:00Z");
    let v = m.to_files(std::collections::BTreeMap::new()).unwrap().version();
    let info = m.info(v.clone(), true);
    assert_that!((info.id, info.version, info.default, info.builtin)).is_equal_to((DEFAULT_PACK, v, true, true));
}

#[test]
fn now_created_is_utc_to_the_second() {
    let s = now_created();
    assert_that!(s.len()).is_equal_to(20);
    assert_that!(s.ends_with('Z') && s.as_bytes()[10] == b'T').is_true();
}

fn with_values(m: &mut PackManifest, widget: &str, values: serde_json::Value) {
    m.examples.insert(WidgetFile::parse(widget).unwrap(), values.as_object().cloned().unwrap());
}

#[test]
fn pack_manifest_examples_round_trip_through_pack_files() {
    let mut m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    with_values(&mut m, "kpi", serde_json::json!({ "value": "12" }));
    let files = m.to_files(widget_files(&[("kpi", "#let widget = none")])).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.manifest()["examples"]["kpi"]["value"].as_str()).is_equal_to(Some("12"));
}

#[test]
fn pack_manifest_examples_change_the_version() {
    let mut m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let before = m.to_files(std::collections::BTreeMap::new()).unwrap().version();
    with_values(&mut m, "kpi", serde_json::json!({ "value": "12" }));
    let after = m.to_files(std::collections::BTreeMap::new()).unwrap().version();
    assert_that!(after).is_not_equal_to(before);
}

fn read_examples(examples: serde_json::Value) -> PackManifest {
    let m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let mut json = serde_json::to_value(&m).unwrap();
    json["examples"] = examples;
    let files = oku_slides_core::PackFiles::new(json.as_object().cloned().unwrap(), std::collections::BTreeMap::new());
    PackManifest::from_files(&files).unwrap()
}

/// Un `pack.json` d'avant t-541 (une liste de cas nommés par widget) se lit : le
/// premier cas donne les valeurs du widget, les autres sont oubliés.
#[test]
fn pack_manifest_with_the_old_example_cases_keeps_the_first_ones_values() {
    let m = read_examples(serde_json::json!({
        "kpi": [{ "name": "Court", "values": { "value": "12" } }, { "name": "Long", "values": { "value": "1 234" } }]
    }));
    let kpi = m.examples.get(&WidgetFile::parse("kpi").unwrap()).cloned();
    assert_that!(kpi).is_equal_to(serde_json::json!({ "value": "12" }).as_object().cloned());
}

/// Un widget aux valeurs vides (ancien sans cas, premier cas sans valeurs, objet vide) n'a pas d'entrée.
#[test]
fn pack_manifest_with_empty_values_keeps_no_entry() {
    let m =
        read_examples(serde_json::json!({ "kpi": [], "quote": [{ "name": "Exemple", "values": {} }], "carte": {} }));
    assert_that!(m.examples.is_empty()).is_true();
}

/// Sans valeurs, `pack.json` n'a pas de clé `examples` : les versions existantes gardent leur empreinte.
#[test]
fn pack_manifest_without_examples_writes_no_examples_key() {
    let m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let files = m.to_files(std::collections::BTreeMap::new()).unwrap();
    assert_that!(files.manifest().contains_key("examples")).is_false();
}
