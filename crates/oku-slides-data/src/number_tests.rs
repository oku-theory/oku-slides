use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::grouping_thousands("1,000", None)]
#[case::grouping_negative("-2,500", None)]
#[case::grouping_plus("+1,000", None)]
#[case::decimal_comma("1,5", Some(1.5))]
#[case::leading_zero_comma("0,125", Some(0.125))]
#[case::four_decimals("1,0000", Some(1.0))]
#[case::two_decimals("12,50", Some(12.5))]
#[case::grouping_and_point("1,000.5", None)]
#[case::two_points("1.2.3", None)]
#[case::two_commas("1,2,3", None)]
#[case::fifteen_digits("900719925474099", Some(900_719_925_474_099.0))]
#[case::sixteen_digits("9007199254740993", None)]
#[case::fifteen_after_zeros("0.000123456789012345", Some(0.000_123_456_789_012_345))]
#[case::sixteen_after_zeros("0.0001234567890123456", None)]
#[case::exponent("1e5", None)]
#[case::inf("inf", None)]
#[case::nan("NaN", None)]
#[case::unicode_minus("\u{2212}1", None)]
#[case::leading_space(" 1", None)]
#[case::trailing_space("1 ", None)]
#[case::empty("", None)]
#[case::sign_only("-", None)]
#[case::point_only(".", None)]
#[case::plus("+1", Some(1.0))]
fn parse_number_follows_the_number_rules(#[case] text: &str, #[case] expected: Option<f64>) {
    assert_that!(parse_number(text)).is_equal_to(expected);
}

#[test]
fn parse_number_keeps_negative_zero() {
    let zero = parse_number("-0");
    assert_that!(zero).is_equal_to(Some(0.0));
    assert_that!(zero.is_some_and(f64::is_sign_negative)).is_true();
}

#[rstest]
#[case::negative_zero(-0.0, "0")]
#[case::whole(1.0, "1")]
#[case::large(1e20, "100000000000000000000")]
#[case::fraction(0.1, "0.1")]
fn format_number_uses_display_without_trailing_zero(#[case] n: f64, #[case] expected: &str) {
    assert_that!(format_number(n)).is_equal_to(expected.to_owned());
}
