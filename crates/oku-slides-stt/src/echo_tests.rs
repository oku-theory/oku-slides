use rstest::rstest;
use speculoos::prelude::*;

use super::{repeats, unspaced};

#[rstest]
#[case::en_exact_run("Slide added", "Slide added.", true)]
#[case::en_partial_run("added with the blue", "Slide added with the blue theme.", true)]
#[case::en_one_shared_word("slide", "Slide added.", false)]
#[case::en_words_out_of_order("added slide", "Slide added.", false)]
#[case::en_extra_word_of_the_user("slide added in red", "Slide added.", false)]
#[case::fr_accents_folded("diapositive ajoutee", "Diapositive ajoutée sur le thème bleu.", true)]
#[case::ja_substring("スライドを追加", "スライドを追加しました。", true)]
#[case::ja_two_chars_too_short("追加", "スライドを追加しました。", false)]
#[case::unrelated("make the title red", "Slide added.", false)]
#[case::empty("", "Slide added.", false)]
fn repeats_table_matches_runs_of_speech(#[case] said: &str, #[case] spoken: &str, #[case] expected: bool) {
    assert_that!(repeats(said, spoken)).is_equal_to(expected);
}

#[test]
fn repeats_one_word_reply_to_one_word_speech_is_not_an_echo() {
    assert_that!(repeats("Done", "Done.")).is_false();
}

#[rstest]
#[case::latin("Slide added", false)]
#[case::japanese("追加しました", true)]
#[case::thai("เพิ่มสไลด์แล้ว", true)]
#[case::korean_is_spaced("슬라이드 추가", false)]
fn unspaced_detects_cjk_and_thai(#[case] text: &str, #[case] expected: bool) {
    assert_that!(unspaced(text)).is_equal_to(expected);
}
