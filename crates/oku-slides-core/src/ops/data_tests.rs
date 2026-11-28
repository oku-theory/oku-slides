use std::sync::Arc;

use oku_slides_data::{ColumnType, DataError, DataErrorKind, DataId, NewTable, TableOp};
use oku_slides_i18n::{FluentValue, Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::error::{DataFaultKind, OpErrorKind, TransactionError};
use crate::model::{Deck, Element};
use crate::ops::{Op, SlotRef, Target, apply_all, bind_choices};
use crate::test_registry::{registry, with_list};
use crate::widget::{HostType, WidgetRegistry};

fn did(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap()
}

/// Months and sales: columns `c1` (Month) and `c2` (Sales), rows `r1` and `r2`.
fn sales() -> NewTable {
    serde_json::from_value(json!({
        "generation": "g1",
        "columns": [{ "id": "c1", "name": "Month" }, { "id": "c2", "name": "sales" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 10] }, { "id": "r2", "cells": ["Feb", 12] }]
    }))
    .unwrap()
}

/// Only a Month column `c1`.
fn months() -> NewTable {
    serde_json::from_value(json!({
        "generation": "g2",
        "columns": [{ "id": "c1", "name": "Month" }],
        "rows": [{ "id": "r1", "cells": ["Jan"] }]
    }))
    .unwrap()
}

fn add(id: &str, name: &str, table: NewTable) -> Op {
    Op::AddData { id: Some(did(id)), name: name.to_owned(), label: None, table: Some(table), sql: None }
}

fn deck() -> Deck {
    Deck::from_json(r#"{"slides":[{"id":"s","kind":"essentiel/content"}]}"#).unwrap()
}

/// A deck with the data `d1` (Sales) and `d2` (Months).
fn deck_with_data() -> Deck {
    let mut deck = deck();
    apply_all(&mut deck, vec![add("d1", "sales", sales()), add("d2", "months", months())], &registry()).unwrap();
    deck
}

fn blocks() -> SlotRef {
    SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() }
}

fn target(element: &str) -> Target {
    Target { slide: "s".into(), element: Some(element.into()) }
}

fn element(id: &str, kind: &str, param: &str, option: &Value) -> Element {
    serde_json::from_value(json!({ "id": id, "kind": kind, "params": { param: option } })).unwrap()
}

fn chart_ref(data: &str) -> Value {
    json!({ "data": data, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] })
}

fn add_chart(id: &str, option: &Value) -> Op {
    Op::AddElement {
        slot: blocks(),
        index: None,
        defaults: false,
        element: element(id, "base/chart", "option", option),
    }
}

fn bind(element: &str, data: &str, columns: Option<Vec<&str>>) -> Op {
    Op::BindData {
        target: target(element),
        param: "option".into(),
        data: did(data),
        columns: columns.map(|c| c.into_iter().map(str::to_owned).collect()),
    }
}

fn run(deck: &mut Deck, ops: Vec<Op>) -> Result<(), TransactionError> {
    run_in(deck, ops, &registry())
}

fn run_in(deck: &mut Deck, ops: Vec<Op>, registry: &WidgetRegistry) -> Result<(), TransactionError> {
    apply_all(deck, ops, registry)
}

fn refused(deck: &mut Deck, ops: Vec<Op>) -> TransactionError {
    let before = deck.clone();
    let err = run(deck, ops).unwrap_err();
    assert_that!(*deck).is_equal_to(before);
    err
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn q(s: &str) -> FluentValue<'static> {
    FluentValue::from(s.to_owned())
}

fn option_of(deck: &Deck, id: &str) -> Value {
    deck.slides[0].locate(id).unwrap().element.params.value("option").unwrap().clone()
}

#[test]
fn add_data_taken_id_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![add("d1", "other", sales())]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::IdTaken));
}

#[test]
fn add_data_taken_name_case_insensitive_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![add("d3", "SALES", sales())]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::NameTaken));
    assert_that!(err.op_error().localize(&fr()))
        .is_equal_to(fr().tr_with("op-data-name-taken", &[("name", q("SALES"))]));
}

