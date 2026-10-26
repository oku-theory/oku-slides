use rstest::rstest;
use speculoos::prelude::*;

use oku_slides_a2ui::ChoiceOption;

use super::{LABEL_MAX, TEXT_MAX, check_hex, check_options, check_text, clip, label};

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

#[test]
fn clip_short_text_is_kept() {
    assert_that!(clip("{\n  \"slide\": 4\n}", 40)).is_equal_to("{\n  \"slide\": 4\n}".to_owned());
}

#[test]
fn clip_long_text_is_cut_with_an_ellipsis() {
    assert_that!(clip("abcdef", 4)).is_equal_to("abc…".to_owned());
}

/// Un détail d'outil vient de l'agent : ses caractères invisibles ne s'affichent pas.
#[test]
fn clip_drops_hidden_controls() {
    assert_that!(clip("a\u{202E}b\u{E0041}c", 10)).is_equal_to("abc".to_owned());
}

#[test]
fn clip_text_of_exactly_max_chars_is_kept_whole() {
    assert_that!(clip("abcd", 4)).is_equal_to("abcd".to_owned());
}

#[test]
fn clip_to_zero_chars_is_empty() {
    assert_that!(clip("abc", 0)).is_equal_to(String::new());
}

/// Every `Pi` and `Pf` quotation mark, the ASCII, full-width and CJK quotes and the backtick.
const QUOTES: &str = "\"'`«»‘’‚‛“”„‟‹›⸂⸃⸄⸅⸉⸊⸌⸍⸜⸝⸠⸡＂＇「」『』〝〞〟";

/// FSI, PDI and every bidirectional control.
const BIDI: &str = "\u{061C}\u{200E}\u{200F}\u{202A}\u{202B}\u{202C}\u{202D}\u{202E}\u{2066}\u{2067}\u{2068}\u{2069}";

#[test]
fn label_output_holds_no_quote_or_bidi_control() {
    // One character per call: a long input would be clipped before its last characters.
    let leaked: Vec<char> = QUOTES
        .chars()
        .chain(BIDI.chars())
        .filter(|c| label(&format!("a{c}b")).chars().any(|o| QUOTES.contains(o) || BIDI.contains(o)))
        .collect();
    assert_that!(leaked).is_empty();
}

#[test]
fn label_maps_every_quote_to_the_modifier_apostrophe() {
    let unmapped: Vec<char> = QUOTES.chars().filter(|c| label(&format!("a{c}b")) != "a\u{02BC}b").collect();
    assert_that!(unmapped).is_empty();
}

#[rstest]
#[case::plain("Deck", "Deck")]
#[case::newline("a\nb", "a b")]
#[case::escape("a\u{1B}b", "a b")]
#[case::delete("a\u{7F}b", "a b")]
#[case::nel("a\u{85}b", "a b")]
#[case::csi("a\u{9B}b", "a b")]
#[case::line_separator("a\u{2028}b", "a b")]
#[case::paragraph_separator("a\u{2029}b", "a b")]
#[case::right_to_left_override("a\u{202E}b", "ab")]
#[case::combining_grapheme_joiner("a\u{34F}b", "ab")]
#[case::hangul_choseong_filler("a\u{115F}b", "ab")]
#[case::hangul_jungseong_filler("a\u{1160}b", "ab")]
#[case::hangul_filler("a\u{3164}b", "ab")]
#[case::halfwidth_hangul_filler("a\u{FFA0}b", "ab")]
#[case::interlinear_anchor("a\u{FFF9}b", "ab")]
#[case::variation_selector("a\u{FE00}b", "ab")]
#[case::variation_selector_16("a\u{FE0F}b", "ab")]
#[case::variation_selector_supplement("a\u{E0100}b", "ab")]
#[case::braille_blank("a\u{2800}b", "ab")]
#[case::musical_begin_beam("a\u{1D159}b", "ab")]
#[case::musical_begin_tie("a\u{1D173}b", "ab")]
#[case::khmer_inherent_vowel("a\u{17B4}b", "ab")]
#[case::mongolian_variation_selector("a\u{180B}b", "ab")]
#[case::zero_width_non_joiner("a\u{200C}b", "ab")]
#[case::zero_width_joiner("a\u{200D}b", "ab")]
#[case::zero_width_space_between_spaces("a \u{200B} b", "a b")]
#[case::leading_zero_width_space("\u{200B} a", "a")]
#[case::trimmed_and_collapsed("  a \t  b  ", "a b")]
#[case::ascii_double_quote("\"x\"", "\u{2bc}x\u{2bc}")]
#[case::corner_brackets("「x」", "\u{2bc}x\u{2bc}")]
#[case::french_guillemets("«x»", "\u{2bc}x\u{2bc}")]
#[case::curly_double("“x”", "\u{2bc}x\u{2bc}")]
#[case::backtick("`x`", "\u{2bc}x\u{2bc}")]
#[case::all_invisible("\u{200B}\u{2800}", "…")]
#[case::all_spaces("   \n ", "…")]
#[case::empty("", "…")]
fn label_cleans_text(#[case] text: &str, #[case] expected: &str) {
    assert_that!(label(text).as_str()).is_equal_to(expected);
}

#[rstest]
#[case::at_the_limit(LABEL_MAX, "x".repeat(LABEL_MAX))]
#[case::over_the_limit(LABEL_MAX + 1, format!("{}…", "x".repeat(LABEL_MAX - 1)))]
fn label_clips_to_sixty_characters(#[case] len: usize, #[case] expected: String) {
    assert_that!(label(&"x".repeat(len))).is_equal_to(expected);
}
