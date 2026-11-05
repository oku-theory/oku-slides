use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn axis(min: Option<f64>, max: Option<f64>, interval: Option<f64>) -> Axis {
    serde_json::from_value(serde_json::json!({ "min": min, "max": max, "interval": interval }))
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn value_scale_both_bounds_without_interval_keeps_them() {
    let scale = value_scale(Some(&axis(Some(2.0), Some(30.0), None)), -3.0, 17.0);
    assert_that!(scale).is_equal_to(ValueScale { min: Some(2.0), max: Some(30.0), splits: None });
}

#[rstest]
#[case::stack_of_16_5(0.0, 16.5, (0.0, 18.0, 9))]
#[case::ten(0.0, 10.0, (0.0, 12.0, 6))]
#[case::seven(0.0, 7.0, (0.0, 8.0, 8))]
#[case::max_on_a_tick_rises(0.0, 18.0, (0.0, 20.0, 10))]
#[case::hundred(0.0, 100.0, (0.0, 120.0, 6))]
#[case::ninety(0.0, 90.0, (0.0, 100.0, 10))]
#[case::tenths(0.0, 0.9, (0.0, 1.0, 10))]
#[case::hundredths_tick_by_tenths(0.0, 0.42, (0.0, 0.5, 5))]
#[case::padded_top_a_hair_over_a_tick(0.0, 9.523_809_523_809_526, (0.0, 10.0, 10))]
#[case::negative(-16.5, 0.0, (-18.0, 0.0, 9))]
#[case::both_signs(-5.0, 16.5, (-10.0, 20.0, 6))]
fn value_scale_automatic_pads_five_percent_and_ticks_at_most_ten(
    #[case] low: f64,
    #[case] high: f64,
    #[case] expected: (f64, f64, usize),
) {
    let scale = value_scale(None, low, high);
    let (lo, hi, splits) = expected;
    assert_that!((scale.min.unwrap_or(f64::NAN) - lo).abs()).is_less_than(1e-9);
    assert_that!((scale.max.unwrap_or(f64::NAN) - hi).abs()).is_less_than(1e-9);
    assert_that!(scale.splits).is_equal_to(Some(splits));
}

#[test]
fn value_scale_automatic_keeps_a_written_min() {
    let scale = value_scale(Some(&axis(Some(0.0), None, None)), 0.0, 16.5);
    assert_that!(scale).is_equal_to(ValueScale { min: Some(0.0), max: Some(18.0), splits: Some(9) });
}

#[rstest]
#[case::max_on_the_grid(None, Some(20.0), (0.0, 16.5), ValueScale { min: Some(0.0), max: Some(20.0), splits: Some(10) })]
#[case::negative_max_on_the_grid(None, Some(-2.0), (-16.5, 0.0), ValueScale { min: Some(-18.0), max: Some(-2.0), splits: Some(8) })]
#[case::max_off_the_grid(None, Some(17.0), (0.0, 16.5), ValueScale { min: None, max: Some(17.0), splits: None })]
#[case::min_off_the_grid(Some(1.0), None, (0.0, 16.5), ValueScale { min: Some(1.0), max: None, splits: None })]
#[case::min_beyond_the_data(Some(10.0), None, (0.0, 5.0), ValueScale { min: Some(10.0), max: None, splits: None })]
fn value_scale_automatic_with_one_written_bound(
    #[case] min: Option<f64>,
    #[case] max: Option<f64>,
    #[case] data: (f64, f64),
    #[case] expected: ValueScale,
) {
    let scale = value_scale(Some(&axis(min, max, None)), data.0, data.1);
    assert_that!(scale).is_equal_to(expected);
}

#[rstest]
#[case::thousandths(0.004)]
#[case::beyond_f32(3.0e38)]
fn value_scale_automatic_out_of_reach_leaves_the_engine_scale(#[case] high: f64) {
    let scale = value_scale(None, 0.0, high);
    assert_that!(scale).is_equal_to(ValueScale::default());
}

#[test]
fn value_scale_automatic_without_data_leaves_the_engine_scale() {
    let scale = value_scale(None, 0.0, 0.0);
    assert_that!(scale).is_equal_to(ValueScale::default());
}

#[rstest]
#[case::both_bounds(Some(0.0), Some(20.0), 5.0, (0.0, 20.0, 4))]
#[case::max_from_data(None, None, 2.0, (-6.0, 18.0, 12))]
#[case::negative_data(None, Some(12.0), 4.0, (-8.0, 12.0, 5))]
#[case::max_off_the_grid_rises(None, Some(10.0), 4.0, (-8.0, 12.0, 5))]
#[case::partial_last_step_rises(Some(0.0), Some(17.0), 5.0, (0.0, 20.0, 4))]
#[case::min_off_the_grid_counts_from_min(Some(1.0), Some(16.0), 5.0, (1.0, 16.0, 3))]
fn value_scale_with_interval_sets_bounds_and_splits(
    #[case] min: Option<f64>,
    #[case] max: Option<f64>,
    #[case] interval: f64,
    #[case] expected: (f64, f64, usize),
) {
    let scale = value_scale(Some(&axis(min, max, Some(interval))), -5.0, 16.5);
    let (lo, hi, splits) = expected;
    assert_that!(scale).is_equal_to(ValueScale { min: Some(lo), max: Some(hi), splits: Some(splits) });
}

#[rstest]
#[case::negative(None, Some(10.0), 4.0)]
#[case::partial(Some(0.0), Some(17.0), 5.0)]
#[case::float_step(Some(0.0), Some(1.0), 0.1)]
fn value_scale_with_interval_ticks_every_interval_from_min(
    #[case] min: Option<f64>,
    #[case] max: Option<f64>,
    #[case] interval: f64,
) {
    let scale = value_scale(Some(&axis(min, max, Some(interval))), -5.0, 16.5);
    let (lo, hi, n) = (scale.min.unwrap_or_default(), scale.max.unwrap_or_default(), scale.splits.unwrap_or(1));
    let tick = (hi - lo) / f64::from(u32::try_from(n).unwrap_or(1));
    assert_that!((tick - interval).abs()).is_less_than(1e-9);
}

#[rstest]
#[case::too_fine(Some(0.0), Some(100.0), 0.5, 100.0)]
#[case::beyond_f32(None, None, 1e300, 10.0)]
#[case::infinite_steps(None, None, 1e-300, 1e10)]
fn value_scale_interval_out_of_reach_keeps_the_written_bounds(
    #[case] min: Option<f64>,
    #[case] max: Option<f64>,
    #[case] interval: f64,
    #[case] high: f64,
) {
    let scale = value_scale(Some(&axis(min, max, Some(interval))), 0.0, high);
    assert_that!(scale).is_equal_to(ValueScale { min, max, splits: None });
}

#[rstest]
#[case::equal(10.0)]
#[case::inverted(8.0)]
fn value_scale_bounds_not_increasing_span_one_interval(#[case] max: f64) {
    let scale = value_scale(Some(&axis(Some(10.0), Some(max), Some(2.0))), 0.0, 1.0);
    assert_that!(scale).is_equal_to(ValueScale { min: Some(10.0), max: Some(12.0), splits: Some(1) });
}

#[rstest]
#[case::zero(0.0)]
#[case::negative(-1.0)]
fn check_axis_interval_not_positive_is_refused(#[case] interval: f64) {
    let err = check_axis(Some(&axis(None, None, Some(interval)))).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(crate::ChartErrorKind::Inconsistent));
}