#[test]
fn add_data_over_deck_bounds_is_refused() {
    let mut deck = deck();
    let ops: Vec<Op> =
        (0..=oku_slides_data::limits::MAX_DATA).map(|i| add(&format!("d{i}"), &format!("n{i}"), months())).collect();
    let err = apply_all(&mut deck, ops, &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataDeckFull);
    assert_that!(err.index()).is_equal_to(oku_slides_data::limits::MAX_DATA);
    assert_that!(deck.data.len()).is_equal_to(0);
}

#[test]
fn edit_data_missing_data_is_refused() {
    let mut deck = deck_with_data();
    let change = TableOp::RemoveRow { row: "r1".into() };
    let err = refused(&mut deck, vec![Op::EditData { data: did("nope"), change }]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Missing));
}

#[test]
fn edit_data_clears_inline_flag() {
    let mut deck = deck_with_data();
    Arc::make_mut(deck.data.get_mut(&did("d1")).unwrap()).inline = true;
    let change =
        TableOp::SetCell { row: "r1".into(), column: "c2".into(), value: oku_slides_data::DataCell::Number(11.0) };
    run(&mut deck, vec![Op::EditData { data: did("d1"), change }]).unwrap();
    assert_that!(deck.data[&did("d1")].inline).is_false();
}

#[test]
fn remove_projected_column_is_refused_and_lists_views() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g1", &chart_ref("d1")), bind("g1", "d1", Some(vec!["c1", "c2"]))]).unwrap();
    run(&mut deck, vec![add_chart("g2", &chart_ref("d1")), bind("g2", "d1", Some(vec!["c2"]))]).unwrap();
    let change = TableOp::RemoveColumn { column: "c2".into() };
    let err = refused(&mut deck, vec![Op::EditData { data: did("d1"), change }]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::ColumnInUse));
    let args = [("column", q("c2")), ("views", q("g1, g2"))];
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-column-in-use", &args));
}

#[test]
fn bind_data_unknown_column_is_refused() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1"))]).unwrap();
    let err = refused(&mut deck, vec![bind("g", "d1", Some(vec!["zz"]))]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::NoColumn));
}

#[rstest]
#[case::empty(vec![])]
#[case::repeated(vec!["c1", "c1"])]
#[case::invalid(vec!["c 1"])]
fn bind_data_bad_columns_is_refused(#[case] columns: Vec<&str>) {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1"))]).unwrap();
    let err = refused(&mut deck, vec![bind("g", "d1", Some(columns))]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::ColumnsInvalid));
}

#[test]
fn bind_data_missing_data_is_refused() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1"))]).unwrap();
    let err = refused(&mut deck, vec![bind("g", "nope", None)]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Missing));
}

#[test]
fn bind_data_to_data_without_used_column_is_refused() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1"))]).unwrap();
    let err = refused(&mut deck, vec![bind("g", "d2", None)]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::ColumnMissing));
    let args = [("column", q("c2")), ("data", q("months"))];
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-column-missing", &args));
}

/// → a query data's columns are its result's, which the session checks: the core takes them.
#[test]
fn bind_data_to_query_data_leaves_columns_to_the_session() {
    let mut deck = deck_with_data();
    let query = Op::AddData {
        id: Some(did("q")),
        name: "top".into(),
        label: None,
        table: None,
        sql: Some("SELECT * FROM sales".into()),
    };
    run(&mut deck, vec![query, add_chart("g", &chart_ref("d1"))]).unwrap();
    run(&mut deck, vec![bind("g", "q", Some(vec!["zz"]))]).unwrap();
    assert_that!(option_of(&deck, "g")["data"]).is_equal_to(json!("q"));
    assert_that!(option_of(&deck, "g")["columns"]).is_equal_to(json!(["zz"]));
}

#[test]
fn bind_data_on_list_param_is_refused() {
    let registry = with_list();
    let mut deck = Deck::from_json(
        r#"{"slides":[{"id":"s","kind":"essentiel/blank","params":{"elements":[
            {"id":"f","kind":"test-el/liste","frame":{"x":0,"y":0,"w":1,"h":1},"params":{"etapes":[]}}]}}]}"#,
    )
    .unwrap();
    run_in(&mut deck, vec![add("d1", "sales", sales())], &registry).unwrap();
    let op = Op::BindData { target: target("f"), param: "etapes".into(), data: did("d1"), columns: None };
    let err = run_in(&mut deck, vec![op], &registry).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::NotView));
}

