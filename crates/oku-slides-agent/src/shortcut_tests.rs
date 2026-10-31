use oku_slides_i18n::Localizer;
use rstest::rstest;
use speculoos::prelude::*;

use super::{dictation_locale, is_spoken_theme_shortcut, normalize};

#[rstest]
#[case::accent_and_period("Thème.", "theme")]
#[case::caps_and_spaces("  THÈMES ! ", "themes")]
#[case::guillemets("«thème»", "theme")]
#[case::curly_quote("thema’s", "thema's")]
#[case::inner_spaces("chủ   đề", "chu đe")]
#[case::devanagari_danda("थीम।", "थीम")]
fn normalize_folds_case_accents_quotes_and_edges(#[case] text: &str, #[case] expected: &str) {
    assert_that!(normalize(text)).is_equal_to(expected.to_owned());
}

#[rstest]
#[case::french("fr", "Thème.", true)]
#[case::french_plural("fr", "thèmes", true)]
#[case::french_bare("fr", "THEME", true)]
#[case::sentence("fr", "change le thème en sombre", false)]
#[case::empty("fr", "", false)]
#[case::blank("fr", "   ", false)]
#[case::ellipsis("fr", "…", false)]
#[case::german("de", "Themen", true)]
#[case::japanese("ja", "テーマ。", true)]
#[case::hindi("hi", "थीम", true)]
#[case::hindi_other_word("hi", "थम", false)]
#[case::thai("th", "ธีม", true)]
#[case::dutch_curly_quote("nl", "thema’s", true)]
#[case::english_in_russian("ru", "Theme", true)]
fn matches_shortcut_accepts_only_the_whole_localized_word(
    #[case] locale: &str,
    #[case] text: &str,
    #[case] expected: bool,
) {
    assert_that!(is_spoken_theme_shortcut(text, &Localizer::new(Some(locale)), || None)).is_equal_to(expected);
}

#[test]
fn matches_shortcut_over_long_prompt_never_matches() {
    let long = format!("theme{}", " ".repeat(100));
    assert_that!(is_spoken_theme_shortcut(&long, &Localizer::new(Some("en")), || None)).is_false();
}

#[rstest]
#[case::dictated_german("Themen", Some("de"), true)]
#[case::no_dictation_language("Themen", None, false)]
#[case::interface_word_still_counts("thème", Some("de"), true)]
#[case::other_language_word("テーマ", Some("de"), false)]
fn is_spoken_theme_shortcut_accepts_the_dictation_language(
    #[case] text: &str,
    #[case] dictation: Option<&'static str>,
    #[case] expected: bool,
) {
    let fr = Localizer::new(Some("fr"));
    assert_that!(is_spoken_theme_shortcut(text, &fr, || dictation)).is_equal_to(expected);
}

#[test]
fn is_spoken_theme_shortcut_matching_the_interface_never_asks_the_dictation() {
    let fr = Localizer::new(Some("fr"));
    let asked = std::cell::Cell::new(false);
    let found = is_spoken_theme_shortcut("thème", &fr, || {
        asked.set(true);
        Some("de")
    });
    assert_that!(found).is_true();
    assert_that!(asked.get()).is_false();
}

#[rstest]
#[case::chosen_wins(Some("de-DE"), Some("ja"), Some("de"))]
#[case::empty_choice_is_auto(Some(""), Some("ja"), Some("ja"))]
#[case::detected(None, Some("pt-BR"), Some("pt"))]
#[case::unshipped_choice_has_no_fallback(Some("sv"), Some("ja"), None)]
#[case::nothing(None, None, None)]
fn dictation_locale_takes_the_chosen_then_the_detected_language(
    #[case] chosen: Option<&str>,
    #[case] detected: Option<&str>,
    #[case] expected: Option<&str>,
) {
    assert_that!(dictation_locale(chosen, detected)).is_equal_to(expected);
}
