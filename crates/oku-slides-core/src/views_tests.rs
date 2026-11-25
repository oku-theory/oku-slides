use oku_slides_data::limits::MAX_ROWS;
use oku_slides_data::{DataId, NewTable};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::ops::{Op, SlotRef, Target, apply_all};
use crate::params::ParamValue;
use crate::test_registry::registry;

fn did(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap()
}

/// A table of `rows` rows and `width` columns `c0`…, cells `text` (or numbers when `None`).
fn table(rows: usize, width: usize, text: Option<&str>) -> NewTable {
    let columns: Vec<Value> = (0..width).map(|i| json!({ "id": format!("c{i}"), "name": format!("C{i}") })).collect();
    let cell = |i: usize| text.map_or(json!(i), |t| json!(t));
    let rows: Vec<Value> = (0..rows)
        .map(|r| json!({ "id": format!("r{r}"), "cells": (0..width).map(cell).collect::<Vec<_>>() }))
        .collect();
    serde_json::from_value(json!({ "generation": "g", "columns": columns, "rows": rows })).unwrap()
}

fn view(id: &str, kind: &str, option: Value) -> Op {
    let param = if kind == "base/table" { "data" } else { "option" };
    let params = serde_json::Map::from_iter([(param.to_owned(), option)]);
    let element = serde_json::from_value(json!({ "id": id, "kind": kind, "params": params })).unwrap();
    let slot = SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() };
    Op::AddElement { slot, index: None, defaults: false, element }
}

fn chart(data: &str) -> Value {
    json!({ "data": data, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] })
}