#[test]
fn bind_data_second_user_clears_inline() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g1", &chart_ref("d1")), add_chart("g2", &chart_ref("d2"))]).unwrap();
    Arc::make_mut(deck.data.get_mut(&did("d1")).unwrap()).inline = true;
    run(&mut deck, vec![bind("g2", "d1", Some(vec!["c1", "c2"]))]).unwrap();
    assert_that!(deck.data[&did("d1")].inline).is_false();
    assert_that!(option_of(&deck, "g2")["data"]).is_equal_to(json!("d1"));
}

#[test]
fn option_with_ref_and_dataset_is_refused() {
    let mut deck = deck_with_data();
    let mut option = chart_ref("d1");
    option["dataset"] = json!({ "source": [["Month", "sales"], ["Jan", 1]] });
    let err = refused(&mut deck, vec![add_chart("g", &option)]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.op_error().localize(&fr())).contains(fr().tr("chart-ref-and-inline").as_str());
}

#[test]
fn option_with_null_data_is_refused() {
    let mut deck = deck_with_data();
    let mut option = chart_ref("d1");
    option["data"] = Value::Null;
    let err = refused(&mut deck, vec![add_chart("g", &option)]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::InvalidChart);
    assert_that!(err.op_error().localize(&fr())).contains(fr().tr("chart-ref-null").as_str());
}

#[test]
fn option_with_ref_to_absent_data_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![add_chart("g", &chart_ref("nope"))]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Missing));
}

#[rstest]
#[case::full_set(Op::SetParam { target: target("g"), param: "option".into(), value: chart_ref("d2") })]
#[case::merge_patch(Op::SetChartOption { target: target("g"), param: "option".into(), patch: json!({ "data": "d2" }) })]
fn option_edit_changing_data_ref_is_refused(#[case] op: Op) {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1"))]).unwrap();
    let err = refused(&mut deck, vec![op]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Rebind));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-rebind", &[("subject", q("g"))]));
}

#[test]
fn option_edit_without_ref_keeps_the_ref() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1")), bind("g", "d1", Some(vec!["c1", "c2"]))]).unwrap();
    let value = json!({ "xAxis": { "type": "category" }, "series": [{ "type": "line" }] });
    run(&mut deck, vec![Op::SetParam { target: target("g"), param: "option".into(), value }]).unwrap();
    let option = option_of(&deck, "g");
    assert_that!(option["data"]).is_equal_to(json!("d1"));
    assert_that!(option["columns"]).is_equal_to(json!(["c1", "c2"]));
    assert_that!(option["series"][0]["type"]).is_equal_to(json!("line"));
}

fn remove(data: &str) -> Op {
    Op::RemoveData { data: did(data) }
}

fn rename(data: &str, name: &str) -> Op {
    Op::RenameData { data: did(data), name: name.to_owned() }
}

fn retype(data: &str, column: &str, ty: ColumnType) -> Op {
    Op::SetColumnType { data: did(data), column: column.to_owned(), ty }
}

/// What happens to an op on data `d1` once it was cut on reading.
#[derive(Debug, Clone, Copy)]
enum OnTruncated {
    /// Refused with `op-data-truncated`.
    Refused,
    /// Applied; `d1` is no longer truncated (restored or gone).
    Accepted,
}

#[rstest]
#[case::edit(Op::EditData { data: did("d1"), change: TableOp::RemoveRow { row: "r1".into() } }, OnTruncated::Refused)]
#[case::rename(rename("d1", "ventes"), OnTruncated::Refused)]
#[case::retype(retype("d1", "c2", ColumnType::Text), OnTruncated::Refused)]
#[case::replace(Op::EditData { data: did("d1"), change: TableOp::Replace { table: months() } }, OnTruncated::Accepted)]
#[case::remove(remove("d1"), OnTruncated::Accepted)]
fn op_on_truncated_data(#[case] op: Op, #[case] outcome: OnTruncated) {
    let mut deck = deck_with_data();
    deck.truncated.insert(did("d1"));
    let result = run(&mut deck, vec![op]);
    match outcome {
        OnTruncated::Refused => {
            let err = result.unwrap_err();
            assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Truncated));
            assert_that!(err.op_error().localize(&fr()))
                .is_equal_to(fr().tr_with("op-data-truncated", &[("data", q("d1"))]));
        }
        OnTruncated::Accepted => {
            assert_that!(result).is_ok();
            assert_that!(deck.truncated.contains(&did("d1"))).is_false();
        }
    }
}

