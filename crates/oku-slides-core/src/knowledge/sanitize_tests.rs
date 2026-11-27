use std::borrow::Cow;

use rstest::rstest;
use speculoos::prelude::*;

use super::{clean_text, escape_inline, escape_specials, is_clean_text};

#[test]
fn clean_text_crlf_and_controls_normalized() {
    assert_that!(clean_text("a\r\nb\rc\u{7}d\te\u{85}f").into_owned()).is_equal_to("a\nb\ncd\tef".to_owned());
}

#[test]
fn clean_text_plain_ascii_borrowed() {
    assert_that!(matches!(clean_text("plain *text*\n\twith tab"), Cow::Borrowed(_))).is_true();
    assert_that!(matches!(clean_text("déjà vu"), Cow::Borrowed(_))).is_true();
}

#[rstest]
#[case::bidi("a\u{202E}b", false)]
#[case::tag("a\u{E0041}b", false)]
#[case::zero_width("a\u{200B}b", false)]
#[case::carriage_return("a\rb", false)]
#[case::nul("a\u{0}b", false)]
#[case::del("a\u{7F}b", false)]
#[case::tab_and_newline("a\tb\nc", true)]
#[case::accents("é ü 漢字", true)]
fn is_clean_text_matches_clean_text(#[case] text: &str, #[case] clean: bool) {
    assert_that!(is_clean_text(text)).is_equal_to(clean);
    assert_that!(clean_text(text) == text).is_equal_to(clean);
    assert_that!(is_clean_text(&clean_text(text))).is_true();
}

#[test]
fn escape_inline_specials_escaped_and_line_ends_spaced() {
    assert_that!(escape_inline("*a*\n[b]\u{202E}|c")).is_equal_to(r"\*a\* \[b\]\|c".to_owned());
}

#[rstest]
#[case("a\r\nb*\u{7}c")]
#[case("x_y | z")]
#[case("\u{2066}<b>\u{2069}")]
fn escape_inline_equals_clean_then_escape(#[case] text: &str) {
    assert_that!(escape_inline(text)).is_equal_to(escape_specials(&clean_text(text)));
}
