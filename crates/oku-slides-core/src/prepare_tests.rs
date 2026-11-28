use std::collections::BTreeMap;

use oku_slides_chart::ChartLabels;
use oku_slides_data::limits::{BIND_DATA_WEIGHT, MAX_COLUMNS, REMOVE_COLUMN_WEIGHT};
use oku_slides_data::{DataCell, DataErrorKind, DataId, NewTable, TableOp};
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::error::{DataFaultKind, OpErrorKind};
use crate::model::{Element, Slide};
use crate::ops::{SlotRef, Target, apply_all};
use crate::params::ParamValue;
use crate::test_registry::registry;

/// A counter drawing `d1`, `d2`… and skipping taken ids.
fn counter() -> impl FnMut(&dyn Fn(&str) -> bool) -> String {
    let mut n = 0;
    move |taken| loop {
        n += 1;
        let id = format!("d{n}");
        if !taken(&id) {
            return id;
        }
    }
}

fn did(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap()
}

fn prepare(deck: &Deck, ops: Vec<Op>, origin: Origin) -> Result<Prepared, OpError> {
    let registry = registry();
    let labels = ChartLabels::default();
    let mut ids = counter();
    let mut ctx = PrepareCtx { deck, registry: &registry, labels: &labels, ids: &mut ids, origin };
    prepare_ops(ops, &mut ctx)
}

fn deck() -> Deck {
    Deck::from_json(r#"{"slides":[{"id":"s","kind":"essentiel/content"}]}"#).unwrap()
}

/// A table of `rows` rows of `width` number cells, ids left out.
fn table(rows: usize, width: usize) -> NewTable {
    let columns: Vec<_> = (0..width).map(|i| json!({ "name": format!("C{i}") })).collect();
    let rows: Vec<_> = (0..rows).map(|_| json!({ "cells": vec![1; width] })).collect();
    serde_json::from_value(json!({ "columns": columns, "rows": rows })).unwrap()
}

fn add(id: Option<&str>, name: &str, table: NewTable) -> Op {
    Op::AddData { id: id.map(did), name: name.to_owned(), label: None, table: Some(table), sql: None }
}

fn added_id(op: &Op) -> String {
    match op {
        Op::AddData { id: Some(id), .. } => id.as_str().to_owned(),
        other => panic!("not an add_data with an id: {other:?}"),
    }
}

#[test]
fn agent_add_data_without_ids_parses_then_prepare_draws() {
    let op: Op = serde_json::from_value(json!({
        "op": "add_data",
        "name": "sales",
        "table": { "columns": [{ "name": "Month" }, { "name": "sales" }], "rows": [{ "cells": ["Jan", 10] }] }
    }))
    .unwrap();
    let prepared = prepare(&deck(), vec![op], Origin::External).unwrap();
    let Op::AddData { id, table: Some(table), .. } = &prepared.ops[0] else { panic!("add_data expected") };
    assert_that!(id.is_some()).is_true();
    assert_that!(table.generation.is_some()).is_true();
    assert_that!(table.columns.iter().all(|c| c.id.is_some())).is_true();
    assert_that!(table.rows.iter().all(|r| r.id.is_some())).is_true();
    let mut deck = deck();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    assert_that!(deck.data.len()).is_equal_to(1);
}

#[test]
fn prepare_fills_insert_row_and_column_ids() {
    let mut deck = deck();
    let prepared = prepare(&deck, vec![add(None, "sales", table(1, 1))], Origin::External).unwrap();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    let data = did("d1");
    let ops = vec![
        Op::EditData {
            data: data.clone(),
            change: TableOp::InsertRow { id: None, after: None, cells: BTreeMap::default() },
        },
        Op::EditData {
            data: data.clone(),
            change: TableOp::InsertColumn { id: None, after: None, name: "New".into(), cells: BTreeMap::default() },
        },
    ];
    let prepared = prepare(&deck, ops, Origin::External).unwrap();
    let filled = prepared.ops.iter().all(|op| {
        matches!(
            op,
            Op::EditData {
                change: TableOp::InsertRow { id: Some(_), .. } | TableOp::InsertColumn { id: Some(_), .. },
                ..
            }
        )
    });
    assert_that!(filled).is_true();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    assert_that!(deck.data[&data].table().unwrap().rows.len()).is_equal_to(2);
    assert_that!(deck.data[&data].table().unwrap().columns.len()).is_equal_to(2);
}

#[test]
fn prepare_draws_ids_not_taken_by_deck_or_batch() {
    let mut deck = deck();
    let prepared = prepare(&deck, vec![add(Some("d1"), "old", table(1, 1))], Origin::External).unwrap();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    let ops = vec![add(Some("d2"), "given", table(1, 1)), add(None, "drawn", table(1, 1))];
    let prepared = prepare(&deck, ops, Origin::External).unwrap();
    let drawn = added_id(&prepared.ops[1]);
    assert_that!(["d1", "d2"].contains(&drawn.as_str())).is_false();
}

#[test]
fn prepare_two_add_data_in_one_batch_get_distinct_ids() {
    let ops = vec![add(None, "a", table(1, 1)), add(None, "b", table(1, 1))];
    let prepared = prepare(&deck(), ops, Origin::External).unwrap();
    assert_that!(added_id(&prepared.ops[0])).is_not_equal_to(added_id(&prepared.ops[1]));
}

#[test]
fn prepare_drawn_id_avoids_id_given_later_in_batch() {
    let ops = vec![add(None, "a", table(1, 1)), add(Some("d1"), "b", table(1, 1))];
    let prepared = prepare(&deck(), ops, Origin::External).unwrap();
    assert_that!(added_id(&prepared.ops[0])).is_equal_to("d2".to_owned());
}

#[test]
fn prepare_sees_earlier_ops_of_the_batch() {
    let insert = TableOp::InsertRow { id: None, after: None, cells: BTreeMap::default() };
    let ops = vec![add(Some("d1"), "a", table(1, 1)), Op::EditData { data: did("d1"), change: insert }];
    let prepared = prepare(&deck(), ops, Origin::External).unwrap();
    let filled = matches!(&prepared.ops[1], Op::EditData { change: TableOp::InsertRow { id: Some(_), .. }, .. });
    assert_that!(filled).is_true();
    let mut deck = deck();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    assert_that!(deck.data[&did("d1")].table().unwrap().rows.len()).is_equal_to(2);
}

#[test]
fn apply_without_prepare_refuses_missing_id() {
    let mut deck = deck();
    let err = apply_all(&mut deck, vec![add(None, "a", table(1, 1))], &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::IdMissing));
}

