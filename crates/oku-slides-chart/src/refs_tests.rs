use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{ChartError, apply_option_patch, parse_option, parse_table, require_data_in_chart, require_data_in_table};

use super::check_ref_and_inline;

/// The French message of `e`, against the French text of `key`.
fn says(e: &ChartError, key: &str) -> (String, String) {
    let fr = Localizer::new(Some("fr"));
    (e.localize(&fr), fr.tr(key))
}

fn bars() -> Value {
    json!({ "data": "d1", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] })
}

#[test]
fn parse_option_with_null_data_is_refused() {
    let e = parse_option(&json!({ "data": null, "series": [{ "type": "bar" }] })).unwrap_err();
    let (got, want) = says(&e, "chart-ref-null");
    assert_that!(got).is_equal_to(want);
}

#[test]
fn parse_table_with_null_data_is_refused() {
    let e = parse_table(&json!({ "data": null })).unwrap_err();
    let (got, want) = says(&e, "chart-ref-null");
    assert_that!(got).is_equal_to(want);
}

#[test]
fn option_patch_with_null_data_is_refused() {
    let chart = parse_option(&bars()).unwrap();
    let e = apply_option_patch(&chart, &json!({ "data": null })).unwrap_err();
    let (got, want) = says(&e, "chart-ref-null");
    assert_that!(got).is_equal_to(want);
}

#[test]
fn parse_option_with_ref_only_checks_the_shape() {
    let chart = parse_option(&bars()).unwrap();
    assert_that!(chart.data.as_ref().map(|d| d.as_str().to_owned())).is_equal_to(Some("d1".to_owned()));
}

#[test]
fn parse_option_with_ref_only_and_no_series_is_refused() {
    let e = parse_option(&json!({ "data": "d1", "series": [] })).unwrap_err();
    let (got, want) = says(&e, "chart-no-series");
    assert_that!(got).is_equal_to(want);
}

#[rstest]
#[case::without_data(json!({ "columns": ["c1"] }))]
#[case::empty(json!({ "data": "d1", "columns": [] }))]
#[case::repeated(json!({ "data": "d1", "columns": ["c1", "c1"] }))]
#[case::malformed(json!({ "data": "d1", "columns": ["c 1"] }))]
fn parse_table_with_bad_columns_is_refused(#[case] v: Value) {
    let e = parse_table(&v).unwrap_err();
    let (got, want) = says(&e, "chart-columns-invalid");
    assert_that!(got).is_equal_to(want);
}

#[test]
fn parse_table_with_ref_only_keeps_display_options() {
    let t = parse_table(&json!({ "data": "d1", "columns": ["c2", "c1"], "align": ["end"], "striped": true })).unwrap();
    assert_that!((t.dataset.is_none(), t.align.len(), t.striped)).is_equal_to((true, 1, true));
}

#[test]
fn require_data_without_ref_or_inline_is_refused() {
    let table = parse_table(&json!({ "striped": true })).unwrap();
    let chart = parse_option(&json!({ "series": [{ "type": "bar" }] })).unwrap();
    assert_that!(require_data_in_table(&table)).is_err();
    assert_that!(require_data_in_chart(&chart)).is_err();
}

#[rstest]
#[case::dataset(json!({ "data": "d1", "dataset": { "source": [["A"]] } }), true)]
#[case::series_data(json!({ "data": "d1", "series": [{ "type": "bar", "data": [1] }] }), true)]
#[case::ref_only(json!({ "data": "d1", "series": [{ "type": "bar" }] }), false)]
#[case::inline_only(json!({ "dataset": { "source": [["A"]] } }), false)]
fn check_ref_and_inline_refuses_both(#[case] v: Value, #[case] refused: bool) {
    let r = check_ref_and_inline(&v);
    assert_that!(r.is_err()).is_equal_to(refused);
    if let Err(e) = r {
        let (got, want) = says(&e, "chart-ref-and-inline");
        assert_that!(got).is_equal_to(want);
    }
}
