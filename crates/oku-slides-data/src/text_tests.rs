use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::limits::MAX_LABEL_CHARS;

#[rstest]
#[case::at_bound(MAX_LABEL_CHARS, MAX_LABEL_CHARS)]
#[case::over_bound(MAX_LABEL_CHARS + 1, MAX_LABEL_CHARS)]
fn clean_label_cuts_to_the_label_bound(#[case] len: usize, #[case] expected: usize) {
    assert_that!(clean_label(&"é".repeat(len)).chars().count()).is_equal_to(expected);
}

#[rstest]
#[case::override_mark('\u{202E}')]
#[case::bell('\u{0007}')]
#[case::isolate('\u{2066}')]
#[case::arabic_mark('\u{061C}')]
#[case::rtl_mark('\u{200F}')]
#[case::zero_width_space('\u{200B}')]
#[case::tag('\u{E0041}')]
#[case::line_separator('\u{2028}')]
#[case::newline('\n')]
fn clean_label_removes_controls(#[case] control: char) {
    assert_that!(clean_label(&format!("a{control}b"))).is_equal_to("ab".to_owned());
}

#[test]
fn clean_label_keeps_the_zero_width_joiner() {
    assert_that!(clean_label("a\u{200D}b")).is_equal_to("a\u{200D}b".to_owned());
}

#[test]
fn strip_controls_does_not_cut() {
    let long = "a".repeat(MAX_LABEL_CHARS * 2);
    assert_that!(strip_controls(&format!("{long}\u{202E}"))).is_equal_to(long);
}