#[rstest]
#[case::at_cap(MAX_BATCH_CELLS, Origin::External, true)]
#[case::over_cap(MAX_BATCH_CELLS + 1, Origin::External, false)]
#[case::host_over_cap(MAX_BATCH_CELLS + 1, Origin::Host, true)]
fn batch_cells_cap(#[case] cells: usize, #[case] origin: Origin, #[case] accepted: bool) {
    // Two ops of one column each, sharing the cells.
    let ops = vec![add(None, "a", table(cells / 2, 1)), add(None, "b", table(cells - cells / 2, 1))];
    let result = prepare(&deck(), ops, origin);
    if accepted {
        assert_that!(result).is_ok();
    } else {
        let err = result.unwrap_err();
        assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::BatchTooLarge));
    }
}

#[test]
fn external_new_table_with_base_is_refused() {
    let mut table = table(0, 1);
    table.base = Some(
        serde_json::from_value(
            json!({ "id": format!("sha256:{}", "a".repeat(64)), "bytes": 1, "rows": 0, "columns": [] }),
        )
        .unwrap(),
    );
    let err = prepare(&deck(), vec![add(None, "a", table)], Origin::External).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Internal));
    let fr = Localizer::new(Some("fr"));
    assert_that!(fr.has("op-data-internal")).is_true();
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr("op-data-internal"));
}

/// A deck holding data `x`: generation `d1`, column `d2`, row `d3`, drawn by [`counter`].
fn deck_with_x() -> Deck {
    let mut deck = deck();
    let prepared = prepare(&deck, vec![add(Some("x"), "x", table(1, 1))], Origin::Host).unwrap();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    deck
}

fn edit_x(change: TableOp) -> Op {
    Op::EditData { data: did("x"), change }
}

fn insert_column(id: Option<&str>) -> TableOp {
    TableOp::InsertColumn { id: id.map(str::to_owned), after: None, name: "New".into(), cells: BTreeMap::default() }
}

#[test]
fn prepare_drawn_row_id_avoids_row_id_given_later_in_batch() {
    let insert =
        |id: Option<&str>| TableOp::InsertRow { id: id.map(str::to_owned), after: None, cells: BTreeMap::default() };
    let ops = vec![edit_x(insert(None)), edit_x(insert(Some("d1")))];
    let prepared = prepare(&deck_with_x(), ops, Origin::External).unwrap();
    assert_that!(prepared.ops[0]).is_equal_to(edit_x(insert(Some("d2"))));
    let mut deck = deck_with_x();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    assert_that!(deck.data[&did("x")].table().unwrap().rows.len()).is_equal_to(3);
}

#[test]
fn prepare_drawn_column_id_avoids_column_id_given_later_in_batch() {
    // `d1` is given later, `d2` is the existing column: the first draw is `d3`.
    let ops = vec![edit_x(insert_column(None)), edit_x(insert_column(Some("d1")))];
    let prepared = prepare(&deck_with_x(), ops, Origin::External).unwrap();
    assert_that!(prepared.ops[0]).is_equal_to(edit_x(insert_column(Some("d3"))));
    let mut deck = deck_with_x();
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    assert_that!(deck.data[&did("x")].table().unwrap().columns.len()).is_equal_to(3);
}

