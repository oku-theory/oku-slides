use rstest::rstest;
use speculoos::prelude::*;

use oku_slides_a2ui::ChoiceOption;

use super::{LABEL_MAX, TEXT_MAX, check_hex, check_options, check_text};

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

fn option(id: &str, label: &str) -> ChoiceOption {
    ChoiceOption { id: id.into(), label: label.into(), hint: None }
}

#[test]
fn check_options_with_distinct_ids_passes() {
    assert_that!(check_options("options", &[option("a", "A"), option("b", "B")])).is_ok();
}

#[test]
fn check_options_with_a_duplicate_id_is_refused() {
    let err = check_options("options", &[option("a", "A"), option("b", "B"), option("a", "Autre")]).unwrap_err();
    assert_that!(err.one_line().contains("options[2]")).is_true();
}

#[test]
fn check_options_with_a_too_long_label_is_refused() {
    let long = "x".repeat(LABEL_MAX + 1);
    assert_that!(check_options("options", &[option("a", "A"), option("b", &long)])).is_err();
}

#[test]
fn check_options_with_a_too_long_id_or_hint_is_refused() {
    let long = "x".repeat(LABEL_MAX + 1);
    assert_that!(check_options("options", &[option("a", "A"), option(&long, "B")])).is_err();
    let hinted = ChoiceOption { id: "b".into(), label: "B".into(), hint: Some("h".repeat(TEXT_MAX + 1)) };
    assert_that!(check_options("options", &[option("a", "A"), hinted])).is_err();
}
