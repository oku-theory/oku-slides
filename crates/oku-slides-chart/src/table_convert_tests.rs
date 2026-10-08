use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Cell, ChartErrorKind, ChartLabels, ChartOption, ColumnAlign, MAX_TABLE_COLUMNS, parse_table};

fn chart(v: serde_json::Value) -> ChartOption {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn chart_to_table_keeps_the_dataset_with_header_column() {
    let c = chart(json!({ "dataset": { "source": [["M", "A"], ["x", 1]] }, "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }] }));
    let t = chart_to_table(&c).unwrap();
    assert_that!((t.dataset.source.len(), t.header_row, t.header_column)).is_equal_to((2, true, true));
}

#[test]
fn chart_to_table_pads_short_rows() {
    let c = chart(json!({ "dataset": { "source": [["M", "A", "B"], ["x", 1]] }, "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }, { "type": "bar" }] }));
    assert_that!(chart_to_table(&c).unwrap().dataset.source[1][2]).is_equal_to(Cell::Null);
}

#[test]
fn chart_to_table_without_dataset_is_a_data_op_error() {
    let c =
        chart(json!({ "xAxis": { "type": "category", "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] }));
    assert_that!(chart_to_table(&c).unwrap_err().kind()).is_equal_to(ChartErrorKind::DataOp);
}

#[test]
fn chart_to_table_beyond_max_columns_is_inconsistent() {
    let header: Vec<String> = (0..=MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
    let series: Vec<_> = (1..=MAX_TABLE_COLUMNS).map(|_| json!({ "type": "bar" })).collect();
    let c = chart(json!({ "dataset": { "source": [header] }, "xAxis": { "type": "category" }, "series": series }));
    assert_that!(chart_to_table(&c).unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn table_to_chart_gets_one_bar_series_per_value_column() {
    let t = parse_table(&json!({ "dataset": { "source": [["M", "A", "B"], ["x", 1, 2]] } })).unwrap();
    let c = table_to_chart(&t, ChartLabels::english()).unwrap();
    assert_that!(c.series.len()).is_equal_to(2);
}

#[test]
fn table_to_chart_with_text_values_keeps_them_as_missing() {
    let t = parse_table(&json!({ "dataset": { "source": [["M", "A"], ["x", "beaucoup"]] },
        "align": [ColumnAlign::Start] }))
    .unwrap();
    assert_that!(table_to_chart(&t, ChartLabels::english())).is_ok();
}