#[test]
fn bind_data_to_truncated_data_is_refused() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1"))]).unwrap();
    deck.truncated.insert(did("d2"));
    let err = refused(&mut deck, vec![bind("g", "d2", Some(vec!["c1"]))]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Truncated));
}

#[test]
fn table_option_with_ref_only_normalizes() {
    let mut deck = deck_with_data();
    let table = element("t", "base/table", "data", &json!({ "data": "d1", "columns": ["c2"] }));
    run(&mut deck, vec![Op::AddElement { slot: blocks(), index: None, defaults: false, element: table }]).unwrap();
    let value = deck.slides[0].locate("t").unwrap().element.params.value("data").unwrap().clone();
    assert_that!(value["data"]).is_equal_to(json!("d1"));
    assert_that!(value["columns"]).is_equal_to(json!(["c2"]));
    assert_that!(value.get("dataset")).is_none();
}

#[test]
fn batch_failing_after_add_data_leaves_no_data() {
    let mut deck = deck();
    let err = apply_all(&mut deck, vec![add("d1", "sales", sales()), add_chart("g", &chart_ref("nope"))], &registry())
        .unwrap_err();
    assert_that!(err.index()).is_equal_to(1);
    assert_that!(deck.data.is_empty()).is_true();
}

#[test]
fn data_users_finds_nested_elements_in_order() {
    let mut deck = deck_with_data();
    let stack = serde_json::from_value(json!({ "id": "st", "kind": "base/stack", "params": { "children": [
        { "id": "inner", "kind": "base/chart", "params": { "option": chart_ref("d1") } }
    ] } }))
    .unwrap();
    run(
        &mut deck,
        vec![
            add_chart("first", &chart_ref("d1")),
            Op::AddElement { slot: blocks(), index: None, defaults: false, element: stack },
            add_chart("other", &chart_ref("d2")),
            add_chart("last", &chart_ref("d1")),
            bind("last", "d1", Some(vec!["c2"])),
        ],
    )
    .unwrap();
    let users = deck.data_users(&did("d1"));
    let found: Vec<(&str, &str, Option<Vec<String>>)> =
        users.iter().map(|v| (v.slide.as_str(), v.element.as_str(), v.columns.clone())).collect();
    assert_that!(found).is_equal_to(vec![
        ("s", "first", None),
        ("s", "inner", None),
        ("s", "last", Some(vec!["c2".to_owned()])),
    ]);
}

#[test]
fn bind_data_rebinding_the_only_view_keeps_inline() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g1", &chart_ref("d1"))]).unwrap();
    Arc::make_mut(deck.data.get_mut(&did("d1")).unwrap()).inline = true;
    run(&mut deck, vec![bind("g1", "d1", Some(vec!["c1"]))]).unwrap();
    assert_that!(deck.data[&did("d1")].inline).is_true();
    run(&mut deck, vec![add_chart("g2", &chart_ref("d2")), bind("g2", "d1", Some(vec!["c1"]))]).unwrap();
    assert_that!(deck.data[&did("d1")].inline).is_false();
}

/// Finding (i): a chart reading `Month` and `sales` rebinds to a data that lacks its unused
/// third column.
#[test]
fn bind_data_without_columns_requires_only_the_columns_the_chart_uses() {
    let wide: NewTable = serde_json::from_value(json!({
        "generation": "g3",
        "columns": [{ "id": "c1", "name": "Month" }, { "id": "c2", "name": "sales" }, { "id": "c3", "name": "note" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 10, "x"] }]
    }))
    .unwrap();
    let mut deck = deck();
    let one_series = json!({ "data": "w", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] });
    run(&mut deck, vec![add("w", "wide", wide), add("d1", "sales", sales()), add_chart("g", &one_series)]).unwrap();
    run(&mut deck, vec![bind("g", "d1", None)]).unwrap();
    let data_users = deck.data_users(&did("d1"));
    assert_that!(data_users.len()).is_equal_to(1);
}

