//! Les graphiques dans le deck : ajout, options, données.

mod common;

use common::registry;
use oku_slides_chart::{Cell, ChartOption, DataOp};
use oku_slides_core::kinds::base;
use oku_slides_core::*;
use oku_slides_i18n::{Localize, Localizer};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn blocks() -> SlotRef {
    SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() }
}

fn g() -> Target {
    Target { slide: "s".into(), element: Some("g".into()) }
}

fn chart(id: &str, option: Value) -> Element {
    serde_json::from_value(json!({ "id": id, "kind": "base/chart", "params": { "option": option } }))
        .unwrap_or_else(|e| panic!("{e}"))
}

fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
    op.apply(deck, &registry())
}

fn deck_with_chart() -> Deck {
    let mut deck = Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#).unwrap_or_else(|e| panic!("{e}"));
    let option = json!({
        "dataset": { "source": [["Mois", "Ventes", "Objectif"], ["Jan", 10, 12], ["Fév", 14, 12]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }, { "type": "line" }]
    });
    apply(&mut deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: chart("g", option) })
        .unwrap_or_else(|e| panic!("{e}"));
    deck
}

fn option(deck: &Deck) -> ChartOption {
    let e = deck.slides[0].locate("g").unwrap_or_else(|| panic!("graphique g absent")).element;
    let v = e.params.value("option").unwrap_or_else(|| panic!("option absente")).clone();
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

fn set_option(patch: Value) -> Op {
    Op::SetChartOption { target: g(), param: "option".into(), patch }
}

fn data(change: DataOp) -> Op {
    Op::SetData { target: g(), param: "option".into(), change }
}

#[test]
fn chart_element_round_trips_through_json() {
    let deck = deck_with_chart();
    let back = Deck::from_json(&deck.to_json().unwrap()).unwrap();
    assert_that!(back).is_equal_to(&deck);
    assert_that!(validate(&deck, &registry(), None)).is_empty();
}

#[test]
fn set_chart_option_sets_title_and_legend() {
    let mut deck = deck_with_chart();
    apply(&mut deck, set_option(json!({ "title": { "text": "Ventes" }, "legend": { "show": false } }))).unwrap();
    assert_that!(option(&deck).title.as_ref().and_then(|t| t.text.clone())).is_equal_to(Some("Ventes".to_owned()));
}

#[test]
fn set_chart_option_touching_dataset_is_refused_and_deck_unchanged() {
    let mut deck = deck_with_chart();
    let before = deck.clone();
    let err = apply(&mut deck, set_option(json!({ "dataset": { "source": [] } }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.localize(&Localizer::new(Some("fr")))).starts_with("Graphique g : ");
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_data_cell_sets_the_value() {
    let mut deck = deck_with_chart();
    apply(&mut deck, data(DataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) })).unwrap();
    let source = option(&deck).dataset.unwrap().source;
    assert_that!(source[1][1]).is_equal_to(Cell::Number(99.0));
}

#[test]
fn set_data_column_ops_resize_series() {
    let mut deck = deck_with_chart();
    apply(&mut deck, data(DataOp::RemoveColumn { at: 2 })).unwrap();
    assert_that!(option(&deck).series.len()).is_equal_to(1);
    let insert = DataOp::InsertColumn {
        at: 2,
        cells: vec![Cell::Text("Marge".into()), Cell::Number(1.0), Cell::Number(2.0)],
        id: None,
    };
    apply(&mut deck, data(insert)).unwrap();
    assert_that!(option(&deck).series.len()).is_equal_to(2);
}

#[test]
fn set_data_leaving_no_value_column_is_refused_and_deck_unchanged() {
    let mut deck = deck_with_chart();
    apply(&mut deck, data(DataOp::RemoveColumn { at: 2 })).unwrap();
    let before = deck.clone();
    let err = apply(&mut deck, data(DataOp::RemoveColumn { at: 1 })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_data_bad_row_is_refused() {
    let mut deck = deck_with_chart();
    let err = apply(&mut deck, data(DataOp::RemoveRow { at: 9 })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.subject()).is_equal_to(Some("g"));
}

#[test]
fn chart_ops_on_a_text_element_return_unknown_param() {
    let mut deck = deck_with_chart();
    apply(
        &mut deck,
        Op::AddElement { slot: blocks(), index: None, defaults: false, element: Element::text("t", vec![]) },
    )
    .unwrap();
    let t = Target { slide: "s".into(), element: Some("t".into()) };
    let op = Op::SetChartOption { target: t.clone(), param: "option".into(), patch: json!({}) };
    assert_that!(apply(&mut deck, op).unwrap_err().kind()).is_equal_to(OpErrorKind::UnknownParam);
    let op = Op::SetData { target: t, param: "option".into(), change: DataOp::RemoveRow { at: 1 } };
    assert_that!(apply(&mut deck, op).unwrap_err().kind()).is_equal_to(OpErrorKind::UnknownParam);
}

#[test]
fn chart_ops_on_a_non_chart_param_return_param_type() {
    let mut deck = deck_with_chart();
    apply(
        &mut deck,
        Op::AddElement { slot: blocks(), index: None, defaults: false, element: Element::text("t", vec![]) },
    )
    .unwrap();
    let t = Target { slide: "s".into(), element: Some("t".into()) };
    let err = apply(&mut deck, Op::SetChartOption { target: t, param: "body".into(), patch: json!({}) }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.localize(&Localizer::new(Some("en"))))
        .is_equal_to("Invalid value for body: a chart option is expected".to_owned());
}

#[test]
fn add_element_with_inconsistent_chart_is_refused() {
    let mut deck = deck_with_chart();
    let bad = chart("x", json!({ "series": [] }));
    let err =
        apply(&mut deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: bad }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.subject()).is_equal_to(Some("x"));
}

#[test]
fn add_element_with_inconsistent_nested_chart_is_refused() {
    let mut deck = deck_with_chart();
    let mut stack = Element::new("p", base::STACK);
    stack.params.set("children", ParamValue::Slot(vec![chart("x", json!({ "series": [] }))]));
    let err =
        apply(&mut deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: stack }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
}

#[test]
fn set_param_chart_option_accepts_a_consistent_option() {
    let mut deck = deck_with_chart();
    let inline = json!({ "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] });
    apply(&mut deck, Op::SetParam { target: g(), param: "option".into(), value: inline }).unwrap();
    assert_that!(option(&deck).dataset).is_none();
}

#[test]
fn set_param_chart_option_inconsistent_returns_invalid_chart() {
    let mut deck = deck_with_chart();
    let before = deck.clone();
    let op = Op::SetParam { target: g(), param: "option".into(), value: json!({ "series": [] }) };
    assert_that!(apply(&mut deck, op).unwrap_err().kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_data_cell_that_adds_a_column_aligns_the_series() {
    // Deux colonnes de valeurs, une seule série : la modification aligne les séries.
    let mut deck = deck_with_chart();
    apply(&mut deck, set_option(json!({ "series": [{ "type": "bar" }] }))).unwrap();
    apply(&mut deck, data(DataOp::SetCell { row: 1, col: 1, value: Cell::Number(5.0) })).unwrap();
    assert_that!(option(&deck).series.len()).is_equal_to(2);
}

#[test]
fn set_data_on_inline_chart_without_dataset_is_invalid_value() {
    let mut deck = Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#).unwrap();
    let inline = chart("g", json!({ "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] }));
    apply(&mut deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: inline }).unwrap();
    let err = apply(&mut deck, data(DataOp::RemoveRow { at: 1 })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn ops_schema_lists_the_chart_ops() {
    let schema = ops_schema().unwrap().to_string();
    assert_that!(schema).contains("set_chart_option");
    assert_that!(schema).contains("set_data");
}
