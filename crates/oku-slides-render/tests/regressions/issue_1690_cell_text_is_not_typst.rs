//! Deck data reaches Typst as JSON strings: a cell or column name written as Typst markup
//! is drawn as text in the table and the chart, never evaluated (t-1690).

use std::collections::BTreeMap;

use oku_slides_core::kinds::{base, essentiel};
use oku_slides_core::{DECK_VERSION, Deck};
use serde_json::json;
use speculoos::prelude::*;

const MARKUP: &str = "#panic(\"evaluated\")] = [#{}";

fn deck() -> Deck {
    let json = json!({
        "version": DECK_VERSION,
        "data": { "d": { "name": "ventes", "kind": "manual", "table": {
            "generation": "g",
            "columns": [
                { "id": "mois", "name": MARKUP, "type": "text" },
                { "id": "ventes", "name": MARKUP, "type": "number" }
            ],
            "rows": ["r1"],
            "cells": { "r1/mois": MARKUP, "r1/ventes": 3 }
        } } },
        "slides": [{ "id": "s", "kind": essentiel::CONTENT, "params": { "blocks": [
            { "id": "g", "kind": base::CHART, "params": { "option": {
                "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } },
            { "id": "t", "kind": base::TABLE, "params": { "data": { "data": "d" } } }
        ] } }]
    });
    Deck::from_json(&json.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn markup_in_data_text_compiles_as_text() {
    let mut r = crate::common::renderer();
    let compiled = r.render(&deck(), &BTreeMap::new()).map(|c| c.page_count());
    assert_that!(compiled.map_err(|e| e.to_string())).is_ok_containing(1);
}
