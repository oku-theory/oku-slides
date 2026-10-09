//! Les tableaux dans le deck : ajout, remplacement, options, données, validation.

mod common;

use common::registry;
use oku_slides_chart::{Cell, DataOp};
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

fn table(id: &str, data: Value) -> Element {
    element(json!({ "id": id, "kind": "table", "params": { "data": data } }))
}

fn chart(id: &str) -> Element {
    element(json!({ "id": id, "kind": "chart", "params": { "option": {
        "dataset": { "source": [["M", "V"], ["a", 1]] }, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }))
}

fn data() -> Value {
    json!({ "dataset": { "source": [["Pays", "Ventes"], ["FR", 10]] } })
}

fn ragged() -> Value {
    json!({ "dataset": { "source": [["A", "B"], [1]] } })
}

fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
    op.apply(deck, &registry())
}

fn empty_deck() -> Deck {
    Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#).unwrap_or_else(|e| panic!("{e}"))
}

fn add(deck: &mut Deck, e: Element) -> Result<(), OpError> {
    apply(deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: e })
}

fn deck_with(e: Element) -> Deck {
    let mut deck = empty_deck();
    add(&mut deck, e).unwrap_or_else(|e| panic!("{e}"));
    deck
}

fn stored(deck: &Deck) -> Value {
    let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("tableau t1 absent")).element;
    e.params.value("data").unwrap_or_else(|| panic!("data absent")).clone()
}

fn kind_of(r: Result<(), OpError>) -> OpErrorKind {
    r.err().unwrap_or_else(|| panic!("refus attendu")).kind()
}

#[test]
fn add_element_valid_table_stores_its_data() {
    let deck = deck_with(table("t1", data()));
    assert_that!(stored(&deck)["dataset"]["source"][1][0]).is_equal_to(json!("FR"));
}

#[test]
fn add_element_ragged_table_is_invalid_table() {
    assert_that!(kind_of(add(&mut empty_deck(), table("t1", ragged())))).is_equal_to(OpErrorKind::InvalidTable);
}

#[test]
fn add_element_ragged_table_inside_a_stack_is_invalid_table() {
    let stack = element(json!({ "id": "p", "kind": "stack", "params": { "children": [
        { "id": "t1", "kind": "table", "params": { "data": ragged() } } ] } }));
    assert_that!(kind_of(add(&mut empty_deck(), stack))).is_equal_to(OpErrorKind::InvalidTable);
}

#[test]
fn add_element_empty_table_localizes_subject_and_reason_in_french() {
    let err = add(&mut empty_deck(), table("t1", json!({ "dataset": { "source": [] } }))).unwrap_err();
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).starts_with("Tableau t1");
    assert_that!(fr).contains("au moins une ligne");
}

#[test]
fn replace_element_chart_by_table_keeps_the_id() {
    let mut deck = deck_with(chart("t1"));
    apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: table("t1", data()) }).unwrap();
    let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("t1 absent")).element;
    assert_that!(e.kind.as_str()).is_equal_to("table");
}

#[test]
fn replace_element_by_ragged_table_is_invalid_table_and_deck_unchanged() {
    let mut deck = deck_with(chart("t1"));
    let before = deck.clone();
    let r = apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: table("t1", ragged()) });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_param_ragged_object_is_invalid_table() {
    let mut deck = deck_with(table("t1", data()));
    let r = apply(&mut deck, Op::SetParam { target: t1(), param: "data".into(), value: ragged() });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
}