/// A table reads every column: rebinding it needs them all.
#[test]
fn bind_data_without_columns_on_a_table_requires_every_column() {
    let mut deck = deck_with_data();
    let table = Op::AddElement {
        slot: blocks(),
        index: None,
        defaults: false,
        element: element("t", "base/table", "data", &json!({ "data": "d1" })),
    };
    run(&mut deck, vec![table]).unwrap();
    let err = refused(
        &mut deck,
        vec![Op::BindData { target: target("t"), param: "data".into(), data: did("d2"), columns: None }],
    );
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::ColumnMissing));
}

/// Month, sales and note: columns `c1`, `c2` and `c3`.
fn wide() -> NewTable {
    serde_json::from_value(json!({
        "generation": "g3",
        "columns": [{ "id": "c1", "name": "Month" }, { "id": "c2", "name": "sales" }, { "id": "c3", "name": "note" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 10, "x"] }]
    }))
    .unwrap()
}

/// [`deck_with_data`] with `d3` (`wide`) and the chart `g` over `d1`.
fn deck_with_choices() -> Deck {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add("d3", "wide", wide()), add_chart("g", &chart_ref("d1"))]).unwrap();
    deck
}

fn choices(deck: &Deck, host: HostType, option: &Value) -> Vec<String> {
    bind_choices(deck, host, option).into_iter().map(|id| id.as_str().to_owned()).collect()
}

#[test]
fn bind_choices_of_a_chart_list_data_holding_its_used_columns() {
    let deck = deck_with_choices();
    assert_that!(choices(&deck, HostType::Chart, &chart_ref("d1"))).is_equal_to(vec!["d1".to_owned(), "d3".to_owned()]);
}

#[test]
fn bind_choices_of_a_projection_list_data_holding_its_columns() {
    let deck = deck_with_choices();
    let option = json!({ "data": "d1", "columns": ["c1"], "series": [{ "type": "bar" }] });
    let all = vec!["d1".to_owned(), "d2".to_owned(), "d3".to_owned()];
    assert_that!(choices(&deck, HostType::Chart, &option)).is_equal_to(all);
}

#[test]
fn bind_choices_of_a_table_need_every_column() {
    let deck = deck_with_choices();
    assert_that!(choices(&deck, HostType::Table, &json!({ "data": "d2" }))).is_equal_to(vec![
        "d1".to_owned(),
        "d2".to_owned(),
        "d3".to_owned(),
    ]);
    assert_that!(choices(&deck, HostType::Table, &json!({ "data": "d3" }))).is_equal_to(vec!["d3".to_owned()]);
}

#[test]
fn bind_choices_leave_out_truncated_data_and_offer_inline_data() {
    let mut deck = deck_with_choices();
    deck.truncated.insert(did("d3"));
    Arc::make_mut(deck.data.get_mut(&did("d2")).unwrap()).inline = true;
    Arc::make_mut(deck.data.get_mut(&did("d1")).unwrap()).inline = true;
    let option = json!({ "data": "d1", "columns": ["c1"] });
    assert_that!(choices(&deck, HostType::Chart, &option)).is_equal_to(vec!["d1".to_owned(), "d2".to_owned()]);
}

/// Every choice binds without a projection.
#[test]
fn bind_choices_each_bind() {
    let deck = deck_with_choices();
    for id in choices(&deck, HostType::Chart, &chart_ref("d1")) {
        let mut copy = deck.clone();
        assert_that!(run(&mut copy, vec![bind("g", &id, None)])).named(&id).is_ok();
    }
}

/// `in_use` args: the views listed, and how many more.
fn in_use(views: &str, more: usize) -> [(&'static str, FluentValue<'static>); 2] {
    [("views", q(views)), ("more", more.into())]
}

