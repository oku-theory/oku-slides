//! Les listes n'avaient pas de bornes de taille dans le cœur : une op (de l'agent
//! comme de l'interface) pouvait poser des cellules ou des items sans limite.

use oku_slides_core::{Deck, LIST_ITEMS_MAX, Op, OpErrorKind, RICH_PARAGRAPHS_MAX, apply_all};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn deck() -> Deck {
    let deck = json!({ "version": 5, "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "n", "kind": "liste", "params": { "etapes": [{ "id": "a" }] } }
    ] } }] });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck : {e}"))
}

fn refused(op: Value) -> OpErrorKind {
    let op: Op = serde_json::from_value(op).unwrap_or_else(|e| panic!("op : {e}"));
    let mut deck = deck();
    let before = deck.clone();
    let Err(e) = apply_all(&mut deck, vec![op], &common::list_registry()) else { panic!("l'op devait être refusée") };
    assert_that!(deck).is_equal_to(before);
    e.op_error().kind()
}

#[test]
fn set_param_list_beyond_the_default_item_bound_is_refused() {
    let rows: Vec<Value> = (0..=LIST_ITEMS_MAX).map(|i| json!({ "id": format!("i{i}") })).collect();
    let op = json!({ "op": "set_param", "target": { "slide": "s", "element": "n" }, "param": "etapes", "value": rows });
    assert_that!(refused(op)).is_equal_to(OpErrorKind::ListBound);
}

#[test]
fn edit_list_rich_cell_beyond_the_paragraph_bound_is_refused() {
    let paragraphs: Vec<Value> = (0..=RICH_PARAGRAPHS_MAX).map(|_| json!({ "runs": [{ "t": "x" }] })).collect();
    let op = json!({ "op": "edit_list", "target": { "slide": "s", "element": "n" }, "param": "etapes",
        "change": { "op": "set_field", "item": "a", "field": "note", "value": paragraphs } });
    assert_that!(refused(op)).is_equal_to(OpErrorKind::ListBound);
}
