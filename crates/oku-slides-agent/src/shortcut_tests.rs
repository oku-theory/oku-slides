use oku_slides_i18n::Localizer;
use rstest::rstest;
use speculoos::prelude::*;

use super::{THEME_SHORTCUT, matches_shortcut, normalize};

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
    assert_that!(matches_shortcut(text, THEME_SHORTCUT, &Localizer::new(Some(locale)))).is_equal_to(expected);
}

#[test]
fn matches_shortcut_over_long_prompt_never_matches() {
    let long = format!("theme{}", " ".repeat(100));
    assert_that!(matches_shortcut(&long, THEME_SHORTCUT, &Localizer::new(Some("en")))).is_false();
}
