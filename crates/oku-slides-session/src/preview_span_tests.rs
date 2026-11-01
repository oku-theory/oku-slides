use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::full_12(PreviewSpan::Full, 12, 12)]
#[case::half_12(PreviewSpan::Half, 12, 6)]
#[case::third_12(PreviewSpan::Third, 12, 4)]
#[case::half_odd(PreviewSpan::Half, 5, 3)]
#[case::third_odd(PreviewSpan::Third, 5, 2)]
#[case::third_of_one(PreviewSpan::Third, 1, 1)]
fn preview_span_columns_rounds_up_and_keeps_one(#[case] span: PreviewSpan, #[case] grid: u32, #[case] cols: u32) {
    assert_that!(span.columns(grid)).is_equal_to(cols);
}

#[test]
fn preview_span_default_is_full() {
    assert_that!(PreviewSpan::default()).is_equal_to(PreviewSpan::Full);
}

#[test]
fn preview_span_serializes_snake_case() {
    assert_that!(serde_json::to_value(PreviewSpan::Third).ok()).is_equal_to(Some(serde_json::json!("third")));
}
