//! `set_kind` gardait les paramètres de l'ancien widget sans les revalider contre
//! les paramètres d'hôte du nouveau : un `data` quelconque finissait sous `host(table)`.

use oku_slides_core::{Deck, Op, OpErrorKind, WidgetRegistry, apply_all};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    let boite = json!({ "name": "boite", "category": "element", "params": [{ "name": "data", "type": "plain" }] });
    let Some(base) = probe["base"].as_array_mut() else { panic!("sonde : base attendu") };
    let cadre = json!({ "name": "cadre", "category": "element", "params": [{ "name": "data", "type": "grid" }] });
    base.push(boite);
    base.push(cadre);
    let probe = serde_json::from_value(probe).unwrap_or_else(|e| panic!("sonde : {e}"));
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}"))
}

fn deck(params: Value) -> Deck {
    let deck = json!({ "version": 5, "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "b", "kind": "boite", "params": params },
        { "id": "c", "kind": "cadre", "params": { "data": { "columns": 2, "rows": 2 } } },
        { "id": "t", "kind": "text", "params": {} }
    ] } }] });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck : {e}"))
}

fn set_kind(element: &str) -> Op {
    let op = json!({ "op": "set_kind", "target": { "slide": "s", "element": element }, "kind": "table" });
    serde_json::from_value(op).unwrap_or_else(|e| panic!("op : {e}"))
}

#[test]
fn set_kind_to_table_with_string_data_is_refused_and_deck_unchanged() {
    let mut deck = deck(json!({ "data": "texte" }));
    let before = deck.clone();
    let Err(e) = apply_all(&mut deck, vec![set_kind("b")], &registry()) else { panic!("l'op devait être refusée") };
    assert_that!(e.op_error().kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_kind_to_table_without_data_still_succeeds() {
    let mut deck = deck(json!({}));
    let result = apply_all(&mut deck, vec![set_kind("t")], &registry());
    assert_that!(result.is_ok()).is_true();
}

#[test]
fn set_kind_to_table_with_grid_object_data_is_refused_as_invalid_table_and_deck_unchanged() {
    let mut deck = deck(json!({}));
    let before = deck.clone();
    let Err(e) = set_kind("c").apply(&mut deck, &registry()) else { panic!("l'op devait être refusée") };
    assert_that!(e.kind()).is_equal_to(OpErrorKind::InvalidTable);
    assert_that!(deck).is_equal_to(before);
}
