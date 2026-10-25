use oku_slides_core::{Op, WidgetFile};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::Session;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, session, session_on};

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
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    example(&s, "content", "title")
}

#[test]
fn apply_emptying_a_layout_text_writes_an_empty_example() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn set_example_values_emptying_a_layout_text_keeps_it_empty() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_example_values("content", Map::from_iter([("title".into(), json!(""))])).unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn undo_after_emptying_a_layout_text_brings_the_text_back() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    s.undo().unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(initial_title());
}

#[test]
fn redo_after_undoing_an_emptied_layout_text_empties_it_again() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    s.undo().unwrap();
    s.redo().unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn apply_on_a_layout_slide_reseeds_the_slide_from_its_example() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Titre", vec![set_param("layout-content", "title", json!("Neuf"))], None).unwrap();
    let slide = s.deck().slide("layout-content").cloned().unwrap();
    assert_that!((slide.params.text("title"), example(&s, "content", "title")))
        .is_equal_to((Some("Neuf"), Some(json!("Neuf"))));
}

#[test]
fn resume_after_emptying_a_layout_text_keeps_it_empty() {
    let data = fresh_dir("oku-slides-layout-examples-");
    let dir = fresh_dir("oku-slides-layout-examples-resume-");
    let mut s = session_on(data.path());
    assert_that!(s.resume(dir.path().to_path_buf())).is_true();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    s.flush_resume();
    drop(s);
    let mut back = session_on(data.path());
    assert_that!(back.resume(dir.path().to_path_buf())).is_true();
    assert_that!(example(&back, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn save_and_reopen_after_emptying_a_layout_text_keeps_it_empty() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}

#[test]
fn set_grid_on_a_layout_slide_writes_the_example_grid() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let op = Op::SetGrid { slide: "layout-blank".into(), columns: Some(4), rows: None };
    s.apply("Grille", vec![op], None).unwrap();
    assert_that!(example(&s, "blank", "grid")).is_equal_to(Some(json!({ "columns": 4, "rows": 6 })));
}

#[test]
fn undo_after_set_grid_on_a_layout_slide_restores_the_example() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
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
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
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
    let (mut s, _) = crate::test_support::editing_layout_pack(data.path(), "acme", &["one"]);
    s.set_widget(None, "liste", LIST_LAYOUT.into()).unwrap();
    s.apply("Ajouter", vec![edit_items(json!({ "op": "insert", "id": "x", "values": { "t": "deux" } }))], None)
        .unwrap();
    s.apply("Retirer", vec![edit_items(json!({ "op": "remove", "item": "x" }))], None).unwrap();
    assert_that!(example(&s, "liste", "items")).is_equal_to(Some(json!([])));
}

#[test]
fn apply_setting_a_layout_text_then_emptying_it_in_one_batch_keeps_it_empty() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let ops =
        vec![set_param("layout-content", "title", json!("Neuf")), set_param("layout-content", "title", json!(""))];
    s.apply("Titre", ops, None).unwrap();
    assert_that!(example(&s, "content", "title")).is_equal_to(Some(json!("")));
}
