//! Views drawn through the renderer: deck data reaches Typst and the chart engine as plain
//! datasets, and cell text never runs as Typst markup.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::kinds::{base, essentiel};
use oku_slides_core::tokens::theme_layer;
use oku_slides_core::{DECK_VERSION, Deck};
use oku_slides_data::{DataCell, TableOp};
use oku_slides_i18n::Localizer;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::Renderer;
use crate::charts::SVG_FIELD;
use crate::fonts::Fonts;

/// A deck whose chart `g` and table `t` both read data `d` (month, sales, margin).
fn shared() -> Deck {
    let cells = json!({ "r1/mois": "Jan", "r1/ventes": 3, "r1/marge": 0.2, "r2/mois": "Fév", "r2/ventes": 5, "r2/marge": 0.25 });
    let json = json!({
        "version": DECK_VERSION,
        "data": { "d": { "name": "ventes", "kind": "manual", "table": {
            "generation": "g1",
            "columns": [
                { "id": "mois", "name": "Mois", "type": "text" },
                { "id": "ventes", "name": "Ventes", "type": "number" },
                { "id": "marge", "name": "Marge", "type": "number" }
            ],
            "rows": ["r1", "r2"],
            "cells": cells
        } } },
        "slides": [{ "id": "s", "kind": essentiel::CONTENT, "params": { "blocks": [
            { "id": "g", "kind": base::CHART, "params": { "option": {
                "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } },
            { "id": "t", "kind": base::TABLE, "params": { "data": { "data": "d" } } }
        ] } }]
    });
    Deck::from_json(&json.to_string()).unwrap()
}

fn edit(deck: &mut Deck, change: &TableOp) {
    let data = deck.data.values_mut().next().unwrap();
    Arc::make_mut(data).table.apply(change).unwrap();
}

fn set_cell(row: &str, column: &str, value: DataCell) -> TableOp {
    TableOp::SetCell { row: row.to_owned(), column: column.to_owned(), value }
}

/// The built-in packs, themed with their component defaults only.
fn renderer() -> Renderer {
    let mut r = Renderer::with_builtins(Fonts::default()).unwrap();
    let packs = crate::builtin_component_defaults().unwrap();
    r.set_theme(Some(theme_layer(packs.iter().map(|(ns, d)| (ns, d.clone())), None))).unwrap();
    r
}

/// Element `id` of the deck Typst last read.
fn drawn(r: &Renderer, id: &str) -> Value {
    let deck = r.world.deck_json();
    deck["slides"][0]["params"]["blocks"].as_array().unwrap().iter().find(|b| b["id"] == id).unwrap().clone()
}

fn chart_key(r: &Renderer) -> String {
    drawn(r, "g")[SVG_FIELD].as_str().unwrap().to_owned()
}

fn render(r: &mut Renderer, deck: &Deck) {
    r.render(deck, &BTreeMap::new()).unwrap();
}

#[test]
fn cell_edit_reaches_chart_and_table_render_datasets() {
    let mut r = renderer();
    let mut deck = shared();
    render(&mut r, &deck);
    let before = chart_key(&r);
    edit(&mut deck, &set_cell("r1", "ventes", DataCell::Number(918_273.0)));
    render(&mut r, &deck);
    assert_that!(drawn(&r, "g")["params"]["option"]["dataset"]["source"][1][1]).is_equal_to(json!(918_273.0));
    assert_that!(drawn(&r, "t")["params"]["data"]["dataset"]["source"][1][1]).is_equal_to(json!(918_273.0));
    assert_that!(chart_key(&r)).is_not_equal_to(before);
}

#[test]
fn edit_on_column_chart_does_not_use_leaves_chart_key() {
    let mut r = renderer();
    let mut deck = shared();
    render(&mut r, &deck);
    let before = chart_key(&r);
    edit(&mut deck, &set_cell("r1", "marge", DataCell::Number(0.9)));
    render(&mut r, &deck);
    assert_that!(drawn(&r, "t")["params"]["data"]["dataset"]["source"][1][2]).is_equal_to(json!(0.9));
    assert_that!(chart_key(&r)).is_equal_to(before);
}

/// Markup in cell text, headers, categories and series names stays text: compiling it as
/// Typst would panic.
#[rstest]
#[case::table_cell_and_chart_category(set_cell("r1", "mois", DataCell::Text("#panic()]".to_owned())))]
#[case::header_and_series_name(TableOp::RenameColumn { column: "ventes".to_owned(), name: "#panic()]".to_owned() })]
fn typst_markup_in_resolved_text_renders_as_text(#[case] change: TableOp) {
    let mut r = renderer();
    let mut deck = shared();
    edit(&mut deck, &change);
    let compiled = r.render(&deck, &BTreeMap::new());
    assert_that!(compiled.map(|c| c.page_count())).is_ok_containing(1);
    assert_that!(drawn(&r, "t")["params"]["data"]["dataset"].to_string()).contains("#panic()]");
    let svg = r.world.chart_svg(&chart_key(&r)).unwrap();
    assert_that!(svg).contains("#panic()]");
}

#[test]
fn chart_svg_with_script_in_cell_has_no_script_element() {
    let mut r = renderer();
    let mut deck = shared();
    edit(&mut deck, &set_cell("r1", "mois", DataCell::Text("<script>alert(1)</script>".to_owned())));
    render(&mut r, &deck);
    let svg = r.world.chart_svg(&chart_key(&r)).unwrap();
    assert_that!(svg.to_lowercase()).does_not_contain("<script");
}

#[test]
fn redraw_after_language_change_keeps_datasets() {
    let mut r = renderer();
    render(&mut r, &shared());
    let before = drawn(&r, "g")["params"]["option"]["dataset"].clone();
    r.set_language(&Localizer::new(Some("fr"))).unwrap();
    let after = drawn(&r, "g");
    assert_that!(after.get(SVG_FIELD)).is_some();
    assert_that!(after["params"]["option"]["dataset"]).is_equal_to(&before);
}
