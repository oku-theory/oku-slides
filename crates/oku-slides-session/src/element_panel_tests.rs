use oku_slides_core::{ListOp, ParamValue};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::lib_tests_support::session_with_plan;

/// The `key` of every field event of the surface, in order.
fn keys(s: &crate::Session) -> Vec<String> {
    let surface = s.element_panel("s-constat", "pl").expect("element surface");
    let json = serde_json::to_value(&surface).expect("surface en JSON");
    let components = json["messages"][1]["updateComponents"]["components"].as_array().cloned().unwrap_or_default();
    components.iter().filter_map(|c| c["action"]["event"]["context"]["key"].as_str().map(str::to_owned)).collect()
}

fn items(s: &crate::Session) -> usize {
    let deck = s.deck();
    let located = deck.slide("s-constat").and_then(|slide| slide.locate("pl")).expect("element pl");
    match located.element.params.get("etapes") {
        Some(ParamValue::List(items)) => items.len(),
        _ => 0,
    }
}

#[test]
fn element_panel_fields_carry_param_keys() {
    let s = session_with_plan();
    assert_that!(keys(&s)).is_equal_to(vec!["etapes".to_owned()]);
}

#[test]
fn element_list_insert_adds_one_item_and_undo_removes_it() {
    let mut s = session_with_plan();
    assert_that!(items(&s)).is_equal_to(2);
    let insert = ListOp::Insert {
        index: None,
        id: "d9".into(),
        values: Map::<String, Value>::from_iter([("titre".into(), json!("Mesurer"))]),
    };
    s.element_list("s-constat", "pl", "etapes", insert).unwrap();
    assert_that!(items(&s)).is_equal_to(3);
    s.undo().unwrap();
    assert_that!(items(&s)).is_equal_to(2);
}

#[test]
fn element_list_labels_the_undo_step_with_the_param_label() {
    let mut s = session_with_plan();
    let remove = ListOp::Remove { item: "d1".into() };
    s.element_list("s-constat", "pl", "etapes", remove).unwrap();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Réglage : Étapes".to_owned()));
}
