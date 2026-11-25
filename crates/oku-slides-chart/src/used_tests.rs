use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::option::ChartOption;

const HEADERS: [&str; 6] = ["Month", "North", "South", "East", "Total", "Note"];

fn option(v: Value) -> ChartOption {
    serde_json::from_value(v).unwrap()
}

#[rstest]
#[case::two_bars(json!({ "xAxis": { "type": "category" }, "series": [{ "type": "bar" }, { "type": "bar" }] }), 3)]
#[case::horizontal_bars(
    json!({ "yAxis": { "type": "category" }, "xAxis": { "type": "value" }, "series": [{ "type": "bar" }] }),
    2
)]
#[case::encoded_by_index(json!({ "series": [{ "type": "line", "encode": { "x": 0, "y": 3 } }] }), 4)]
#[case::encoded_by_name(json!({ "series": [{ "type": "line", "encode": { "y": "East" } }] }), 4)]
#[case::pie(json!({ "series": [{ "type": "pie" }] }), 2)]
#[case::waterfall_with_totals(json!({ "series": [{ "type": "waterfall", "encode": { "y": 3 } }] }), 5)]
#[case::scatter(
    json!({ "series": [{ "type": "scatter", "encode": { "x": 0, "y": 1 } }, { "type": "scatter", "encode": { "x": 2, "y": 3 } }] }),
    4
)]
#[case::radar(json!({ "series": [{ "type": "radar" }] }), 6)]
fn used_columns_counts_the_prefix_the_series_read(#[case] v: Value, #[case] expected: usize) {
    assert_that!(option(v).used_columns(&HEADERS)).is_equal_to(expected);
}

#[test]
fn used_columns_never_exceeds_the_table() {
    let o = option(json!({ "series": [{ "type": "line", "encode": { "y": 40 } }] }));
    assert_that!(o.used_columns(&HEADERS)).is_equal_to(HEADERS.len());
}

#[test]
fn used_columns_without_series_is_zero() {
    let o = option(json!({ "series": [] }));
    assert_that!(o.used_columns(&HEADERS)).is_equal_to(0);
}
