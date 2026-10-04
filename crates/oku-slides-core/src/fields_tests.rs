use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_registry;

fn slide(v: serde_json::Value) -> Slide {
    serde_json::from_value(v).unwrap()
}

fn order(s: &Slide) -> Vec<String> {
    let reg = test_registry::shared();
    s.text_fields(&reg).iter().map(|f| format!("{}.{}", f.owner_id, f.name())).collect()
}

#[test]
fn text_fields_of_content_lists_slide_then_elements_in_reading_order() {
    let s = slide(json!({
        "id": "s", "kind": "content",
        "params": { "title": "T", "blocks": [
            { "id": "b1", "kind": "text" },
            { "id": "st", "kind": "stack", "params": { "layout": "columns", "children": [
                { "id": "c1", "kind": "card" }
            ] } }
        ] }
    }));
    assert_that!(order(&s)).is_equal_to(vec![
        "s.kicker".to_owned(),
        "s.title".into(),
        "b1.body".into(),
        "c1.kicker".into(),
        "c1.title".into(),
        "c1.body".into(),
    ]);
}

#[test]
fn text_fields_skip_widgets_without_text_params() {
    let s = slide(json!({ "id": "s", "kind": "blank", "params": { "elements": [
        { "id": "f", "kind": "shape", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 } }
    ] } }));
    assert_that!(order(&s)).is_empty();
}

#[test]
fn text_fields_of_unknown_widget_are_empty() {
    let s = slide(json!({ "id": "s", "kind": "inconnu" }));
    assert_that!(order(&s)).is_empty();
}

#[test]
fn text_fields_expand_list_items_in_item_order() {
    let reg = test_registry::with_list();
    let s = slide(json!({ "id": "s", "kind": "blank", "params": { "elements": [
        { "id": "f", "kind": "liste", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 }, "params": { "etapes": [
            { "id": "d0", "date": "2019", "fait": true },
            { "id": "i-1", "texte": "x" }
        ] } }
    ] } }));
    let names: Vec<String> = s.text_fields(&reg).iter().map(|f| format!("{}.{}", f.owner_id, f.name())).collect();
    assert_that!(names).is_equal_to(vec![
        "f.etapes/d0/date".to_owned(),
        "f.etapes/d0/texte".into(),
        "f.etapes/d0/note".into(),
        "f.etapes/i-1/date".into(),
        "f.etapes/i-1/texte".into(),
        "f.etapes/i-1/note".into(),
    ]);
}

#[test]
fn text_fields_of_an_item_read_the_item_values() {
    let reg = test_registry::with_list();
    let s = slide(json!({ "id": "s", "kind": "blank", "params": { "elements": [
        { "id": "f", "kind": "liste", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 },
          "params": { "etapes": [{ "id": "d0", "date": "2019" }] } }
    ] } }));
    let fields = s.text_fields(&reg);
    let date = fields.iter().find(|f| f.name() == "etapes/d0/date");
    assert_that!(date.map(|f| f.values.text("date"))).is_equal_to(Some(Some("2019")));
    assert_that!(date.and_then(|f| f.list.map(|(l, i)| (l, i.id.as_str())))).is_equal_to(Some(("etapes", "d0")));
}

#[test]
fn text_fields_of_a_plain_param_have_the_owner_values_and_no_list() {
    let s = slide(json!({ "id": "s", "kind": "content", "params": { "title": "T" } }));
    let reg = test_registry::shared();
    let fields = s.text_fields(&reg);
    let title = fields.iter().find(|f| f.name() == "title");
    assert_that!(title.map(|f| f.values.text("title"))).is_equal_to(Some(Some("T")));
    assert_that!(title.is_some_and(|f| f.list.is_none())).is_true();
}