#[rstest]
#[case::at_cap(0, true)]
#[case::over_cap(1, false)]
fn remove_column_counts_its_weight_toward_the_cap(#[case] extra: usize, #[case] accepted: bool) {
    // MAX_COLUMNS removes fill the cap; one more cell is past it.
    assert_that!(REMOVE_COLUMN_WEIGHT * MAX_COLUMNS).is_equal_to(MAX_BATCH_CELLS);
    let remove = edit_x(TableOp::RemoveColumn { column: "c".into() });
    let mut ops = vec![remove; MAX_COLUMNS];
    ops.extend(
        (0..extra)
            .map(|_| edit_x(TableOp::SetCell { row: "r".into(), column: "c".into(), value: DataCell::Number(1.0) })),
    );
    let result = check_external(&ops);
    if accepted {
        assert_that!(result).is_ok();
    } else {
        let err = result.unwrap_err();
        assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::BatchTooLarge));
    }
}

#[rstest]
#[case::at_cap(0, true)]
#[case::over_cap(1, false)]
fn bind_data_counts_its_weight_toward_the_cap(#[case] extra: usize, #[case] accepted: bool) {
    let binds = MAX_BATCH_CELLS / BIND_DATA_WEIGHT;
    assert_that!(binds).is_equal_to(1_000);
    let target = crate::ops::Target { slide: "s".into(), element: Some("g".into()) };
    let bind = Op::BindData { target, param: "option".into(), data: did("x"), columns: None };
    let mut ops = vec![bind; binds];
    ops.extend((0..extra).map(|_| edit_x(TableOp::RemoveRow { row: "r".into() })));
    let result = check_external(&ops);
    if accepted {
        assert_that!(result).is_ok();
    } else {
        let err = result.unwrap_err();
        assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::BatchTooLarge));
    }
}

#[rstest]
#[case::empty("", 1)]
#[case::under_a_kib(&"x".repeat(1023), 1)]
#[case::one_kib(&"x".repeat(1024), 2)]
#[case::max(&"x".repeat(16_384), 17)]
fn query_weight_counts_one_plus_its_kibs(#[case] sql: &str, #[case] weight: usize) {
    assert_that!(query_weight(sql)).is_equal_to(weight);
}

#[rstest]
#[case::at_cap(0, true)]
#[case::over_cap(1, false)]
fn query_ops_count_their_weight_toward_the_cap(#[case] extra: usize, #[case] accepted: bool) {
    // 15 KiB of SQL weighs 16: the cap holds that many queries, half added, half set.
    let sql = format!("SELECT * FROM sales{}", " ".repeat(15 * 1024 - 19));
    let count = MAX_BATCH_CELLS / query_weight(&sql);
    assert_that!(count * query_weight(&sql)).is_equal_to(MAX_BATCH_CELLS);
    let add = Op::AddData { id: None, name: "q".into(), label: None, table: None, sql: Some(sql.clone()) };
    let set = Op::SetQuery { data: did("x"), sql };
    let mut ops: Vec<Op> = (0..count).map(|i| if i % 2 == 0 { add.clone() } else { set.clone() }).collect();
    ops.extend((0..extra).map(|_| edit_x(TableOp::RemoveRow { row: "r".into() })));
    let result = check_external(&ops);
    if accepted {
        assert_that!(result).is_ok();
    } else {
        let err = result.unwrap_err();
        assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::BatchTooLarge));
    }
}

// ---- Inline view data -----------------------------------------------------------------

fn target(element: &str) -> Target {
    Target { slide: "s".into(), element: Some(element.into()) }
}

fn blocks() -> SlotRef {
    SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() }
}

fn inline_chart_option() -> serde_json::Value {
    json!({ "xAxis": { "type": "category", "data": ["1", "2"] }, "series": [{ "type": "bar", "data": [3, 5] }] })
}

