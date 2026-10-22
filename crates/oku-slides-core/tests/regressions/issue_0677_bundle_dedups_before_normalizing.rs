//! t-677 : un deck JSON dont un slot porte deux copies exactes d'un élément
//! (même id) était normalisé avant d'être dédoublonné : `normalize_hosts`
//! posait des ids de lignes et de colonnes aléatoires différents sur chaque
//! copie, qui ne passaient plus pour exactes, et la relecture renommait l'une
//! en `{id}-2`. `SourceBundle::new` répare donc les ids avant la normalisation.

use oku_slides_core::{Deck, SourceBundle};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn bundle(element: &Value) -> SourceBundle {
    let deck = Deck::from_json(
        &json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": { "blocks": [element, element] } }] })
            .to_string(),
    )
    .unwrap_or_else(|e| panic!("deck : {e}"));
    SourceBundle::new(deck, common::shared()).unwrap_or_else(|e| panic!("bundle : {e}"))
}

fn blocks(deck: &Deck) -> Vec<String> {
    let json = serde_json::to_value(deck).unwrap_or_else(|e| panic!("json : {e}"));
    let blocks = json["slides"][0]["params"]["blocks"].as_array().cloned().unwrap_or_default();
    blocks.iter().filter_map(|b| b["id"].as_str().map(str::to_owned)).collect()
}

fn assert_single(element: &Value) {
    let b = bundle(element);
    assert_that!(blocks(b.deck())).is_equal_to(vec!["e".to_owned()]);
    assert_that!(serde_json::to_string(b.deck()).unwrap_or_default().contains("-2\"")).is_false();
    assert_that!(b.deck()).is_equal_to(&b.replica().deck().0);
}

#[test]
fn bundle_new_with_duplicate_chart_keeps_one() {
    assert_single(&json!({ "id": "e", "kind": "base/chart", "params": { "option": {
        "dataset": { "source": [["M", "V"], ["a", 1]] }, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }));
}

#[test]
fn bundle_new_with_duplicate_table_keeps_one() {
    assert_single(&json!({ "id": "e", "kind": "base/table", "params": { "data": {
        "dataset": { "source": [["Pays", "Ventes"], ["FR", 10]] } } } }));
}

#[test]
fn bundle_new_with_copies_differing_by_edge_whitespace_keeps_one() {
    let a =
        json!({ "id": "e", "kind": "base/text", "params": { "body": [{ "runs": [{ "t": "salut" }] }], "title": "x" } });
    let b = json!({ "id": "e", "kind": "base/text", "params": { "body": [{ "runs": [{ "t": "salut" }] }], "title": " x " } });
    let deck = Deck::from_json(
        &json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": { "blocks": [a, b] } }] }).to_string(),
    )
    .unwrap_or_else(|e| panic!("deck : {e}"));
    let bundle = SourceBundle::new(deck, common::shared()).unwrap_or_else(|e| panic!("bundle : {e}"));
    assert_that!(blocks(bundle.deck())).is_equal_to(vec!["e".to_owned()]);
    assert_that!(bundle.deck()).is_equal_to(&bundle.replica().deck().0);
}
