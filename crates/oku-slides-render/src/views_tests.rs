use std::sync::Arc;

use oku_slides_core::kinds::{base, essentiel};
use oku_slides_core::ops::{Op, SlotRef, Target, apply_all};
use oku_slides_core::{Deck, ParamValue, PlanOutcome, WidgetRegistry, plan_views, validate::validate};
use oku_slides_data::{BaseId, BaseRef, DataId, NewTable, TableOp};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{BaseRead, Results, resolve_views, with_results};

/// Base registry, read from its frozen probe (`fixtures/widgets`).
fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let text = std::fs::read_to_string(path).unwrap();
    WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap()
}

fn did(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap()
}

/// `rows` rows by `width` columns `c0`… named `C0`…; cell (r, i) is `r * 10 + i`, or `text`.
fn table(rows: usize, width: usize, text: Option<&str>) -> NewTable {
    let columns: Vec<Value> = (0..width).map(|i| json!({ "id": format!("c{i}"), "name": format!("C{i}") })).collect();
    let rows: Vec<Value> = (0..rows)
        .map(|r| {
            let cells: Vec<Value> = (0..width).map(|i| text.map_or(json!(r * 10 + i), |t| json!(t))).collect();
            json!({ "id": format!("r{r}"), "cells": cells })
        })
        .collect();
    serde_json::from_value(json!({ "generation": "g", "columns": columns, "rows": rows })).unwrap()
}

fn param(kind: &str) -> &'static str {
    if kind == base::TABLE { "data" } else { "option" }
}

fn view(id: &str, kind: &str, option: Value) -> Op {
    let params = serde_json::Map::from_iter([(param(kind).to_owned(), option)]);
    let element = serde_json::from_value(json!({ "id": id, "kind": kind, "params": params })).unwrap();
    let slot = SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() };
    Op::AddElement { slot, index: None, defaults: false, element }
}

fn chart(data: &str) -> Value {
    json!({ "data": data, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] })
}

fn add_data(id: &str, table: NewTable) -> Op {
    Op::AddData { id: Some(did(id)), name: id.into(), label: None, table: Some(table), sql: None }
}

/// A deck holding data `d` and the views `ops` add.
fn deck(data: NewTable, ops: Vec<Op>) -> Deck {
    let mut deck = Deck::from_json(&format!(r#"{{"slides":[{{"id":"s","kind":"{}"}}]}}"#, essentiel::CONTENT)).unwrap();
    let mut all = vec![add_data("d", data)];
    all.extend(ops);
    apply_all(&mut deck, all, &registry()).unwrap();
    deck
}

/// The resolved value of element `id`'s view param.
fn resolved(deck: &Deck, id: &str) -> Value {
    let slide = &deck.slides[0];
    let e = slide.walk().find(|l| l.element.id == id).unwrap().element;
    match e.params.get(param(&e.kind)) {
        Some(ParamValue::Value(v)) => v.clone(),
        other => panic!("no view value: {other:?}"),
    }
}

fn set_value(deck: &mut Deck, id: &str, f: impl FnOnce(&mut Value)) {
    let blocks = deck.slides[0].params.slot_mut("blocks").unwrap();
    let e = blocks.iter_mut().find(|e| e.id == id).unwrap();
    let name = param(&e.kind);
    let Some(ParamValue::Value(v)) = e.params.0.get_mut(name) else { panic!("no view value") };
    f(v);
}

fn source(v: &Value) -> &Vec<Value> {
    v["dataset"]["source"].as_array().unwrap()
}

#[test]
fn resolve_projects_columns() {
    let mut option = chart("d");
    option["columns"] = json!(["c2", "c0"]);
    let r = resolve_views(
        &deck(table(2, 3, None), vec![view("g", base::CHART, option)]),
        &registry(),
        None,
        &Results::new(),
    );
    let v = resolved(&r, "g");
    assert_that!(v["dataset"]["source"]).is_equal_to(json!([["C2", "C0"], [2.0, 0.0], [12.0, 10.0]]));
    // Ids serve edits only: the drawn dataset carries none.
    assert_that!(v["dataset"].get("columns")).is_none();
    assert_that!(v["dataset"].get("ids")).is_none();
    assert_that!(v.get("data")).is_none();
    assert_that!(v.get("columns")).is_none();
}

#[test]
fn resolve_two_projections_of_one_data() {
    let mut a = chart("d");
    a["columns"] = json!(["c0", "c1"]);
    let mut b = chart("d");
    b["columns"] = json!(["c0", "c2"]);
    let d = deck(table(1, 3, None), vec![view("a", base::CHART, a), view("b", base::CHART, b)]);
    let r = resolve_views(&d, &registry(), None, &Results::new());
    assert_that!(resolved(&r, "a")["dataset"]["source"]).is_equal_to(json!([["C0", "C1"], [0.0, 1.0]]));
    assert_that!(resolved(&r, "b")["dataset"]["source"]).is_equal_to(json!([["C0", "C2"], [0.0, 2.0]]));
}

#[test]
fn resolve_empties_data() {
    let r = resolve_views(
        &deck(table(1, 2, None), vec![view("g", base::CHART, chart("d"))]),
        &registry(),
        None,
        &Results::new(),
    );
    assert_that!(r.data.is_empty()).is_true();
}

#[test]
fn resolve_leaves_input_deck_unchanged() {
    let d = deck(table(1, 2, None), vec![view("g", base::CHART, chart("d"))]);
    let before = d.clone();
    let _ = resolve_views(&d, &registry(), None, &Results::new());
    assert_that!(d).is_equal_to(before);
}

#[test]
fn forged_param_with_inline_dataset_is_ignored() {
    let mut d = deck(table(1, 2, None), vec![view("g", base::CHART, chart("d"))]);
    set_value(&mut d, "g", |v| v["dataset"] = json!({ "source": [["X", "Y"], ["forged", 666]] }));
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "g");
    assert_that!(v.to_string()).does_not_contain("forged");
    assert_that!(v["dataset"]["source"]).is_equal_to(json!([["C0", "C1"], [0.0, 1.0]]));
}

/// The inline path stays readable until task 4d removes it.
#[test]
fn param_without_data_keeps_its_inline_dataset() {
    let mut d = deck(table(1, 2, None), vec![view("g", base::CHART, chart("d"))]);
    let inline = json!({ "source": [["X", "Y"], ["a", 1]] });
    set_value(&mut d, "g", |v| {
        v.as_object_mut().unwrap().remove("data");
        v["dataset"] = inline.clone();
    });
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "g");
    assert_that!(v["dataset"]).is_equal_to(inline);
}