fn dataset_chart(title: Option<&str>, values: [f64; 2]) -> serde_json::Value {
    let mut v = json!({
        "dataset": { "source": [["Month", "Sales", "Note"], ["Jan", values[0], "a"], ["Feb", values[1], "b"]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    });
    if let Some(title) = title {
        v["title"] = json!({ "text": title });
    }
    v
}

fn table_option(values: [i64; 2]) -> serde_json::Value {
    json!({ "dataset": { "source": [["Name", "Score"], ["Ann", values[0]], ["Bob", values[1]]] } })
}

fn elem(id: &str, kind: &str, option: &serde_json::Value) -> Element {
    let param = if kind == "base/table" { "data" } else { "option" };
    serde_json::from_value(json!({ "id": id, "kind": kind, "params": { param: option } })).unwrap()
}

fn add_elem(id: &str, kind: &str, option: &serde_json::Value) -> Op {
    Op::AddElement { slot: blocks(), index: None, defaults: false, element: elem(id, kind, option) }
}

/// Prepares with French labels and applies; returns the warnings.
fn run(deck: &mut Deck, ops: Vec<Op>) -> Vec<Issue> {
    let registry = registry();
    let labels = ChartLabels::new(&Localizer::new(Some("fr")));
    let mut ids = counter();
    let mut ctx = PrepareCtx { deck, registry: &registry, labels: &labels, ids: &mut ids, origin: Origin::External };
    let prepared = prepare_ops(ops, &mut ctx).unwrap();
    apply_all(deck, prepared.ops, &registry).unwrap();
    prepared.warnings
}

fn option_of(deck: &Deck, element: &str) -> serde_json::Value {
    let slide = &deck.slides[0];
    let e = slide.locate(element).unwrap().element;
    let Some(ParamValue::Value(v)) = e.params.get("option") else { panic!("no option on {element}") };
    v.clone()
}

fn bound(deck: &Deck, element: &str) -> String {
    option_of(deck, element)["data"].as_str().unwrap().to_owned()
}

fn puts(ops: &[Op]) -> Vec<&Op> {
    ops.iter().filter(|op| matches!(op, Op::PutViewData { .. })).collect()
}

/// A deck with chart `g` (inline, then lifted) and table `t`.
fn deck_with_views() -> Deck {
    let mut deck = deck();
    run(
        &mut deck,
        vec![
            add_elem("g", "base/chart", &dataset_chart(None, [1.0, 2.0])),
            add_elem("t", "base/table", &table_option([1, 2])),
        ],
    );
    deck
}

fn put_chart(values: [f64; 2]) -> Op {
    Op::SetParam { target: target("g"), param: "option".into(), value: dataset_chart(None, values) }
}

#[rstest]
#[case::set_param(Op::SetParam { target: target("g"), param: "option".into(), value: dataset_chart(None, [7.0, 8.0]) })]
#[case::set_chart_option(Op::SetChartOption { target: target("g"), param: "option".into(), patch: json!({ "dataset": { "source": [["M", "V"], ["a", 7]] } }) })]
#[case::set_table_option(Op::SetTableOption { target: target("t"), param: "data".into(), patch: json!({ "dataset": { "source": [["N", "S"], ["x", 7]] } }) })]
#[case::replace_element(Op::ReplaceElement { slide: "s".into(), element: elem("g", "base/chart", &inline_chart_option()) })]
#[case::add_chart(add_elem("g2", "base/chart", &inline_chart_option()))]
#[case::add_table(add_elem("t2", "base/table", &table_option([3, 4])))]
#[case::add_slide({
    let mut slide = Slide::new("s2", crate::kinds::essentiel::CONTENT);
    slide.params.set("blocks", ParamValue::Slot(vec![elem("g3", "base/chart", &inline_chart_option())]));
    Op::AddSlide { index: None, defaults: false, slide }
})]
#[case::nested_in_slot({
    let stack: Element = serde_json::from_value(json!({
        "id": "grp", "kind": crate::kinds::base::STACK,
        "params": { "children": [serde_json::to_value(elem("g4", "base/chart", &inline_chart_option())).unwrap()] }
    })).unwrap();
    Op::AddElement { slot: blocks(), index: None, defaults: false, element: stack }
})]
fn inline_data_in_each_carrying_op_becomes_put_view_data(#[case] op: Op) {
    let deck = deck_with_views();
    let prepared = prepare(&deck, vec![op], Origin::External).unwrap();
    assert_that!(puts(&prepared.ops).len()).is_equal_to(1);
    assert_that!(matches!(prepared.ops[0], Op::PutViewData { .. })).is_true();
    let carried = serde_json::to_string(&prepared.ops[1]).unwrap();
    assert_that!(carried.contains("\"dataset\"")).is_false();
    let mut applied = deck.clone();
    apply_all(&mut applied, prepared.ops, &registry()).unwrap();
}

#[test]
fn inline_echarts_series_becomes_data() {
    let mut deck = deck();
    run(&mut deck, vec![add_elem("g", "base/chart", &inline_chart_option())]);
    let option = option_of(&deck, "g");
    assert_that!(option.get("dataset")).is_none();
    assert_that!(option["series"][0].get("data")).is_none();
    let data = &deck.data[&did(&bound(&deck, "g"))];
    assert_that!(data.inline).is_true();
    let names: Vec<&str> = data.table().unwrap().columns.iter().map(|c| c.name.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["Catégorie", "Série 1"]);
    assert_that!(data.table().unwrap().rows.len()).is_equal_to(2);
}

#[test]
fn merge_patch_touching_dataset_becomes_put_view_data() {
    let deck = deck_with_views();
    let patch = json!({ "dataset": { "source": [["M", "V"], ["a", 7]] } });
    let op = Op::SetChartOption { target: target("g"), param: "option".into(), patch };
    let prepared = prepare(&deck, vec![op], Origin::External).unwrap();
    assert_that!(matches!(prepared.ops[1], Op::SetParam { .. })).is_true();
    let mut deck = deck;
    apply_all(&mut deck, prepared.ops, &registry()).unwrap();
    assert_that!(deck.data[&did(&bound(&deck, "g"))].table().unwrap().rows.len()).is_equal_to(1);
}

