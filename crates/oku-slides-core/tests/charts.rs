//! Charts in the deck: add, options, data refs.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::{data_deck, registry};
use oku_slides_chart::ChartOption;
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

fn chart(id: &str, option: &Value) -> Element {
    serde_json::from_value(json!({ "id": id, "kind": "base/chart", "params": { "option": option } }))
        .unwrap_or_else(|e| panic!("{e}"))
}

fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
    op.apply(deck, &registry())
}

fn deck_with_chart() -> Deck {
    let mut deck = data_deck();
    let option =
        json!({ "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }, { "type": "line" }] });
    apply(&mut deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: chart("g", &option) })
        .unwrap_or_else(|e| panic!("{e}"));
    deck
}

fn option(deck: &Deck) -> ChartOption {
    let e = deck.slides[0].locate("g").unwrap_or_else(|| panic!("no chart g")).element;
    let v = e.params.value("option").unwrap_or_else(|| panic!("no option")).clone();
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

fn set_option(patch: Value) -> Op {
    Op::SetChartOption { target: g(), param: "option".into(), patch }
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
fn set_chart_option_touching_dataset_is_inline_and_deck_unchanged() {
    let mut deck = deck_with_chart();
    let before = deck.clone();
    let err = apply(&mut deck, set_option(json!({ "dataset": { "source": [["M"], ["a"]] } }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Inline));
    assert_that!(err.subject()).is_equal_to(Some("g"));
    assert_that!(deck).is_equal_to(before);
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
    let op = Op::SetChartOption { target: t, param: "option".into(), patch: json!({}) };
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
    let bad = chart("x", &json!({ "series": [] }));
    let err =
        apply(&mut deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: bad }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.subject()).is_equal_to(Some("x"));
}

#[test]
fn add_element_with_inconsistent_nested_chart_is_refused() {
    let mut deck = deck_with_chart();
    let mut stack = Element::new("p", base::STACK);
    stack.params.set("children", ParamValue::Slot(vec![chart("x", &json!({ "series": [] }))]));
    let err =
        apply(&mut deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: stack }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidChart);
}

#[test]
fn set_param_chart_option_accepts_a_consistent_option() {
    let mut deck = deck_with_chart();
    let value = json!({ "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "line" }] });
    apply(&mut deck, Op::SetParam { target: g(), param: "option".into(), value }).unwrap();
    assert_that!(option(&deck).dataset).is_none();
}

#[test]
fn set_param_chart_option_with_inline_series_is_inline() {
    let mut deck = deck_with_chart();
    let inline = json!({ "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] });
    let err = apply(&mut deck, Op::SetParam { target: g(), param: "option".into(), value: inline }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Inline));
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
fn ops_schema_lists_the_chart_ops() {
    let schema = ops_schema().unwrap().to_string();
    assert_that!(schema).contains("set_chart_option");
    assert_that!(schema).contains("edit_data");
}
