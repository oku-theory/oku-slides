use rstest::rstest;
use speculoos::prelude::*;

use super::fold;

#[rstest]
#[case::french_accents("Thème Élève", "theme eleve")]
#[case::decomposed_input("the\u{0300}me", "theme")]
#[case::ligatures("Œuvre Æther", "oeuvre aether")]
#[case::nordic_and_german("Søren Straße", "soren strasse")]
#[case::polish("Zażółć", "zazołc")]
#[case::curly_apostrophe("thema’s", "thema's")]
#[case::greek("Θέμα", "θεμα")]
#[case::cyrillic_short_i("Й", "и")]
#[case::vietnamese("Chủ Đề", "chu đe")]
#[case::devanagari_vowel_kept("थीम", "थीम")]
#[case::hangul_kept("테마", "테마")]
#[case::punctuation_kept("« Thème ! »", "« theme ! »")]
fn fold_drops_case_and_accents_only(#[case] text: &str, #[case] expected: &str) {
    assert_that!(fold(text)).is_equal_to(expected.to_owned());
}