#[test]
fn check_axis_positive_interval_is_accepted() {
    assert_that!(check_axis(Some(&axis(None, None, Some(0.25))))).is_ok();
}

#[rstest]
#[case::equal(10.0, None)]
#[case::inverted(8.0, None)]
#[case::equal_with_interval(10.0, Some(2.0))]
fn check_axis_max_not_above_min_is_refused(#[case] max: f64, #[case] interval: Option<f64>) {
    let err = check_axis(Some(&axis(Some(10.0), Some(max), interval))).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(crate::ChartErrorKind::Inconsistent));
}

#[test]
fn check_axis_bounds_refusal_is_localized() {
    use oku_slides_i18n::{Localize, Localizer};
    let fr = Localizer::new(Some("fr"));
    let err = check_axis(Some(&axis(Some(3.0), Some(1.0), None))).err().map(|e| e.localize(&fr));
    assert_that!(err).is_equal_to(Some(Message::new("chart-axis-bounds").localize(&fr)));
}

#[test]
fn stacked_extent_sums_stacks_by_sign_and_keeps_lone_series() {
    let (a, b, c) = ([Some(10.0), None, Some(12.5)], [Some(4.0), Some(-3.0), Some(6.0)], [Some(-7.0), Some(2.0)]);
    let series = [(Some("s"), &a[..]), (Some("s"), &b[..]), (None, &c[..])];
    assert_that!(stacked_extent(series.into_iter())).is_equal_to((-7.0, 18.5));
}

#[test]
fn stacked_extent_opposite_signs_in_one_stack_do_not_cancel() {
    let (a, b) = ([Some(5.0)], [Some(-3.0)]);
    let series = [(Some("s"), &a[..]), (Some("s"), &b[..])];
    assert_that!(stacked_extent(series.into_iter())).is_equal_to((-3.0, 5.0));
}

#[test]
fn stacked_extent_negatives_of_one_stack_add_up() {
    let (a, b) = ([Some(-3.0)], [Some(-4.0)]);
    let series = [(Some("s"), &a[..]), (Some("s"), &b[..])];
    assert_that!(stacked_extent(series.into_iter())).is_equal_to((-7.0, 0.0));
}

#[test]
fn stacked_extent_without_data_is_zero() {
    assert_that!(stacked_extent(std::iter::empty())).is_equal_to((0.0, 0.0));
}