#[test]
fn set_param_string_is_param_type() {
    let mut deck = deck_with(table("t1", data()));
    let r = apply(&mut deck, Op::SetParam { target: t1(), param: "data".into(), value: json!("texte") });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn set_data_on_a_table_edits_its_cell() {
    let mut deck = deck_with(table("t1", data()));
    let change = DataOp::SetCell { row: 1, col: 1, value: Cell::Number(42.0) };
    apply(&mut deck, Op::SetData { target: t1(), param: "data".into(), change }).unwrap();
    assert_that!(stored(&deck)["dataset"]["source"][1][1]).is_equal_to(json!(42.0));
}

#[test]
fn set_data_on_a_table_beyond_limits_is_invalid_table_and_deck_unchanged() {
    let mut deck = deck_with(table("t1", data()));
    let before = deck.clone();
    let change = DataOp::Replace { source: vec![vec![Cell::Null; 21]] };
    let r = apply(&mut deck, Op::SetData { target: t1(), param: "data".into(), change });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_table_option_striped_true_stores_it() {
    let mut deck = deck_with(table("t1", data()));
    let patch = json!({ "striped": true });
    apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch }).unwrap();
    assert_that!(stored(&deck)["striped"]).is_equal_to(json!(true));
}

#[test]
fn set_table_option_touching_dataset_is_invalid_table_and_deck_unchanged() {
    let mut deck = deck_with(table("t1", data()));
    let before = deck.clone();
    let patch = json!({ "dataset": null });
    let r = apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
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
    let mut deck = deck_with(table("t1", data()));
    let r = apply(&mut deck, Op::SetChartOption { target: t1(), param: "data".into(), patch: json!({}) });
    assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn validate_deck_with_valid_table_reports_nothing() {
    assert_that!(validate(&deck_with(table("t1", data())), &registry(), None)).is_empty();
}

#[test]
fn validate_deck_with_ragged_table_reports_one_issue() {
    let deck = Deck::from_json(
        &json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
            { "id": "t1", "kind": "table", "params": { "data": ragged() } } ] } }] })
        .to_string(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(validate(&deck, &registry(), None)).has_length(1);
}

#[test]
fn ops_schema_lists_set_data() {
    assert_that!(ops_schema().unwrap().to_string()).contains("set_data");
}

#[test]
fn ops_schema_lists_set_table_option() {
    assert_that!(ops_schema().unwrap().to_string()).contains("set_table_option");
}

fn ids_of(v: &Value, key: &str, len: usize) -> Vec<String> {
    let list = v["dataset"][key].as_array().unwrap_or_else(|| panic!("{key} absent: {v}"));
    assert_eq!(list.len(), len, "{key} mal aligné: {v}");
    list.iter().map(|i| i.as_str().unwrap_or_else(|| panic!("id non texte")).to_owned()).collect()
}

fn assert_aligned(v: &Value, rows: usize, cols: usize) {
    for (key, len) in [("ids", rows), ("columns", cols)] {
        let list = ids_of(v, key, len);
        let mut unique = list.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), list.len(), "{key} répétés: {v}");
    }
}

#[test]
fn add_element_table_without_ids_stores_aligned_ids_and_columns() {
    let deck = deck_with(table("t1", data()));
    let v = stored(&deck);
    assert_aligned(&v, 2, 2);
    assert_that!(v["dataset"]["source"]).is_equal_to(json!([["Pays", "Ventes"], ["FR", 10.0]]));
}

#[test]
fn replace_element_chart_without_ids_stores_ids() {
    let mut deck = deck_with(table("t1", data()));
    apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: chart("t1") }).unwrap_or_else(|e| panic!("{e}"));
    let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("absent")).element;
    let v = e.params.value("option").unwrap_or_else(|| panic!("option absente")).clone();
    assert_aligned(&v, 2, 2);
}

#[test]
fn set_param_table_without_ids_stores_ids() {
    let mut deck = deck_with(table("t1", data()));
    let value = json!({ "dataset": { "source": [["A", "B", "C"], [1, 2, 3], [4, 5, 6]] } });
    apply(&mut deck, Op::SetParam { target: t1(), param: "data".into(), value }).unwrap_or_else(|e| panic!("{e}"));
    assert_aligned(&stored(&deck), 3, 3);
}

