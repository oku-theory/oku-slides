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
#[case::hangul_filler('\u{3164}', true)]
#[case::halfwidth_hangul_filler('\u{FFA0}', true)]
#[case::emoji_presentation('\u{FE0F}', false)]
#[case::choseong_filler('\u{115F}', false)]
#[case::zwj('\u{200D}', false)]
#[case::zwnj('\u{200C}', false)]
#[case::letter('é', false)]
fn is_hidden_control_matches_bidi_and_invisible_characters(#[case] c: char, #[case] hidden: bool) {
    assert_that!(is_hidden_control(c)).is_equal_to(hidden);
}

#[test]
fn clean_arg_removes_hidden_controls_and_keeps_joiners() {
    assert_that!(clean_arg("ré\u{202E}cent\u{200B}e 👩\u{200D}💻")).is_equal_to("récente 👩\u{200D}💻".to_owned());
}

#[test]
fn clean_arg_turns_line_controls_into_spaces() {
    assert_that!(clean_arg("a\r\nb\u{2028}c\u{1B}[2Jd")).is_equal_to("a  b c [2Jd".to_owned());
}

#[rstest]
#[case::lf('\n', true)]
#[case::nel('\u{0085}', true)]
#[case::ls('\u{2028}', true)]
#[case::ps('\u{2029}', true)]
#[case::del('\u{7F}', true)]
#[case::space(' ', false)]
#[case::nbsp('\u{00A0}', false)]
fn is_line_control_matches_cc_and_unicode_separators(#[case] c: char, #[case] control: bool) {
    assert_that!(is_line_control(c)).is_equal_to(control);
}