#[test]
fn merge_patch_columns_null_removes_projection() {
    let mut deck = deck_with_views();
    assert_that!(option_of(&deck, "g").get("columns")).is_some();
    let patch = json!({ "columns": null });
    deck = {
        let mut d = deck;
        run(&mut d, vec![Op::SetChartOption { target: target("g"), param: "option".into(), patch }]);
        d
    };
    assert_that!(option_of(&deck, "g").get("columns")).is_none();
    assert_that!(option_of(&deck, "g").get("data")).is_some();
}

#[test]
fn batch_of_two_inline_charts_named_ventes_gets_ventes_2() {
    let mut deck = deck();
    run(
        &mut deck,
        vec![
            add_elem("a", "base/chart", &dataset_chart(Some("Ventes"), [1.0, 2.0])),
            add_elem("b", "base/chart", &dataset_chart(Some("Ventes"), [3.0, 4.0])),
        ],
    );
    let a = &deck.data[&did(&bound(&deck, "a"))];
    let b = &deck.data[&did(&bound(&deck, "b"))];
    assert_that!(a.name.as_str()).is_equal_to("ventes");
    assert_that!(b.name.as_str()).is_equal_to("ventes_2");
    assert_that!(a.label.as_deref()).is_equal_to(Some("Ventes"));
}

#[test]
fn reset_inline_three_times_in_one_batch_keeps_one_data() {
    let mut deck = deck();
    run(
        &mut deck,
        vec![
            add_elem("g", "base/chart", &dataset_chart(None, [1.0, 2.0])),
            put_chart([3.0, 4.0]),
            put_chart([5.0, 6.0]),
        ],
    );
    assert_that!(deck.data.len()).is_equal_to(1);
    let data = deck.data.values().next().unwrap();
    assert_that!(
        data.table().unwrap().cell(data.table().unwrap().rows.get(0).unwrap(), &data.table().unwrap().columns[1].id)
    )
    .is_equal_to(oku_slides_data::CellRead::Overlay(&DataCell::Number(5.0)));
}

#[test]
fn reset_inline_three_times_keeps_one_data_and_its_id() {
    let mut deck = deck_with_views();
    let id = bound(&deck, "g");
    for v in [3.0, 5.0, 7.0] {
        run(&mut deck, vec![put_chart([v, v])]);
    }
    assert_that!(deck.data.len()).is_equal_to(2);
    assert_that!(bound(&deck, "g")).is_equal_to(id);
}

#[test]
fn reset_inline_on_shared_view_creates_new_data() {
    let mut deck = deck_with_views();
    let shared = bound(&deck, "g");
    run(
        &mut deck,
        vec![add_elem(
            "h",
            "base/chart",
            &json!({ "data": shared, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] }),
        )],
    );
    run(&mut deck, vec![put_chart([9.0, 9.0])]);
    assert_that!(bound(&deck, "g")).is_not_equal_to(shared.clone());
    assert_that!(bound(&deck, "h")).is_equal_to(shared);
    assert_that!(deck.data.len()).is_equal_to(3);
}

#[test]
fn reset_inline_after_cell_edit_creates_new_data() {
    let mut deck = deck_with_views();
    let id = bound(&deck, "g");
    let data = deck.data[&did(&id)].table().unwrap();
    let (row, column) = (data.rows.get(0).unwrap().to_owned(), data.columns[1].id.clone());
    let edit = Op::EditData { data: did(&id), change: TableOp::SetCell { row, column, value: DataCell::Number(4.0) } };
    run(&mut deck, vec![edit]);
    run(&mut deck, vec![put_chart([9.0, 9.0])]);
    assert_that!(bound(&deck, "g")).is_not_equal_to(id);
}

#[test]
fn replace_in_place_keeps_column_ids_by_name_and_row_ids_by_position() {
    let mut deck = deck_with_views();
    let id = did(&bound(&deck, "g"));
    let before = deck.data[&id].table().unwrap().clone();
    let renamed = json!({
        "dataset": { "source": [["Sales", "Month"], [1, "Jan"], [2, "Feb"], [3, "Mar"]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "encode": { "x": 1, "y": 0 } }]
    });
    run(&mut deck, vec![Op::SetParam { target: target("g"), param: "option".into(), value: renamed }]);
    let after = deck.data[&id].table().unwrap();
    let by_name = |t: &oku_slides_data::Table, n: &str| t.columns.iter().find(|c| c.name == n).unwrap().id.clone();
    assert_that!(by_name(after, "Month")).is_equal_to(by_name(&before, "Month"));
    assert_that!(by_name(after, "Sales")).is_equal_to(by_name(&before, "Sales"));
    assert_that!(after.rows.get(0)).is_equal_to(before.rows.get(0));
    assert_that!(after.rows.get(1)).is_equal_to(before.rows.get(1));
    assert_that!(after.generation).is_not_equal_to(before.generation.clone());
}

/// An op that makes an inline data a deck data of its own.
#[derive(Debug, Clone, Copy)]
enum Clears {
    EditData,
    SecondBind,
    Rename,
    SetColumnType,
}

