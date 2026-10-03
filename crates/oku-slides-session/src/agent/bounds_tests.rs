use rstest::rstest;
use speculoos::prelude::*;

use super::{TEXT_MAX, check_hex, check_text};

#[test]
fn check_text_at_the_limit_passes() {
    assert_that!(check_text("title", &"x".repeat(TEXT_MAX), TEXT_MAX)).is_ok();
}

#[test]
fn check_text_over_the_limit_is_refused() {
    assert_that!(check_text("title", &"x".repeat(TEXT_MAX + 1), TEXT_MAX)).is_err();
}

#[test]
fn check_text_with_a_bidi_override_is_refused() {
    assert_that!(check_text("label", "Annuler\u{202E}tuot emirppuS", TEXT_MAX)).is_err();
}

#[rstest]
#[case("র\u{200D}্য")]
#[case("👩\u{200D}💻")]
#[case("\u{200C}")]
fn check_text_keeps_joiners_used_by_scripts_and_emoji(#[case] text: &str) {
    assert_that!(check_text("label", text, TEXT_MAX)).is_ok();
}

#[rstest]
#[case("a\u{E0041}")]
#[case("a\u{061C}")]
#[case("a\u{00AD}")]
#[case("a\u{180E}")]
fn check_text_refuses_tag_characters_and_other_invisibles(#[case] text: &str) {
    assert_that!(check_text("label", text, TEXT_MAX)).is_err();
}

#[rstest]
#[case("#00154e")]
#[case("#abc")]
#[case("#abcd")]
#[case("#00154eff")]
fn check_hex_accepts_hex_colors(#[case] color: &str) {
    assert_that!(check_hex("swatch", color)).is_ok();
}

#[rstest]
#[case("url(https://example.com)")]
#[case("00154e")]
#[case("#12")]
#[case("#ggg")]
fn check_hex_refuses_anything_else(#[case] color: &str) {
    assert_that!(check_hex("swatch", color)).is_err();
}
