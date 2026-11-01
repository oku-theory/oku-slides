//! t-1379: a line separator or a C0 control in an interpolated argument reached native
//! dialogs (a forged file name injecting lines) and terminals (ESC sequences).

use oku_slides_i18n::{FluentValue, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

#[rstest]
#[case::line_feed("a\nb")]
#[case::carriage_return("a\rb")]
#[case::next_line("a\u{0085}b")]
#[case::line_separator("a\u{2028}b")]
#[case::paragraph_separator("a\u{2029}b")]
#[case::tab("a\tb")]
#[case::escape("a\u{1B}b")]
#[case::delete("a\u{7F}b")]
fn tr_with_control_in_argument_becomes_a_space(#[case] path: &str) {
    let l = Localizer::new(Some("en"));
    let shown = l.tr_with("cli-reading", &[("path", FluentValue::from(path))]);
    assert_that!(shown).is_equal_to(l.tr_with("cli-reading", &[("path", FluentValue::from("a b"))]));
}