#[rstest]
#[case::edit_data(Clears::EditData)]
#[case::second_bind(Clears::SecondBind)]
#[case::rename_data(Clears::Rename)]
#[case::set_column_type(Clears::SetColumnType)]
fn inline_cleared_by(#[case] by: Clears) {
    let mut deck = deck_with_views();
    let id = did(&bound(&deck, "g"));
    assert_that!(deck.data[&id].inline).is_true();
    let op = match by {
        Clears::SecondBind => {
            run(
                &mut deck,
                vec![add_elem(
                    "h",
                    "base/chart",
                    &json!({ "xAxis": { "type": "category", "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] }),
                )],
            );
            Op::BindData { target: target("h"), param: "option".into(), data: id.clone(), columns: None }
        }
        Clears::EditData => {
            let t = deck.data[&id].table().unwrap();
            let (row, column) = (t.rows.get(0).unwrap().to_owned(), t.columns[1].id.clone());
            Op::EditData { data: id.clone(), change: TableOp::SetCell { row, column, value: DataCell::Number(4.0) } }
        }
        Clears::Rename => Op::RenameData { data: id.clone(), name: "renamed".into() },
        Clears::SetColumnType => {
            let column = deck.data[&id].table().unwrap().columns[1].id.clone();
            Op::SetColumnType { data: id.clone(), column, ty: oku_slides_data::ColumnType::Text }
        }
    };
    run(&mut deck, vec![op]);
    assert_that!(deck.data[&id].inline).is_false();
}

#[test]
fn set_param_without_data_keeps_ref() {
    let mut deck = deck_with_views();
    let id = bound(&deck, "g");
    let value = json!({ "xAxis": { "type": "category" }, "series": [{ "type": "line" }] });
    run(&mut deck, vec![Op::SetParam { target: target("g"), param: "option".into(), value }]);
    assert_that!(bound(&deck, "g")).is_equal_to(id);
}

#[test]
fn inline_over_bounds_is_refused() {
    let deck = deck();
    let wide: Vec<serde_json::Value> = (0..=MAX_COLUMNS).map(|i| json!(format!("c{i}"))).collect();
    let option = json!({ "dataset": { "source": [wide] } });
    let prepared = prepare(&deck, vec![add_elem("t", "base/table", &option)], Origin::External).unwrap();
    let mut applied = deck.clone();
    let err = apply_all(&mut applied, prepared.ops, &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::TooLarge));
    assert_that!(applied).is_equal_to(deck);
}

/// JSON holds no NaN (serde reads it as null): the row cut is the warning inline data can raise.
#[test]
fn inline_long_row_is_cut_and_reported() {
    let mut deck = deck();
    let option = json!({ "dataset": { "source": [["N", "V"], ["a", 1], ["b", 1, "extra"]] } });
    let warnings = run(&mut deck, vec![add_elem("t", "base/table", &option)]);
    let keys: Vec<&str> = warnings.iter().map(|w| w.message.id()).collect();
    assert_that!(keys).is_equal_to(vec!["data-row-cut"]);
    assert_that!(warnings[0].element.as_deref()).is_equal_to(Some("t"));
}

#[test]
fn prepare_refuses_put_view_data_in_input() {
    let deck = deck_with_views();
    let prepared = prepare(&deck, vec![put_chart([1.0, 1.0])], Origin::External).unwrap();
    let put = prepared.ops.into_iter().next().unwrap();
    let err = prepare(&deck, vec![put], Origin::External).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Internal));
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr("op-data-internal"));
}

#[test]
fn deserialized_put_view_data_is_refused_by_apply() {
    let deck = deck_with_views();
    let prepared = prepare(&deck, vec![put_chart([1.0, 1.0])], Origin::External).unwrap();
    let json = serde_json::to_value(&prepared.ops[0]).unwrap();
    let forged: Op = serde_json::from_value(json).unwrap();
    let mut applied = deck.clone();
    let err = forged.apply(&mut applied, &registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Internal));
}

#[test]
fn ops_schema_has_no_put_view_data() {
    let schema = crate::ops_schema().unwrap().to_string();
    assert_that!(schema.contains("put_view_data")).is_false();
}

#[test]
fn prepare_then_apply_twice_on_clones_gives_equal_decks() {
    let deck = deck_with_views();
    let prepared = prepare(
        &deck,
        vec![add_elem("x", "base/chart", &inline_chart_option()), put_chart([2.0, 3.0])],
        Origin::External,
    )
    .unwrap();
    let (mut a, mut b) = (deck.clone(), deck.clone());
    apply_all(&mut a, prepared.ops.clone(), &registry()).unwrap();
    apply_all(&mut b, prepared.ops, &registry()).unwrap();
    assert_that!(a).is_equal_to(b);
}

// Ported from the session's chart canonicalization.

#[test]
fn prepare_moves_inline_series_to_a_localized_dataset() {
    let mut deck = deck();
    run(&mut deck, vec![add_elem("g", "base/chart", &inline_chart_option())]);
    let data = &deck.data[&did(&bound(&deck, "g"))];
    assert_that!(data.table().unwrap().columns[0].name.as_str()).is_equal_to("Catégorie");
    assert_that!(data.table().unwrap().columns[1].name.as_str()).is_equal_to("Série 1");
}

