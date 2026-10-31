use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::rlo('\u{202E}', true)]
#[case::lri('\u{2066}', true)]
#[case::fsi('\u{2068}', true)]
#[case::pdi('\u{2069}', true)]
#[case::rlm('\u{200F}', true)]
#[case::alm('\u{061C}', true)]
#[case::zwsp('\u{200B}', true)]
#[case::bom('\u{FEFF}', true)]
#[case::tag('\u{E0041}', true)]
#[case::zwj('\u{200D}', false)]
#[case::zwnj('\u{200C}', false)]
#[case::letter('é', false)]
fn is_hidden_control_matches_bidi_and_invisible_characters(#[case] c: char, #[case] hidden: bool) {
    assert_that!(is_hidden_control(c)).is_equal_to(hidden);
}

#[test]
fn strip_hidden_removes_controls_and_keeps_joiners() {
    assert_that!(strip_hidden("ré\u{202E}cent\u{200B}e 👩\u{200D}💻")).is_equal_to("récente 👩\u{200D}💻".to_owned());
}
