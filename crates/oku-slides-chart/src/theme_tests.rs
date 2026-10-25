use rstest::rstest;
use speculoos::prelude::*;

use super::color;

fn rgba(hex: &str) -> (u8, u8, u8, u8) {
    let c = color(hex);
    (c.r, c.g, c.b, c.a)
}

#[rstest]
#[case::short("#1af", (0x11, 0xaa, 0xff, 255))]
#[case::long_upper("#2F5D8A", (0x2f, 0x5d, 0x8a, 255))]
#[case::with_alpha("#2f5d8a80", (0x2f, 0x5d, 0x8a, 0x80))]
#[case::no_hash_trimmed("  c4622d ", (0xc4, 0x62, 0x2d, 255))]
fn color_valid_hex_reads_every_component(#[case] hex: &str, #[case] expected: (u8, u8, u8, u8)) {
    assert_that!(rgba(hex)).is_equal_to(expected);
}

#[rstest]
#[case::two_byte_char_spans_two_digits("#é1", (0, 0, 17, 255))]
#[case::plus_sign_is_not_a_digit("#+f+f+f", (0, 0, 0, 255))]
#[case::one_bad_component_only("#zz5d8a", (0, 0x5d, 0x8a, 255))]
#[case::short_bad_digit("#1z3", (17, 0, 51, 255))]
fn color_invalid_component_reads_zero(#[case] hex: &str, #[case] expected: (u8, u8, u8, u8)) {
    assert_that!(rgba(hex)).is_equal_to(expected);
}

#[rstest]
#[case::six_digits("#zzzzzz", 255)]
#[case::other_length("#zz", 255)]
#[case::eight_digits("#zzzzzzzz", 0)]
fn color_fully_invalid_is_black_with_length_dependent_alpha(#[case] hex: &str, #[case] alpha: u8) {
    assert_that!(rgba(hex)).is_equal_to((0, 0, 0, alpha));
}
