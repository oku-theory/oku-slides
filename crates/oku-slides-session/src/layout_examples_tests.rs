use oku_slides_core::{Op, WidgetFile};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::Session;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, session_on};

/// La valeur d'exemple `param` du layout `name` que montre `s` (mode pack).
fn example(s: &Session, name: &str, param: &str) -> Option<Value> {
    s.example_values(None, &WidgetFile::parse(name).unwrap()).unwrap().get(param).cloned()
}

/// L'op du panneau ou de la scène qui règle `param` de la slide `slide` à `value`.
fn set_param(slide: &str, param: &str, value: Value) -> Op {
    let mut op = json!({ "op": "set_param", "target": { "slide": slide }, "param": param });
    op["value"] = value;
    serde_json::from_value(op).unwrap()
}

/// Vide le titre de la slide du layout `content` par une op (la scène, la carte de la slide).
fn empty_content_title(s: &mut Session) {
    s.apply("Titre", vec![set_param("layout-content", "title", json!(""))], None).unwrap();
}

/// Le titre d'exemple d'Essentiel pour `content`, avant toute édition.
fn initial_title() -> Option<Value> {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    example(&s, "content", "title")
}

#[test]
fn apply_emptying_a_layout_text_writes_an_empty_example() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    empty_content_title(&mut s);
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn set_example_values_emptying_a_layout_text_keeps_it_empty() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    s.set_example_values("content", Map::from_iter([("title".into(), json!(""))])).unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn undo_after_emptying_a_layout_text_brings_the_text_back() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    empty_content_title(&mut s);
    s.undo().unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(initial_title());
}

#[test]
fn redo_after_undoing_an_emptied_layout_text_empties_it_again() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    empty_content_title(&mut s);
    s.undo().unwrap();
    s.redo().unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn apply_on_a_layout_slide_reseeds_the_slide_from_its_example() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    s.apply("Titre", vec![set_param("layout-content", "title", json!("Neuf"))], None).unwrap();
    let slide = s.deck().slide("layout-content").cloned().unwrap();
    assert_that!((slide.params.text("title"), example(&s, "content", "title")))
        .is_equal_to((Some("Neuf"), Some(json!("Neuf"))));
}

#[test]
fn resume_after_emptying_a_layout_text_keeps_it_empty() {
    let data = fresh_dir("oku-slides-layout-examples-");
    let dir = fresh_dir("oku-slides-layout-examples-resume-");
    let mut opener = session_on(data.path());
    let mut s = crate::test_support_packs::pack_tab_from(&mut opener, data.path(), ESSENTIEL_PACK);
    assert_that!(s.resume(dir.path().to_path_buf())).is_true();
    empty_content_title(&mut s);
    s.flush_resume();
    drop(s);
    let mut back = session_on(data.path());
    assert_that!(back.resume(dir.path().to_path_buf())).is_true();
    assert_that!(example(&back, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn save_and_reopen_after_emptying_a_layout_text_keeps_it_empty() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    empty_content_title(&mut s);
    s.save_pack(false).unwrap();
    drop(s);
    let s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn set_grid_on_a_layout_slide_writes_the_example_grid() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    let op = Op::SetGrid { slide: "layout-blank".into(), columns: Some(4), rows: None };
    s.apply("Grille", vec![op], None).unwrap();
    assert_that!(example(&s, "blank", "grid")).is_equal_to(Some(json!({ "columns": 4, "rows": 6 })));
}

#[test]
fn undo_after_set_grid_on_a_layout_slide_restores_the_example() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    let before = example(&s, "blank", "grid");
    let op = Op::SetGrid { slide: "layout-blank".into(), columns: Some(4), rows: None };
    s.apply("Grille", vec![op], None).unwrap();
    s.undo().unwrap();
    assert_that!(example(&s, "blank", "grid")).is_equal_to(before);
}

/// L'op qui retire l'élément `element` de la slide `slide`.
fn remove_element(slide: &str, element: &str) -> Op {
    serde_json::from_value(json!({ "op": "remove_element", "slide": slide, "element": element })).unwrap()
}

#[test]
fn remove_element_of_the_last_block_of_a_layout_keeps_an_empty_slot() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    s.apply("Retirer", vec![remove_element("layout-content", "b")], None).unwrap();
    assert_that!(example(&s, "content", "blocks")).is_equal_to(Some(json!([])));
}

