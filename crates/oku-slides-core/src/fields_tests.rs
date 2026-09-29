use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_registry;

fn slide(v: serde_json::Value) -> Slide {
    serde_json::from_value(v).unwrap()
}

fn order(s: &Slide) -> Vec<String> {
    let reg = test_registry::shared();
    s.text_fields(&reg).iter().map(|f| format!("{}.{}", f.owner_id, f.param.name)).collect()
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
