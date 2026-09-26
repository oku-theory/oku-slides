//! Les graphiques dans le deck : ajout, options, données, et leurs inverses.

use oku_slides_chart::{Cell, ChartDataOp};
use oku_slides_core::*;
use oku_slides_i18n::{Localize, Localizer};
use serde_json::json;
use speculoos::prelude::*;

fn deck_with_chart() -> Deck {
    let mut deck = Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#).unwrap_or_else(|e| panic!("{e}"));
    let chart = serde_json::from_value(json!({
        "id": "g",
        "kind": "chart",
        "option": {
            "dataset": { "source": [["Mois", "Ventes", "Objectif"], ["Jan", 10, 12], ["Fév", 14, 12]] },
            "xAxis": { "type": "category" },
            "series": [{ "type": "bar" }, { "type": "line" }]
        }
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    Op::AddElement { slide: "s".into(), zone: Zone::Blocks, parent: None, index: None, element: chart }
        .apply(&mut deck)
        .unwrap_or_else(|e| panic!("{e}"));
    deck
}

fn option(deck: &Deck) -> &oku_slides_chart::ChartOption {
    match &deck.slides[0].blocks[0].body {
        ElementBody::Chart { option } => option,
        other => panic!("graphique attendu, pas {}", other.kind_name()),
    }
}

fn data(change: ChartDataOp) -> Op {
    Op::SetChartData { slide: "s".into(), element: "g".into(), change }
}

#[test]
fn chart_element_round_trips_through_json() {
    let deck = deck_with_chart();
    let back = Deck::from_json(&deck.to_json().unwrap()).unwrap();
    assert_that!(back).is_equal_to(&deck);
    assert_that!(deck.slides[0].blocks[0].body.kind_name()).is_equal_to("chart");
}

#[test]
fn set_chart_option_then_inverse_restores_the_chart() {
    let mut deck = deck_with_chart();
    let before = deck.clone();
    let patch = json!({ "title": { "text": "Ventes" }, "legend": { "show": false } });
    let inverse = Op::SetChartOption { slide: "s".into(), element: "g".into(), patch }.apply(&mut deck).unwrap();
    assert_that!(option(&deck).title.as_ref().and_then(|t| t.text.as_deref())).is_equal_to(Some("Ventes"));
    inverse.apply(&mut deck).unwrap();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_chart_option_touching_dataset_is_refused_and_deck_unchanged() {
    let mut deck = deck_with_chart();
    let before = deck.clone();
    let patch = json!({ "dataset": { "source": [] } });
    let err = Op::SetChartOption { slide: "s".into(), element: "g".into(), patch }.apply(&mut deck).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.localize(&Localizer::new(Some("fr")))).starts_with("Graphique g : ");
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_chart_data_cell_then_inverse_restores_the_value() {
    let mut deck = deck_with_chart();
    let before = deck.clone();
    let inverse = data(ChartDataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) }).apply(&mut deck).unwrap();
    let source = &option(&deck).dataset.as_ref().unwrap().source;
    assert_that!(source[1][1]).is_equal_to(Cell::Number(99.0));
    inverse.apply(&mut deck).unwrap();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_chart_data_column_ops_resize_series_and_undo_exactly() {
    let mut deck = deck_with_chart();
    let before = deck.clone();
    let removed = data(ChartDataOp::RemoveColumn { at: 2 }).apply(&mut deck).unwrap();
    assert_that!(option(&deck).series.len()).is_equal_to(1);
    let added = data(ChartDataOp::InsertColumn {
        at: 2,
        cells: vec![Cell::Text("Marge".into()), Cell::Number(1.0), Cell::Number(2.0)],
    })
    .apply(&mut deck)
    .unwrap();
    assert_that!(option(&deck).series.len()).is_equal_to(2);
    added.apply(&mut deck).unwrap();
    // La série « ligne » retirée revient telle quelle, pas copiée sur la barre.
    removed.apply(&mut deck).unwrap();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_chart_data_leaving_no_value_column_is_refused_and_deck_unchanged() {
    let mut deck = deck_with_chart();
    data(ChartDataOp::RemoveColumn { at: 2 }).apply(&mut deck).unwrap();
    let before = deck.clone();
    let err = data(ChartDataOp::RemoveColumn { at: 1 }).apply(&mut deck).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_chart_data_bad_row_is_refused() {
    let mut deck = deck_with_chart();
    let err = data(ChartDataOp::RemoveRow { at: 9 }).apply(&mut deck).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.subject()).is_equal_to(Some("g"));
}

#[test]
fn chart_ops_on_a_text_element_return_not_a_chart() {
    let mut deck = deck_with_chart();
    Op::AddElement {
        slide: "s".into(),
        zone: Zone::Blocks,
        parent: None,
        index: None,
        element: Element::text("t", vec![]),
    }
    .apply(&mut deck)
    .unwrap();
    let err =
        Op::SetChartOption { slide: "s".into(), element: "t".into(), patch: json!({}) }.apply(&mut deck).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::NotAChart);
    let err = Op::SetChartData { slide: "s".into(), element: "t".into(), change: ChartDataOp::RemoveRow { at: 1 } }
        .apply(&mut deck)
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::NotAChart);
    assert_that!(err.localize(&Localizer::new(Some("en")))).is_equal_to("t is not a chart".to_owned());
}

#[test]
fn inconsistent_chart_does_not_deserialize() {
    let bad = json!({ "slides": [{ "id": "s", "kind": "content",
        "blocks": [{ "id": "x", "kind": "chart", "option": { "series": [] } }] }] });
    let err = Deck::from_json(&bad.to_string()).unwrap_err();
    assert_that!(err.to_string()).contains("at least one series");
}

#[test]
fn add_element_with_inconsistent_chart_built_in_rust_is_refused() {
    let mut deck = deck_with_chart();
    let mut bad = deck.slides[0].blocks[0].clone();
    bad.id = "x".into();
    if let ElementBody::Chart { option } = &mut bad.body {
        option.series.clear();
    }
    let err = Op::AddElement { slide: "s".into(), zone: Zone::Blocks, parent: None, index: None, element: bad }
        .apply(&mut deck)
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
}

#[test]
fn set_chart_data_cell_that_adds_series_undoes_them_too() {
    // Deux colonnes de valeurs, une seule série : la modification aligne les séries.
    let mut deck = deck_with_chart();
    Op::SetChartOption { slide: "s".into(), element: "g".into(), patch: json!({ "series": [{ "type": "bar" }] }) }
        .apply(&mut deck)
        .unwrap();
    let before = deck.clone();
    let inverse = data(ChartDataOp::SetCell { row: 1, col: 1, value: Cell::Number(5.0) }).apply(&mut deck).unwrap();
    assert_that!(option(&deck).series.len()).is_equal_to(2);
    inverse.apply(&mut deck).unwrap();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_chart_data_on_inline_chart_without_dataset_is_invalid_value() {
    let mut deck = Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#).unwrap();
    let inline = serde_json::from_value(json!({
        "id": "g", "kind": "chart",
        "option": { "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] }
    }))
    .unwrap();
    Op::AddElement { slide: "s".into(), zone: Zone::Blocks, parent: None, index: None, element: inline }
        .apply(&mut deck)
        .unwrap();
    let err = data(ChartDataOp::RemoveRow { at: 1 }).apply(&mut deck).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn ops_schema_lists_the_chart_ops() {
    let schema = ops_schema().unwrap().to_string();
    assert_that!(schema).contains("set_chart_option");
    assert_that!(schema).contains("set_chart_data");
}
