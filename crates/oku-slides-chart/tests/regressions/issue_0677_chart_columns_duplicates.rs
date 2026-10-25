//! `chart-columns` acceptait des ids en double et une liste de longueur arbitraire.

use oku_slides_chart::{ChartErrorKind, MAX_TABLE_COLUMNS, parse_table};
use serde_json::json;
use speculoos::prelude::*;

fn kind(ids: &serde_json::Value) -> Option<ChartErrorKind> {
    parse_table(&json!({ "dataset": { "source": [["A", "B"]] }, "chart": {}, "chart-columns": ids }))
        .err()
        .map(|e| e.kind())
}

#[test]
fn duplicate_chart_columns_are_refused() {
    assert_that!(kind(&json!(["a", "a"]))).is_equal_to(Some(ChartErrorKind::Inconsistent));
}

#[test]
fn too_many_chart_columns_are_refused() {
    let ids: Vec<String> = (0..=MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
    assert_that!(kind(&json!(ids))).is_equal_to(Some(ChartErrorKind::Inconsistent));
}

#[test]
fn distinct_chart_columns_are_accepted() {
    assert_that!(kind(&json!(["a", "b"]))).is_none();
}