#[test]
fn prepare_reaches_slot_children_and_new_slides() {
    let stack: Element = serde_json::from_value(json!({
        "id": "grp", "kind": crate::kinds::base::STACK,
        "params": { "children": [serde_json::to_value(elem("g1", "base/chart", &inline_chart_option())).unwrap()] }
    }))
    .unwrap();
    let mut slide = Slide::new("s2", crate::kinds::essentiel::CONTENT);
    slide.params.set("blocks", ParamValue::Slot(vec![elem("g2", "base/chart", &inline_chart_option())]));
    let mut deck = deck();
    run(
        &mut deck,
        vec![
            Op::AddElement { slot: blocks(), index: None, defaults: false, element: stack },
            Op::AddSlide { index: None, defaults: false, slide },
        ],
    );
    assert_that!(deck.data.len()).is_equal_to(2);
}

#[test]
fn prepare_leaves_params_that_are_not_charts_alone() {
    let ops = vec![
        add_elem("t", crate::kinds::base::TEXT, &inline_chart_option()),
        add_elem("u", "inconnu", &inline_chart_option()),
    ];
    let prepared = prepare(&deck(), ops.clone(), Origin::External).unwrap();
    assert_that!(prepared.ops).is_equal_to(ops);
}

#[test]
fn prepare_leaves_other_ops_alone() {
    let ops = vec![Op::RemoveElement { slide: "s".into(), element: "g".into() }];
    let prepared = prepare(&deck(), ops.clone(), Origin::External).unwrap();
    assert_that!(prepared.ops).is_equal_to(ops);
}

#[rstest]
#[case::chart(Op::SetChartOption { target: target("g"), param: "option".into(), patch: json!({ "data": null, "dataset": { "source": [["M", "V"], ["a", 7]] } }) })]
#[case::table(Op::SetTableOption { target: target("t"), param: "data".into(), patch: json!({ "data": null, "dataset": { "source": [["N", "S"], ["x", 7]] } }) })]
fn prepare_lift_patch_with_data_null_is_refused(#[case] op: Op) {
    let deck = deck_with_views();
    let err = prepare(&deck, vec![op], Origin::External).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).contains(fr.tr("chart-ref-null").as_str());
}

/// The column ids of the table a `PutViewData` stores.
fn put_columns(op: &Op) -> Vec<String> {
    let Op::PutViewData { table, .. } = op else { panic!("not a put: {op:?}") };
    table.columns.iter().filter_map(|c| c.id.clone()).collect()
}

#[test]
fn reset_inline_skips_column_ids_given_later_in_the_batch() {
    let deck = deck_with_views();
    let own = did(&bound(&deck, "g"));
    let reset = || Op::SetParam {
        target: target("g"),
        param: "option".into(),
        value: json!({
            "dataset": { "source": [["Month", "Sales", "Extra"], ["Jan", 1, "x"]] },
            "xAxis": { "type": "category" },
            "series": [{ "type": "bar" }]
        }),
    };
    let alone = prepare(&deck, vec![reset()], Origin::External).unwrap();
    let drawn = put_columns(&alone.ops[0]).pop().unwrap();
    let insert = Op::EditData {
        data: own,
        change: TableOp::InsertColumn {
            id: Some(drawn.clone()),
            after: None,
            name: "Given".into(),
            cells: BTreeMap::new(),
        },
    };
    let both = prepare(&deck, vec![reset(), insert], Origin::External).unwrap();
    assert_that!(put_columns(&both.ops[0])).does_not_contain(&drawn);
    let mut applied = deck;
    apply_all(&mut applied, both.ops, &registry()).unwrap();
}

/// A large batch of inline charts with one title: each id is drawn in one probe, and names
/// follow one another. 500 charts, not 2,000: the apply's ref guard, run on the scratch deck
/// for each op, walks every view, which makes a larger batch slow in debug builds.
#[test]
fn prepare_batch_of_inline_charts_draws_each_id_in_one_probe() {
    let charts = 500;
    let deck = deck();
    let ops: Vec<Op> = (0..charts)
        .map(|i| add_elem(&format!("g{i}"), "base/chart", &dataset_chart(Some("Ventes"), [1.0, 2.0])))
        .collect();
    let probes = std::cell::Cell::new(0usize);
    let mut n = 0usize;
    let mut ids = |taken: &dyn Fn(&str) -> bool| loop {
        n += 1;
        probes.set(probes.get() + 1);
        let id = format!("d{n}");
        if !taken(&id) {
            return id;
        }
    };
    let registry = registry();
    let labels = ChartLabels::default();
    let mut ctx =
        PrepareCtx { deck: &deck, registry: &registry, labels: &labels, ids: &mut ids, origin: Origin::External };
    let prepared = prepare_ops(ops, &mut ctx).unwrap();
    let names: Vec<&str> = prepared
        .ops
        .iter()
        .filter_map(|op| if let Op::PutViewData { name, .. } = op { Some(name.as_str()) } else { None })
        .collect();
    assert_that!(names.len()).is_equal_to(charts);
    assert_that!(names[charts - 1]).is_equal_to(format!("ventes_{charts}").as_str());
    // Each chart draws a generation, its column and row ids, and its data id: no retries.
    let drawn: usize =
        prepared.ops.iter().filter(|op| matches!(op, Op::PutViewData { .. })).map(|_| 1 + 3 + 2 + 1).sum();
    assert_that!(probes.get()).is_equal_to(drawn);
}