#[rstest]
#[case::fits(99, 99)]
#[case::one_over(100, 99)]
#[case::many_over(150, 99)]
fn resolve_table_rows_cut(#[case] rows: usize, #[case] shown: usize) {
    let d = deck(table(rows, 2, None), vec![view("t", base::TABLE, json!({ "data": "d" }))]);
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "t");
    assert_that!(source(&v).len()).is_equal_to(shown + 1);
}

#[rstest]
#[case::fits(20, 20)]
#[case::one_over(21, 20)]
fn resolve_table_columns_cut(#[case] width: usize, #[case] shown: usize) {
    let d = deck(table(1, width, None), vec![view("t", base::TABLE, json!({ "data": "d" }))]);
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "t");
    assert_that!(source(&v)[0].as_array().unwrap().len()).is_equal_to(shown);
}

#[rstest]
#[case::table(base::TABLE)]
#[case::chart(base::CHART)]
fn resolve_cell_over_1000_chars_is_cut(#[case] kind: &str) {
    let long = "é".repeat(1500);
    let option = if kind == base::TABLE { json!({ "data": "d" }) } else { chart("d") };
    let d = deck(table(1, 2, Some(&long)), vec![view("v", kind, option)]);
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "v");
    assert_that!(source(&v)[1][0].as_str().unwrap().chars().count()).is_equal_to(1000);
}

#[test]
fn resolve_chart_gets_only_used_columns() {
    let d = deck(table(2, 5, None), vec![view("g", base::CHART, chart("d"))]);
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "g");
    assert_that!(v["dataset"]["source"][0]).is_equal_to(json!(["C0", "C1"]));
}

/// A projected chart reads its columns by id: an edit elsewhere in the data never moves its series.
#[rstest]
#[case::column_inserted_before(TableOp::InsertColumn { id: Some("n".into()), after: None, name: "N".into(), cells: std::collections::BTreeMap::default() }, "C2")]
#[case::unused_column_removed(TableOp::RemoveColumn { column: "c3".into() }, "C2")]
#[case::used_column_renamed(TableOp::RenameColumn { column: "c2".into(), name: "Ventes".into() }, "Ventes")]
fn series_follow_column_ids_after_edit(#[case] change: TableOp, #[case] series: &str) {
    let option =
        json!({ "data": "d", "columns": ["c0", "c2"], "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] });
    let mut d = deck(table(2, 4, None), vec![view("g", base::CHART, option)]);
    apply_all(&mut d, vec![Op::EditData { data: did("d"), change }], &registry()).unwrap();
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "g");
    assert_that!(v["dataset"]["source"]).is_equal_to(json!([["C0", series], [0.0, 2.0], [10.0, 12.0]]));
}

#[test]
fn resolve_chart_never_cut() {
    let d = deck(table(3000, 2, None), vec![view("g", base::CHART, chart("d"))]);
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "g");
    assert_that!(source(&v).len()).is_equal_to(3001);
}