#[test]
fn remove_data_in_use_is_refused_and_lists_views() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g1", &chart_ref("d1")), add_chart("g2", &chart_ref("d1"))]).unwrap();
    let err = refused(&mut deck, vec![remove("d1")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::InUse));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-in-use", &in_use("g1, g2", 0)));
}

#[rstest]
#[case::ten(10, 0)]
#[case::eleven(11, 1)]
#[case::twelve(12, 2)]
fn remove_data_in_use_lists_10_then_more(#[case] views: usize, #[case] more: usize) {
    let mut deck = deck_with_data();
    let charts = (0..views).map(|i| add_chart(&format!("g{i:02}"), &chart_ref("d1"))).collect();
    run(&mut deck, charts).unwrap();
    let err = refused(&mut deck, vec![remove("d1")]);
    let listed: Vec<String> = (0..10).map(|i| format!("g{i:02}")).collect();
    let expected = fr().tr_with("op-data-in-use", &in_use(&listed.join(", "), more));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(expected);
}

#[test]
fn remove_data_after_its_view_is_removed_succeeds() {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_chart("g", &chart_ref("d1"))]).unwrap();
    let remove_view = Op::RemoveElement { slide: "s".into(), element: "g".into() };
    run(&mut deck, vec![remove_view, remove("d1")]).unwrap();
    assert_that!(deck.data.contains_key(&did("d1"))).is_false();
    assert_that!(deck.data.contains_key(&did("d2"))).is_true();
}

#[test]
fn remove_data_missing_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![remove("nope")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Missing));
}

/// A deck past its data bounds (a merge can leave one) still sheds a data.
#[test]
fn remove_data_succeeds_over_deck_bounds() {
    let mut deck = deck_with_data();
    let extra = Arc::clone(&deck.data[&did("d2")]);
    for i in 0..oku_slides_data::limits::MAX_DATA {
        deck.data.insert(did(&format!("x{i}")), Arc::clone(&extra));
    }
    assert_that!(crate::data_bounds::check_deck_bounds(&deck)).is_err();
    run(&mut deck, vec![remove("d1")]).unwrap();
    assert_that!(deck.data.contains_key(&did("d1"))).is_false();
}

#[test]
fn rename_data_sets_the_name_and_clears_inline() {
    let mut deck = deck_with_data();
    Arc::make_mut(deck.data.get_mut(&did("d1")).unwrap()).inline = true;
    run(&mut deck, vec![rename("d1", "ventes")]).unwrap();
    assert_that!(deck.data[&did("d1")].name.as_str()).is_equal_to("ventes");
    assert_that!(deck.data[&did("d1")].inline).is_false();
}

#[test]
fn rename_to_taken_name_case_insensitive_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![rename("d2", "SALES")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::NameTaken));
    assert_that!(err.op_error().localize(&fr()))
        .is_equal_to(fr().tr_with("op-data-name-taken", &[("name", q("SALES"))]));
    // Its own name, case changed, is not taken.
    run(&mut deck, vec![rename("d1", "sales_2"), rename("d1", "sales")]).unwrap();
}

#[test]
fn rename_invalid_name_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![rename("d1", "1 bad")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::InvalidName));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(DataError::invalid_name("1 bad").localize(&fr()));
}

#[test]
fn set_column_type_converts_the_column_and_clears_inline() {
    let mut deck = deck_with_data();
    Arc::make_mut(deck.data.get_mut(&did("d1")).unwrap()).inline = true;
    run(&mut deck, vec![retype("d1", "c2", ColumnType::Integer)]).unwrap();
    let data = &deck.data[&did("d1")];
    assert_that!(data.table().unwrap().columns[1].ty).is_equal_to(ColumnType::Integer);
    assert_that!(data.inline).is_false();
}

#[test]
fn set_column_type_refusal_names_rows_fr() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![retype("d1", "c1", ColumnType::Number)]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::ColumnType));
    let mut table = deck.data[&did("d1")].table().unwrap().clone();
    let expected = table.set_column_type("c1", ColumnType::Number).unwrap_err().localize(&fr());
    let text = err.op_error().localize(&fr());
    assert_that!(text).is_equal_to(expected);
    assert_that!(text).contains("r1");
    assert_that!(text).contains("r2");
}

#[path = "data_query_tests.rs"]
mod data_query_tests;