/// Un layout à liste sans minimum, d'un item par défaut.
const LIST_LAYOUT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("liste", category: "slide", params: (items: p.list(max: 3, item: (t: p.plain()),
  default: ((t: "un"),))), (args, ctx) => [#args.items.len()])
"#;

/// L'op qui change la liste `items` de la slide du layout `liste`.
fn edit_items(change: Value) -> Op {
    let mut op = json!({ "op": "edit_list", "target": { "slide": "layout-liste" }, "param": "items" });
    op["change"] = change;
    serde_json::from_value(op).unwrap()
}

#[test]
fn edit_list_removing_the_last_item_of_a_layout_list_keeps_it_empty() {
    let data = fresh_dir("oku-slides-layout-examples-");
    let (mut s, _) = crate::test_support::layout_pack_tab(data.path(), "acme", &["one"]);
    s.set_widget(None, "liste", LIST_LAYOUT.into()).unwrap();
    s.apply("Ajouter", vec![edit_items(json!({ "op": "insert", "id": "x", "values": { "t": "deux" } }))], None)
        .unwrap();
    s.apply("Retirer", vec![edit_items(json!({ "op": "remove", "item": "x" }))], None).unwrap();
    assert_that!(example(&s, "liste", "items")).is_equal_to(Some(json!([])));
}

#[test]
fn apply_setting_a_layout_text_then_emptying_it_in_one_batch_keeps_it_empty() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    let ops =
        vec![set_param("layout-content", "title", json!("Neuf")), set_param("layout-content", "title", json!(""))];
    s.apply("Titre", ops, None).unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

/// The data of the edited pack's manifest (pack mode).
fn manifest_data(s: &Session) -> Vec<(String, String)> {
    let t = s.widget_target(None).unwrap();
    let m = crate::pack::PackManifest::from_files(&t.files).unwrap();
    m.data.iter().map(|(id, d)| (id.as_str().to_owned(), d.name.clone())).collect()
}

#[test]
fn layout_example_chart_writes_its_data_to_the_manifest() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    let op = json!({"op": "add_element", "slot": {"owner": {"slide": "layout-content"}, "param": "blocks"},
        "element": {"id": "c1", "kind": "base/chart",
            "params": {"option": {"data": "base-example-chart", "xAxis": {"type": "category"}, "series": [{"type": "bar"}]}}}});
    s.apply("Graphique", vec![serde_json::from_value(op).unwrap()], None)
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    let written = manifest_data(&s);
    assert_that!(written.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["chart"]);
    // The working deck shows the example from the manifest, its data included.
    let (id, _) = &written[0];
    assert_that!(s.deck().data.contains_key(id.as_str())).is_true();
    let (_, issues) = oku_slides_core::plan_views(s.deck(), &s.registry);
    assert_that!(issues).is_empty();
}

#[test]
fn undo_of_a_layout_example_chart_drops_its_data_from_the_manifest() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    let op = json!({"op": "add_element", "slot": {"owner": {"slide": "layout-content"}, "param": "blocks"},
        "element": {"id": "t1", "kind": "base/table", "params": {"data": {"data": "base-example-table"}}}});
    s.apply("Tableau", vec![serde_json::from_value(op).unwrap()], None).unwrap_or_else(|e| panic!("{}", e.one_line()));
    s.undo().unwrap();
    assert_that!(manifest_data(&s)).is_empty();
    assert_that!(s.deck().data.is_empty()).is_true();
}

/// `data` labelled `label`.
fn labelled(data: &std::sync::Arc<oku_slides_data::Data>, label: &str) -> std::sync::Arc<oku_slides_data::Data> {
    std::sync::Arc::new(oku_slides_data::Data { label: Some(label.to_owned()), ..oku_slides_data::Data::clone(data) })
}

/// Ops adding the data `id` and a table of the layout `content` showing it.
fn table_on_content(id: &str) -> Vec<Op> {
    let table = json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "Month" }],
        "rows": [{ "id": "r1", "cells": ["Jan"] }] });
    let add = json!({ "op": "add_data", "id": id, "name": format!("sales{id}"), "table": table });
    let element = json!({"op": "add_element", "slot": {"owner": {"slide": "layout-content"}, "param": "blocks"},
        "element": {"id": format!("t-{id}"), "kind": "base/table", "params": {"data": {"data": id}}}});
    vec![serde_json::from_value(add).unwrap(), serde_json::from_value(element).unwrap()]
}

#[rstest::rstest]
#[case::unchanged("Ventes", "ftl:example-sales")]
#[case::edited("Coûts", "Coûts")]
fn example_data_keeps_the_manifest_form_only_of_unchanged_data(#[case] next_name: &str, #[case] written: &str) {
    let dir = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(dir.path(), ESSENTIEL_PACK);
    s.apply("Tableau", table_on_content("d1"), None).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let id = oku_slides_data::DataId::try_from("d1".to_owned()).unwrap();
    let mut current = s.deck().clone();
    let shown = current.data.get(&id).cloned().unwrap();
    current.data.insert(id.clone(), labelled(&shown, "Ventes"));
    let mut next = current.clone();
    next.data.insert(id.clone(), labelled(&shown, next_name));
    let kept = std::collections::BTreeMap::from([(id, labelled(&shown, "ftl:example-sales"))]);
    let data = super::example_data(&next, &current, &kept);
    let labels: Vec<Option<&str>> = data.values().map(|d| d.label.as_deref()).collect();
    assert_that!(labels).is_equal_to(vec![Some(written)]);
}

#[test]
fn layout_examples_naming_more_data_than_a_pack_holds_are_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    let ops: Vec<Op> = (0..=oku_slides_core::MAX_PACK_DATA).flat_map(|i| table_on_content(&format!("d{i}"))).collect();
    let before = s.deck().clone();
    let err = s.apply("Tableaux", ops, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::InvalidDocument);
    assert_that!(format!("{err:?}")).contains("Data");
    assert_that!(manifest_data(&s)).is_empty();
    assert_that!(*s.deck()).is_equal_to(&before);
}