#[rstest]
#[case::chart_with_dataset("base/chart", dataset_chart(None, [1.0, 2.0]))]
#[case::chart_series("base/chart", inline_chart_option())]
#[case::table_dataset("base/table", table_option([1, 2]))]
fn apply_refuses_inline_data_without_prepare(#[case] kind: &str, #[case] option: serde_json::Value) {
    let mut applied = deck();
    let before = applied.clone();
    let err = add_elem("x", kind, &option).apply(&mut applied, &registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Inline));
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr_with("op-data-inline", &[("subject", "x".into())]));
    assert_that!(applied).is_equal_to(before);
}

#[test]
fn apply_without_prepare_refuses_patch_touching_inline_series() {
    let deck = deck_with_views();
    let patch =
        json!({ "xAxis": { "type": "category", "data": ["a", "b"] }, "series": [{ "type": "bar", "data": [4, 6] }] });
    let op = Op::SetChartOption { target: target("g"), param: "option".into(), patch };
    let mut applied = deck.clone();
    let err = op.clone().apply(&mut applied, &registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Inline));
    assert_that!(applied).is_equal_to(&deck);
    let prepared = prepare(&deck, vec![op], Origin::External).unwrap();
    assert_that!(puts(&prepared.ops).len()).is_equal_to(1);
    apply_all(&mut applied, prepared.ops, &registry()).unwrap();
    assert_that!(option_of(&applied, "g")["series"][0].get("data")).is_none();
}

/// The keys task 4d adds, with sample args.
fn task4d_samples() -> Vec<(&'static str, Vec<(&'static str, oku_slides_i18n::FluentValue<'static>)>)> {
    vec![
        ("op-data-inline", vec![("subject", "g".into())]),
        ("issue-data-inline-dropped", vec![("element", "g".into())]),
        ("deck-inline-data", vec![("element", "g".into())]),
    ]
}

/// The chart keys task 4d renames, which take no args.
const TASK4D_RENAMED: [&str; 4] = ["chart-ids-length", "chart-ids-invalid", "chart-ids-duplicate", "chart-no-dataset"];

#[test]
fn every_task4d_key_formats_in_all_locales() {
    // The positional data op keys, built so the removed prefix appears nowhere in the code.
    let mut gone: Vec<String> = [
        "no-row",
        "no-column",
        "insert-row",
        "remove-row",
        "insert-column",
        "too-many-cells",
        "remove-column",
        "keep-header",
        "no-dataset",
        "ids-length",
        "ids-invalid",
        "ids-duplicate",
        "id-taken",
    ]
    .iter()
    .map(|s| format!("{}-{s}", "chart-data"))
    .collect();
    gone.push("chart-dataset-header".to_owned());
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in task4d_samples() {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.contains('g')).named(&format!("{locale} {key}: {text}")).is_true();
        }
        for key in TASK4D_RENAMED {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr(key);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
        }
        for key in &gone {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    for (key, args) in task4d_samples() {
        assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
    }
    for key in TASK4D_RENAMED {
        assert_that!(fr.tr(key)).named(key).is_not_equal_to(en.tr(key));
    }
}

/// An inline data read back as a query (a replica can hold one) is never switched to manual in
/// place by a view's new inline table: the put is refused as derived.
#[test]
fn reset_inline_over_a_query_data_is_refused_as_derived() {
    let mut deck = deck_with_views();
    let id = did(&bound(&deck, "g"));
    let data = std::sync::Arc::make_mut(deck.data.get_mut(&id).unwrap());
    data.source = oku_slides_data::Source::Query { sql: "SELECT 1".into() };
    let before = deck.clone();
    let registry = registry();
    let labels = ChartLabels::new(&Localizer::new(Some("fr")));
    let mut ids = counter();
    let mut ctx =
        PrepareCtx { deck: &mut deck, registry: &registry, labels: &labels, ids: &mut ids, origin: Origin::External };
    let prepared = prepare_ops(vec![put_chart([3.0, 4.0])], &mut ctx).unwrap();
    let mut applied = before.clone();
    let err = apply_all(&mut applied, prepared.ops, &registry).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Derived));
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with("op-data-derived", &[("data", id.as_str().into())]);
    assert_that!(err.op_error().localize(&fr)).is_equal_to(expected);
    assert_that!(deck.data).is_equal_to(&before.data);
}