/// A deck holding data `d` and the views `ops` add.
fn deck(data: NewTable, ops: Vec<Op>) -> Deck {
    let mut deck = Deck::from_json(r#"{"slides":[{"id":"s","kind":"essentiel/content"}]}"#).unwrap();
    let mut all = vec![Op::AddData { id: Some(did("d")), name: "d".into(), label: None, table: data }];
    all.extend(ops);
    apply_all(&mut deck, all, &registry()).unwrap();
    deck
}

fn keys(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

#[rstest]
#[case::fits(99, 99, None)]
#[case::one_over(100, 99, Some(100))]
#[case::many_over(150, 99, Some(150))]
fn plan_table_rows_cut(#[case] rows: usize, #[case] shown: usize, #[case] total: Option<usize>) {
    let deck = deck(table(rows, 2, None), vec![view("t", "base/table", json!({ "data": "d" }))]);
    let (plans, issues) = plan_views(&deck, &registry());
    assert_that!(plans[0].rows.len()).is_equal_to(shown);
    match total {
        None => assert_that!(issues).is_empty(),
        Some(total) => {
            assert_that!(keys(&issues)).is_equal_to(vec!["issue-view-truncated"]);
            let expected = Message::new("issue-view-truncated")
                .arg("element", "t")
                .arg("what", "rows")
                .arg("shown", shown)
                .arg("total", total);
            assert_that!(issues[0].message).is_equal_to(expected);
        }
    }
}

#[rstest]
#[case::fits(20, false)]
#[case::one_over(21, true)]
fn plan_table_columns_cut(#[case] width: usize, #[case] cut: bool) {
    let deck = deck(table(1, width, None), vec![view("t", "base/table", json!({ "data": "d" }))]);
    let (plans, issues) = plan_views(&deck, &registry());
    assert_that!(plans[0].columns.len()).is_equal_to(width.min(20));
    assert_that!(plans[0].outcome).is_equal_to(PlanOutcome::Resolve { cut });
    assert_that!(issues.len()).is_equal_to(usize::from(cut));
}

#[test]
fn plan_chart_reads_only_used_columns_and_every_row() {
    let deck = deck(table(3000, 5, None), vec![view("g", "base/chart", chart("d"))]);
    let (plans, issues) = plan_views(&deck, &registry());
    assert_that!(plans[0].columns).is_equal_to(vec!["c0".to_owned(), "c1".to_owned()]);
    assert_that!(plans[0].rows.len()).is_equal_to(3000);
    assert_that!(issues).is_empty();
}

#[test]
fn plan_projection_is_read_in_order() {
    let mut option = chart("d");
    option["columns"] = json!(["c3", "c1"]);
    let deck = deck(table(2, 5, None), vec![view("g", "base/chart", option)]);
    let (plans, _) = plan_views(&deck, &registry());
    assert_that!(plans[0].columns).is_equal_to(vec!["c3".to_owned(), "c1".to_owned()]);
}

/// Two charts read two columns each: 50,000 rows of `d` plus 50,000 (and `extra`) rows of `e`
/// put the second view at the cell budget, or over it.
#[rstest]
#[case::exact(0, PlanOutcome::Resolve { cut: false })]
#[case::one_over(1, PlanOutcome::OverBudget)]
fn plan_cell_budget(#[case] extra: usize, #[case] second: PlanOutcome) {
    let rows = MAX_RESOLVED_CELLS / 4;
    let mut deck = deck(table(rows, 2, None), vec![view("a", "base/chart", chart("d"))]);
    let ops = vec![
        Op::AddData { id: Some(did("e")), name: "e".into(), label: None, table: table(rows + extra, 2, None) },
        view("b", "base/chart", chart("e")),
    ];
    apply_all(&mut deck, ops, &registry()).unwrap();
    let (plans, issues) = plan_views(&deck, &registry());
    assert_that!(plans[0].outcome).is_equal_to(PlanOutcome::Resolve { cut: false });
    assert_that!(plans[1].outcome).is_equal_to(second.clone());
    let over = second == PlanOutcome::OverBudget;
    let expected = over.then(|| Message::new("issue-render-budget").arg("element", "b"));
    assert_that!(issues.first().map(|i| i.message.clone())).is_equal_to(expected);
    assert_that!(issues.len()).is_equal_to(usize::from(over));
}

#[test]
fn plan_counts_shared_data_once_per_view() {
    let rows = MAX_RESOLVED_CELLS / 4 + 1;
    let deck =
        deck(table(rows, 2, None), vec![view("a", "base/chart", chart("d")), view("b", "base/chart", chart("d"))]);
    let (plans, _) = plan_views(&deck, &registry());
    assert_that!(plans[0].outcome).is_equal_to(PlanOutcome::Resolve { cut: false });
    assert_that!(plans[1].outcome).is_equal_to(PlanOutcome::OverBudget);
}

/// One text column named `n` holding `cells`.
fn text_table(cells: &[String]) -> NewTable {
    let rows: Vec<Value> =
        cells.iter().enumerate().map(|(r, c)| json!({ "id": format!("r{r}"), "cells": [c] })).collect();
    serde_json::from_value(json!({ "generation": "g", "columns": [{ "id": "c0", "name": "n" }], "rows": rows }))
        .unwrap()
}

/// Four views of `d` read 16,000,004 bytes; a fifth view of `e` brings the sum to the byte
/// budget, plus `extra`. The deck stays under its own 8 MiB text bound: shared data counts per view.
#[rstest]
#[case::exact(0, PlanOutcome::Resolve { cut: false })]
#[case::one_over(1, PlanOutcome::OverBudget)]
fn plan_byte_budget(#[case] extra: usize, #[case] last: PlanOutcome) {
    let radar = || json!({ "data": "d", "series": [{ "type": "radar" }] });
    let views = (0..4).map(|i| view(&format!("v{i}"), "base/chart", radar())).collect();
    let mut deck = deck(text_table(&vec!["x".repeat(1000); 4000]), views);
    let rest = MAX_RESOLVED_BYTES - 4 * (1 + 4000 * 1000) - 1;
    let mut cells = vec!["x".repeat(1000); rest / 1000];
    cells.push("x".repeat(rest % 1000 + extra));
    let mut e = radar();
    e["data"] = json!("e");
    let ops = vec![
        Op::AddData { id: Some(did("e")), name: "e".into(), label: None, table: text_table(&cells) },
        view("w", "base/chart", e),
    ];
    apply_all(&mut deck, ops, &registry()).unwrap();
    let (plans, _) = plan_views(&deck, &registry());
    let total: usize = plans.iter().map(|p| p.text_bytes).sum();
    assert_that!(total).is_equal_to(MAX_RESOLVED_BYTES + extra);
    assert_that!(plans[3].outcome).is_equal_to(PlanOutcome::Resolve { cut: false });
    assert_that!(plans[4].outcome).is_equal_to(last);
}

#[test]
fn plan_cell_over_1000_chars_counts_its_cut() {
    let deck = deck(table(1, 1, Some(&"é".repeat(1500))), vec![view("t", "base/table", json!({ "data": "d" }))]);
    let (plans, _) = plan_views(&deck, &registry());
    assert_that!(plans[0].text_bytes).is_equal_to(2 + 2000);
}

#[test]
fn plan_missing_data_reports_it() {
    let mut deck = deck(table(1, 2, None), vec![view("g", "base/chart", chart("d"))]);
    deck.data.clear();
    let (plans, issues) = plan_views(&deck, &registry());
    assert_that!(plans[0].outcome).is_equal_to(PlanOutcome::MissingData);
    let expected = Message::new("issue-data-missing").arg("element", "g").arg("data", "d");
    assert_that!(issues[0].message).is_equal_to(expected);
}

#[test]
fn plan_projected_column_removed_by_a_peer_reports_without_panic() {
    let mut option = chart("d");
    option["columns"] = json!(["c0", "c1"]);
    let mut deck = deck(table(1, 2, None), vec![view("g", "base/chart", option)]);
    let data = deck.data.get_mut(&did("d")).unwrap();
    std::sync::Arc::make_mut(data).table.columns.pop();
    let (plans, issues) = plan_views(&deck, &registry());
    assert_that!(plans[0].outcome).is_equal_to(PlanOutcome::MissingColumn("c1".into()));
    let expected = Message::new("issue-view-column-missing").arg("element", "g").arg("column", "c1");
    assert_that!(issues.iter().map(|i| i.message.clone()).collect::<Vec<_>>()).is_equal_to(vec![expected]);
}

/// `deck` with the option of its first block set to `option`, as a merge could leave it.
fn with_stored_option(mut deck: Deck, option: Value) -> Deck {
    let (_, blocks) = deck.slides[0].params.slots_mut().next().unwrap();
    blocks[0].params.set("option", ParamValue::Value(option));
    deck
}

#[rstest]
#[case::projection_not_text(json!({ "data": "d", "columns": ["c0", 3], "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] }))]
#[case::option_unreadable(json!({ "data": "d", "series": "bar" }))]
fn plan_unreadable_view_reports_it(#[case] option: Value) {
    let deck = with_stored_option(deck(table(1, 2, None), vec![view("g", "base/chart", chart("d"))]), option);
    let (plans, issues) = plan_views(&deck, &registry());
    assert_that!(plans[0].outcome).is_equal_to(PlanOutcome::Unreadable);
    let expected = Message::new("issue-view-unreadable").arg("element", "g");
    assert_that!(issues.iter().map(|i| i.message.clone()).collect::<Vec<_>>()).is_equal_to(vec![expected]);
}

#[test]
fn plan_table_cut_on_rows_and_columns_reports_both() {
    let deck = deck(table(150, 21, None), vec![view("t", "base/table", json!({ "data": "d" }))]);
    let (_, issues) = plan_views(&deck, &registry());
    let cut = |what: &str, shown: usize, total: usize| {
        Message::new("issue-view-truncated")
            .arg("element", "t")
            .arg("what", what.to_owned())
            .arg("shown", shown)
            .arg("total", total)
    };
    let messages: Vec<Message> = issues.into_iter().map(|i| i.message).collect();
    assert_that!(messages).is_equal_to(vec![cut("rows", 99, 150), cut("columns", 20, 21)]);
}

/// A view over the cell budget is refused before its text is read: the fourth of four views
/// reading 65,536 text cells each.
#[test]
fn plan_over_cell_budget_reads_no_text() {
    let mut option = chart("d");
    option["columns"] = json!(["c0"]);
    let views = (0..4).map(|i| view(&format!("v{i}"), "base/chart", option.clone())).collect();
    let deck = deck(text_table(&vec!["x".to_owned(); MAX_ROWS]), views);
    let (plans, _) = plan_views(&deck, &registry());
    assert_that!(plans[2].outcome).is_equal_to(PlanOutcome::Resolve { cut: false });
    assert_that!(plans[3].outcome).is_equal_to(PlanOutcome::OverBudget);
    assert_that!(plans[3].text_bytes).is_equal_to(0);
}

/// Text is counted only until it passes the bytes left: the fifth of five views reading
/// 4,000,000 bytes each stops soon after the 777,216 left.
#[test]
fn plan_text_count_stops_past_the_byte_budget() {
    let radar = || json!({ "data": "d", "series": [{ "type": "radar" }] });
    let views = (0..5).map(|i| view(&format!("v{i}"), "base/chart", radar())).collect();
    let deck = deck(text_table(&vec!["x".repeat(1000); 4000]), views);
    let (plans, _) = plan_views(&deck, &registry());
    let left = MAX_RESOLVED_BYTES - 4 * (1 + 4000 * 1000);
    assert_that!(plans[4].outcome).is_equal_to(PlanOutcome::OverBudget);
    assert_that!(plans[4].text_bytes).is_greater_than(left);
    assert_that!(plans[4].text_bytes).is_less_than_or_equal_to(left + 1000);
}

fn task4b_samples() -> Vec<(&'static str, Vec<(&'static str, oku_slides_i18n::FluentValue<'static>)>)> {
    let truncated = |what: &'static str| {
        vec![("element", "g".into()), ("what", what.into()), ("shown", 99.into()), ("total", 150.into())]
    };
    vec![
        ("render-chart-too-many-values", vec![("count", 20_001.into()), ("max", 20_000.into())]),
        ("issue-view-truncated", truncated("rows")),
        ("issue-view-truncated", truncated("columns")),
        ("issue-render-budget", vec![("element", "g".into())]),
        ("issue-data-missing", vec![("element", "g".into()), ("data", "sales".into())]),
        ("issue-view-column-missing", vec![("element", "g".into()), ("column", "c1".into())]),
        ("issue-view-unreadable", vec![("element", "g".into())]),
        ("data-row-cut", vec![("count", 3.into())]),
    ]
}

#[test]
fn every_task4b_key_formats_in_all_locales() {
    use oku_slides_i18n::Localizer;
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in task4b_samples() {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.trim().is_empty()).named(&format!("{locale} {key}")).is_false();
        }
        for gone in ["session-undo-chart-data", "session-undo-table-data"] {
            assert_that!(l.has(gone)).named(&format!("{locale} {gone}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    for (key, args) in task4b_samples() {
        assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
    }
    let what =
        |w: &'static str| vec![("element", "g".into()), ("what", w.into()), ("shown", 1.into()), ("total", 2.into())];
    assert_that!(fr.tr_with("issue-view-truncated", &what("rows")))
        .is_not_equal_to(fr.tr_with("issue-view-truncated", &what("columns")));
}

#[rstest]
#[case::ru("ru", vec![1, 2, 5, 21, 22])]
#[case::pl("pl", vec![1, 2, 5])]
fn data_row_cut_has_distinct_plural_forms(#[case] locale: &str, #[case] counts: Vec<i32>) {
    let l = oku_slides_i18n::Localizer::new(Some(locale));
    let form = |n: i32| l.tr_with("data-row-cut", &[("count", n.into())]).replace(&n.to_string(), "#");
    let (one, few, many) = (form(1), form(2), form(5));
    assert_that!(one).is_not_equal_to(few.clone());
    assert_that!(few.clone()).is_not_equal_to(many);
    if counts.len() > 3 {
        assert_that!(form(21)).is_equal_to(one);
        assert_that!(form(22)).is_equal_to(few);
    }
}
