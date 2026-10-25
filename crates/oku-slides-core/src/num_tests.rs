use rstest::rstest;
use speculoos::prelude::*;

use super::{byte, count_f64, floor_i32, int_f64, narrow_f32, round_i64, round_u32, size_f64, trunc_u32, trunc_usize};

#[rstest]
#[case(f64::NAN, 0)]
#[case(-3.7, 0)]
#[case(2.5, 3)]
#[case(2.4, 2)]
#[case(1e12, u32::MAX)]
#[case(f64::INFINITY, u32::MAX)]
fn round_u32_rounds_half_away_and_saturates(#[case] v: f64, #[case] expected: u32) {
    assert_that!(round_u32(v)).is_equal_to(expected);
}

#[rstest]
#[case(f64::NAN, 0)]
#[case(-2.5, -3)]
#[case(2.5, 3)]
#[case(1e300, i64::MAX)]
#[case(-1e300, i64::MIN)]
fn round_i64_rounds_half_away_and_saturates(#[case] v: f64, #[case] expected: i64) {
    assert_that!(round_i64(v)).is_equal_to(expected);
}

#[rstest]
#[case(f64::NAN, 0)]
#[case(-0.5, -1)]
#[case(2.9, 2)]
#[case(1e300, i32::MAX)]
#[case(-1e300, i32::MIN)]
fn floor_i32_rounds_down_and_saturates(#[case] v: f64, #[case] expected: i32) {
    assert_that!(floor_i32(v)).is_equal_to(expected);
}

#[rstest]
#[case(f64::NAN, 0)]
#[case(-3.7, 0)]
#[case(2.9, 2)]
#[case(1e12, u32::MAX)]
fn trunc_u32_truncates_toward_zero_and_saturates(#[case] v: f64, #[case] expected: u32) {
    assert_that!(trunc_u32(v)).is_equal_to(expected);
}

#[rstest]
#[case(f64::NAN, 0)]
#[case(-3.7, 0)]
#[case(2.9, 2)]
#[case(f64::INFINITY, usize::MAX)]
fn trunc_usize_truncates_toward_zero_and_saturates(#[case] v: f64, #[case] expected: usize) {
    assert_that!(trunc_usize(v)).is_equal_to(expected);
}

#[rstest]
#[case(f64::NAN, 0)]
#[case(-4.0, 0)]
#[case(127.5, 128)]
#[case(300.0, 255)]
fn byte_rounds_and_clamps_to_a_channel(#[case] v: f64, #[case] expected: u8) {
    assert_that!(byte(v)).is_equal_to(expected);
}

#[test]
fn narrow_f32_overflow_gives_signed_infinity() {
    assert_that!(narrow_f32(1e300)).is_equal_to(f32::INFINITY);
    assert_that!(narrow_f32(-1e300)).is_equal_to(f32::NEG_INFINITY);
}

#[test]
fn to_float_small_integers_are_exact() {
    assert_that!(count_f64(42)).is_equal_to(42.0);
    assert_that!(size_f64(1 << 40)).is_equal_to(1_099_511_627_776.0);
    assert_that!(int_f64(-7)).is_equal_to(-7.0);
}
