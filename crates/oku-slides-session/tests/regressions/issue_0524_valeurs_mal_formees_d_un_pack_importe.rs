//! t-524 : un `examples` mal formé dans le `pack.json` d'un pack reçu ne rend
//! plus tout le manifeste illisible, et les valeurs reçues obéissent aux bornes
//! de taille des textes.
//!
//! Avant : une entrée invalide (nom de widget hors de `[a-z][a-z0-9-]*`,
//! valeurs qui ne sont pas un objet) faisait échouer `PackManifest::from_files` ;
//! le pack n'entrait pas dans la bibliothèque, et `save_pack` (`.ok()`)
//! perdait en silence description et valeurs d'exemple. Une valeur reçue plus
//! longue que `PLAIN_MAX` passait telle quelle dans l'aperçu.

use std::collections::BTreeMap;

use oku_slides_core::{PLAIN_MAX, PackFiles, WidgetFile};
use oku_slides_session::{PackLibrary, PackManifest, PreviewFormat, Storage};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, session_in};

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

fn kpi() -> WidgetFile {
    WidgetFile::parse("kpi").unwrap_or_else(|| panic!("nom de widget"))
}

/// Un pack reçu : `pack.json` tel qu'écrit par un autre, avec le widget `kpi`.
fn received(id: Uuid, label: &str) -> PackFiles {
    let manifest = json!({
        "id": id, "name": "Charte", "namespace": "charte", "description": "Ma charte", "created": "2026-10-01T00:00:00Z", "templates": [],
        "examples": {
            "kpi": { "value": "12", "label": label },
            "Pas un nom": { "value": "1" },
            "carte": "pas des valeurs",
            "ancien": [{ "name": "Cas", "values": "pas un objet" }]
        }
    });
    let Value::Object(manifest) = manifest else { panic!("objet") };
    PackFiles::new(oku_slides_core::PackKind::Widgets, manifest, BTreeMap::from([(kpi(), KPI.to_owned())]))
}

/// Le même pack, sans entrée mal formée : seule la valeur `label` dépasse éventuellement.
fn well_formed(id: Uuid, label: &str) -> PackFiles {
    let mut manifest = received(id, label).manifest().clone();
    manifest.insert("examples".into(), json!({ "kpi": { "value": "12", "label": label } }));
    PackFiles::new(oku_slides_core::PackKind::Widgets, manifest, BTreeMap::from([(kpi(), KPI.to_owned())]))
}

#[test]
fn malformed_example_entries_are_dropped_and_the_valid_ones_kept() {
    let m = PackManifest::from_files(&received(Uuid::new_v4(), "Ventes")).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(m.description.as_str()).is_equal_to("Ma charte");
    assert_that!(m.examples.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![kpi()]);
    assert_that!(m.examples[&kpi()].get("value")).is_equal_to(Some(&json!("12")));
}

#[test]
fn examples_that_are_not_an_object_leave_the_manifest_readable() {
    let mut files = received(Uuid::new_v4(), "Ventes").manifest().clone();
    files.insert("examples".into(), json!("pas un objet"));
    let m = PackManifest::from_files(&PackFiles::new(oku_slides_core::PackKind::Widgets, files, BTreeMap::new()))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(m.examples.is_empty()).is_true();
}

#[test]
fn a_received_pack_with_malformed_examples_keeps_its_values_through_save_pack() {
    let data = data_dir();
    let id = Uuid::new_v4();
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    let library = PackLibrary::new(Some(data.join("widgets")), None, storage);
    library.add_version(id, &received(id, "Ventes")).unwrap_or_else(|e| panic!("{e}"));
    let mut s = session_in(&data);
    s.edit_pack(id).unwrap_or_else(|e| panic!("{e}"));
    s.save_pack(false).unwrap_or_else(|e| panic!("{e}"));
    let current = library.current_version(id).unwrap_or_else(|| panic!("version courante"));
    assert_that!(current.manifest.description.as_str()).is_equal_to("Ma charte");
    assert_that!(current.manifest.examples.get(&kpi()).and_then(|v| v.get("value"))).is_equal_to(Some(&json!("12")));
}

#[test]
fn a_received_value_over_the_text_bound_is_ignored_in_the_preview_and_dropped_at_the_next_write() {
    let data = data_dir();
    let id = Uuid::new_v4();
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    let library = PackLibrary::new(Some(data.join("widgets")), None, storage);
    let long = "x".repeat(PLAIN_MAX + 1);
    library.add_version(id, &well_formed(id, &long)).unwrap_or_else(|e| panic!("{e}"));
    let mut s = session_in(&data);
    s.edit_pack(id).unwrap_or_else(|e| panic!("{e}"));
    let p = s.preview_widget(None, "kpi", &Map::new(), PreviewFormat::Svg).unwrap_or_else(|e| panic!("{e}"));
    let params = &p.view.unwrap_or_else(|| panic!("vue de l'aperçu"))["slide"]["params"]["elements"][0]["params"];
    assert_that!(params["value"]).is_equal_to(json!("12"));
    assert_that!(params["label"] == json!(long)).is_false();
    let written = s.set_example_values("kpi", Map::from_iter([("value".into(), json!("13"))]));
    let written = written.unwrap_or_else(|e| panic!("{e}"));
    assert_that!(written.get("label")).is_none();
    assert_that!(written.get("value")).is_equal_to(Some(&json!("13")));
}