#[test]
fn add_slide_with_table_without_ids_stores_ids() {
    let mut deck = empty_deck();
    let slide: Slide = serde_json::from_value(json!({ "id": "s2", "kind": "content", "params": { "blocks": [
        { "id": "t1", "kind": "table", "params": { "data": data() } }] } }))
    .unwrap_or_else(|e| panic!("{e}"));
    apply(&mut deck, Op::AddSlide { index: None, defaults: false, slide }).unwrap_or_else(|e| panic!("{e}"));
    let e = deck.slide("s2").and_then(|s| s.locate("t1")).unwrap_or_else(|| panic!("absent")).element;
    let v = e.params.value("data").unwrap_or_else(|| panic!("data absente")).clone();
    assert_aligned(&v, 2, 2);
}

#[test]
fn apply_all_keeps_existing_ids() {
    let mut deck = deck_with(table("t1", data()));
    let before = stored(&deck);
    let change = DataOp::SetCell { row: 1, col: 1, value: Cell::Number(7.0) };
    apply(&mut deck, Op::SetData { target: t1(), param: "data".into(), change }).unwrap_or_else(|e| panic!("{e}"));
    let patch = json!({ "striped": true });
    apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch })
        .unwrap_or_else(|e| panic!("{e}"));
    let after = stored(&deck);
    assert_that!(after["dataset"]["ids"]).is_equal_to(before["dataset"]["ids"].clone());
    assert_that!(after["dataset"]["columns"]).is_equal_to(before["dataset"]["columns"].clone());
}

/// Un deck dont l'élément `t1` porte `params` tels quels, sans passer par les ops.
fn forged_deck(kind: &str, params: Value) -> Deck {
    let deck = json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "t1", "kind": kind, "params": params }] } }] });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

fn forged_stored(deck: &Deck, param: &str) -> Value {
    let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("t1 absent")).element;
    e.params.value(param).unwrap_or_else(|| panic!("{param} absent")).clone()
}

#[test]
fn add_element_table_inside_a_stack_stores_aligned_ids_and_columns() {
    let stack = element(json!({ "id": "p", "kind": "stack", "params": { "children": [
        { "id": "t1", "kind": "table", "params": { "data": data() } } ] } }));
    let deck = deck_with(stack);
    let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("t1 absent")).element;
    let v = e.params.value("data").unwrap_or_else(|| panic!("data absent")).clone();
    assert_aligned(&v, 2, 2);
}

#[test]
fn set_kind_table_keeping_a_value_without_ids_normalizes_it() {
    let mut deck = forged_deck("table", json!({ "data": data() }));
    assert_that!(forged_stored(&deck, "data")["dataset"]["ids"].is_null()).is_true();
    apply(&mut deck, Op::SetKind { target: t1(), kind: "table".into() }).unwrap_or_else(|e| panic!("{e}"));
    assert_aligned(&forged_stored(&deck, "data"), 2, 2);
}

#[test]
fn set_chart_option_on_a_stored_value_without_ids_leaves_aligned_ids() {
    let option = json!({ "dataset": { "source": [["M", "V"], ["a", 1]] }, "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }] });
    let mut deck = forged_deck("chart", json!({ "option": option }));
    assert_that!(forged_stored(&deck, "option")["dataset"]["ids"].is_null()).is_true();
    let patch = json!({ "title": { "text": "T" } });
    apply(&mut deck, Op::SetChartOption { target: t1(), param: "option".into(), patch })
        .unwrap_or_else(|e| panic!("{e}"));
    assert_aligned(&forged_stored(&deck, "option"), 2, 2);
}

#[test]
fn set_table_option_on_a_stored_value_without_ids_leaves_aligned_ids() {
    let mut deck = forged_deck("table", json!({ "data": data() }));
    assert_that!(forged_stored(&deck, "data")["dataset"]["ids"].is_null()).is_true();
    let patch = json!({ "striped": true });
    apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch })
        .unwrap_or_else(|e| panic!("{e}"));
    assert_aligned(&forged_stored(&deck, "data"), 2, 2);
}
