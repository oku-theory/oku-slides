//! Tables in the deck: add, replace, options, data refs, validation.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::{data_deck, registry};
use oku_slides_core::kinds::base;
use oku_slides_core::*;
use oku_slides_i18n::{Localize, Localizer};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn blocks() -> SlotRef {
    SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() }
}

fn t1() -> Target {
    Target { slide: "s".into(), element: Some("t1".into()) }
}

fn element(v: Value) -> Element {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

fn table(id: &str, data: &Value) -> Element {
    element(json!({ "id": id, "kind": "base/table", "params": { "data": data } }))
}

fn chart(id: &str) -> Element {
    element(json!({ "id": id, "kind": "base/chart", "params": { "option": {
        "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }))
}

/// A table view of the deck data `d`.
fn data() -> Value {
    json!({ "data": "d" })
}

/// A table option carrying its rows inline, as older decks stored them.
fn inline() -> Value {
    json!({ "dataset": { "source": [["Country", "Sales"], ["FR", 10]] } })
}

fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
    op.apply(deck, &registry())
}

fn add(deck: &mut Deck, e: Element) -> Result<(), OpError> {
    apply(deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: e })
}

fn deck_with(e: Element) -> Deck {
    let mut deck = data_deck();
    add(&mut deck, e).unwrap_or_else(|e| panic!("{e}"));
    deck
}

fn stored(deck: &Deck) -> Value {
    let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("no table t1")).element;
    e.params.value("data").unwrap_or_else(|| panic!("no data")).clone()
}

fn kind_of(r: Result<(), OpError>) -> OpErrorKind {
    r.err().unwrap_or_else(|| panic!("refusal expected")).kind()
}

const INLINE: OpErrorKind = OpErrorKind::DataRefused(DataFaultKind::Inline);

#[test]
fn add_element_table_stores_its_data_ref() {
    let deck = deck_with(table("t1", &data()));
    assert_that!(stored(&deck)["data"]).is_equal_to(json!("d"));
    assert_that!(stored(&deck).get("dataset")).is_none();
}

#[test]
fn add_element_inline_table_is_inline_and_deck_unchanged() {
    let mut deck = data_deck();
    let before = deck.clone();
    assert_that!(kind_of(add(&mut deck, table("t1", &inline())))).is_equal_to(INLINE);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn add_element_inline_table_inside_a_stack_is_inline() {
    let stack = element(json!({ "id": "p", "kind": "base/stack", "params": { "children": [
        { "id": "t1", "kind": "base/table", "params": { "data": inline() } } ] } }));
    assert_that!(kind_of(add(&mut data_deck(), stack))).is_equal_to(INLINE);
}

#[test]
fn add_element_inline_table_localizes_subject_in_french() {
    let err = add(&mut data_deck(), table("t1", &inline())).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr_with("op-data-inline", &[("subject", "t1".into())]));
}

#[test]
fn replace_element_chart_by_table_keeps_the_id() {
    let mut deck = deck_with(chart("t1"));
    apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: table("t1", &data()) }).unwrap();
    let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("no t1")).element;
    assert_that!(e.kind.as_str()).is_equal_to(base::TABLE);
}

#[test]
fn replace_element_by_inline_table_is_inline_and_deck_unchanged() {
    let mut deck = deck_with(chart("t1"));
    let before = deck.clone();
    let r = apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: table("t1", &inline()) });
    assert_that!(kind_of(r)).is_equal_to(INLINE);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_param_inline_object_is_inline() {
    let mut deck = deck_with(table("t1", &data()));
    let r = apply(&mut deck, Op::SetParam { target: t1(), param: "data".into(), value: inline() });
    assert_that!(kind_of(r)).is_equal_to(INLINE);
}

#[test]
fn set_param_string_is_param_type() {
    let mut deck = deck_with(table("t1", &data()));
    let r = apply(&mut deck, Op::SetParam { target: t1(), param: "data".into(), value: json!("text") });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn set_table_option_striped_true_stores_it_and_keeps_the_ref() {
    let mut deck = deck_with(table("t1", &data()));
    let patch = json!({ "striped": true });
    apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch }).unwrap();
    assert_that!(stored(&deck)["striped"]).is_equal_to(json!(true));
    assert_that!(stored(&deck)["data"]).is_equal_to(json!("d"));
}

#[test]
fn set_table_option_touching_dataset_is_inline_and_deck_unchanged() {
    let mut deck = deck_with(table("t1", &data()));
    let before = deck.clone();
    let patch = json!({ "dataset": { "source": [["A"], [1]] } });
    let r = apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch });
    assert_that!(kind_of(r)).is_equal_to(INLINE);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_table_option_on_a_chart_is_param_type() {
    let mut deck = deck_with(chart("t1"));
    let r = apply(&mut deck, Op::SetTableOption { target: t1(), param: "option".into(), patch: json!({}) });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn set_chart_option_on_a_table_is_param_type() {
    let mut deck = deck_with(table("t1", &data()));
    let r = apply(&mut deck, Op::SetChartOption { target: t1(), param: "data".into(), patch: json!({}) });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn validate_deck_with_valid_table_reports_nothing() {
    assert_that!(validate(&deck_with(table("t1", &data())), &registry(), None)).is_empty();
}

#[test]
fn ops_schema_lists_edit_data() {
    let schema = ops_schema().unwrap().to_string();
    assert_that!(schema).contains("\"edit_data\"");
    assert_that!(schema).does_not_contain("\"set_data\"");
}

#[test]
fn ops_schema_lists_set_table_option() {
    assert_that!(ops_schema().unwrap().to_string()).contains("set_table_option");
}