/// Views of 50,000 rows by 2 columns: two sum exactly 200,000 cells; a third row on the
/// second's data puts it over, and only it loses its dataset (shared data counts per view).
#[rstest]
#[case::exact(0, true)]
#[case::one_over(1, false)]
fn resolve_cell_budget(#[case] extra: usize, #[case] second: bool) {
    let rows = oku_slides_core::MAX_RESOLVED_CELLS / 4;
    let ops = vec![
        add_data("e", table(rows + extra, 2, None)),
        view("a", base::CHART, chart("d")),
        view("b", base::CHART, chart("e")),
    ];
    let r = resolve_views(&deck(table(rows, 2, None), ops), &registry(), None, &Results::new());
    assert_that!(resolved(&r, "a").get("dataset").is_some()).is_true();
    assert_that!(resolved(&r, "b").get("dataset").is_some()).is_equal_to(second);
}

#[test]
fn resolve_missing_data_reports_and_skips() {
    let mut d = deck(table(1, 2, None), vec![view("g", base::CHART, chart("d"))]);
    d.data.clear();
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "g");
    assert_that!(v.get("dataset")).is_none();
    let keys: Vec<&str> = validate(&d, &registry(), None).iter().map(|i| i.message.id()).collect();
    assert_that!(keys).contains("issue-data-missing");
}

#[test]
fn resolve_projected_column_removed_by_peer_reports_without_panic() {
    let mut option = chart("d");
    option["columns"] = json!(["c0", "c1"]);
    let mut d = deck(table(1, 2, None), vec![view("g", base::CHART, option)]);
    Arc::make_mut(d.data.get_mut(&did("d")).unwrap()).table_mut().unwrap().columns.pop();
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "g");
    assert_that!(v.get("dataset")).is_none();
}

/// A deck whose data `d` (1 row, 2 columns) has a base.
fn based_deck() -> Deck {
    let mut d = deck(table(1, 2, None), vec![view("g", base::CHART, chart("d"))]);
    let id = BaseId::try_from(format!("sha256:{}", "a".repeat(64))).unwrap();
    let base_ref = BaseRef { id, bytes: 10, rows: 1, columns: Vec::new() };
    Arc::make_mut(d.data.get_mut(&did("d")).unwrap()).table_mut().unwrap().base = Some(base_ref);
    d
}

/// A base reader answering one fixed row, recording the columns it was asked for.
struct FixedBase(std::sync::Mutex<Vec<Vec<String>>>);

impl BaseRead for FixedBase {
    fn read_view(
        &self,
        _table: &oku_slides_data::Table,
        columns: &[String],
        _rows: std::ops::Range<usize>,
        _max_cell_chars: usize,
    ) -> Result<oku_slides_data::Dataset, oku_slides_data::DataError> {
        self.0.lock().unwrap().push(columns.to_vec());
        let row = columns.iter().map(|_| json!(7)).collect::<Vec<_>>();
        let header = columns.iter().map(|c| json!(c)).collect::<Vec<_>>();
        Ok(serde_json::from_value(json!({ "source": [header, row] })).unwrap())
    }
}

#[test]
fn in_process_render_of_based_view_draws_failure_frame() {
    let v = resolved(&resolve_views(&based_deck(), &registry(), None, &Results::new()), "g");
    assert_that!(v.get("dataset")).is_none();
}

/// → the plan's columns reach the reader, and its dataset the view.
#[test]
fn based_view_reads_through_its_base_reader() {
    let bases = FixedBase(std::sync::Mutex::new(Vec::new()));
    let v = resolved(&resolve_views(&based_deck(), &registry(), Some(&bases), &Results::new()), "g");
    assert_that!(source(&v)[1]).is_equal_to(json!([7.0, 7.0]));
    assert_that!(*bases.0.lock().unwrap()).is_equal_to(vec![vec!["c0".to_owned(), "c1".to_owned()]]);
}

#[test]
fn base_reader_refusal_draws_failure_frame() {
    struct Refusing;
    impl BaseRead for Refusing {
        fn read_view(
            &self,
            _: &oku_slides_data::Table,
            _: &[String],
            _: std::ops::Range<usize>,
            _: usize,
        ) -> Result<oku_slides_data::Dataset, oku_slides_data::DataError> {
            Err(oku_slides_data::DataError::base_invalid("forged"))
        }
    }
    let v = resolved(&resolve_views(&based_deck(), &registry(), Some(&Refusing), &Results::new()), "g");
    assert_that!(v.get("dataset")).is_none();
}

/// `align` and `column-widths` index the resolved columns: the projection's order, not the data's.
#[test]
fn table_positional_fields_index_resolved_columns() {
    let option =
        json!({ "data": "d", "columns": ["c2", "c0"], "align": ["end", "start"], "column-widths": [2.0, 1.0] });
    let d = deck(table(1, 3, None), vec![view("t", base::TABLE, option)]);
    let v = resolved(&resolve_views(&d, &registry(), None, &Results::new()), "t");
    let parsed = oku_slides_chart::parse_table(&v).unwrap();
    assert_that!(parsed.dataset.unwrap().source[0].len()).is_equal_to(2);
    assert_that!(parsed.align.len()).is_equal_to(2);
    assert_that!(parsed.column_widths).is_equal_to(vec![2.0, 1.0]);
}

/// A view resolves exactly when its plan does, and the issues validate reports are the plan's.
#[test]
fn plan_and_resolve_agree_on_issues() {
    let ops = vec![
        view("ok", base::CHART, chart("d")),
        view("cut", base::TABLE, json!({ "data": "d" })),
        view("missing", base::CHART, chart("d")),
        view("gone", base::CHART, chart("d")),
    ];
    let mut d = deck(table(120, 2, None), ops);
    set_value(&mut d, "missing", |v| v["data"] = json!("nowhere"));
    set_value(&mut d, "gone", |v| v["columns"] = json!(["c0", "c9"]));
    let r = resolve_views(&d, &registry(), None, &Results::new());
    let (plans, issues) = plan_views(&d, &registry());
    for plan in &plans {
        let resolves = matches!(plan.outcome, PlanOutcome::Resolve { .. });
        assert_that!(resolved(&r, &plan.view.element).get("dataset").is_some()).is_equal_to(resolves);
    }
    let all = validate(&d, &registry(), None);
    for issue in &issues {
        assert_that!(all.contains(issue)).is_true();
    }
    let keys: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(keys).is_equal_to(vec!["issue-view-truncated", "issue-data-missing", "issue-view-column-missing"]);
}

/// A deck holding data `d` and query data `q` reading it, with chart `g` on `q`.
fn query_deck() -> Deck {
    let mut deck = deck(table(2, 2, None), Vec::new());
    let add = Op::AddData {
        id: Some(did("q")),
        name: "q".into(),
        label: None,
        table: None,
        sql: Some("SELECT * FROM d".into()),
    };
    apply_all(&mut deck, vec![add, view("g", base::CHART, chart("q"))], &registry()).unwrap();
    deck
}

/// The result of `q`: data `d`'s table with its row ids renumbered, as a ready result reads.
fn result_table(rows: usize) -> oku_slides_data::Table {
    let d = query_deck();
    let mut t = d.data.get(&did("d")).unwrap().table().unwrap().clone();
    t.rows = oku_slides_data::Rows::repaired((0..rows).map(|i| format!("r{i}")));
    t
}

#[test]
fn query_view_reads_its_ready_result() {
    let results = Results::from([(did("q"), Some(result_table(2)))]);
    let v = resolved(&resolve_views(&query_deck(), &registry(), None, &results), "g");
    assert_that!(source(&v)).has_length(3);
}

#[rstest]
#[case::pending(Results::new())]
#[case::failed(Results::from([(did("q"), None)]))]
fn query_view_without_result_draws_failure_frame(#[case] results: Results) {
    let v = resolved(&resolve_views(&query_deck(), &registry(), None, &results), "g");
    assert_that!(v.get("dataset")).is_none();
}

#[test]
fn with_results_leaves_manual_data_and_names() {
    let d = query_deck();
    let results = Results::from([(did("q"), Some(result_table(2))), (did("d"), Some(result_table(1)))]);
    let derived = with_results(&d, &results);
    assert_that!(derived.data.get(&did("d"))).is_equal_to(d.data.get(&did("d")));
    let q = derived.data.get(&did("q")).unwrap();
    assert_that!(q.name.as_str()).is_equal_to("q");
    assert_that!(q.table().map(|t| t.rows.len())).is_equal_to(Some(2));
}
