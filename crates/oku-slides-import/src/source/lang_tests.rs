use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{item, para, run_with, source_deck, source_slide, text_box};

#[rstest]
#[case('a', Script::Latin)]
#[case('é', Script::Latin)]
#[case('漢', Script::EastAsian)]
#[case('か', Script::EastAsian)]
#[case('한', Script::EastAsian)]
#[case('。', Script::EastAsian)]
#[case('ع', Script::Complex)]
#[case('ש', Script::Complex)]
#[case('ก', Script::Complex)]
#[case('क', Script::Complex)]
fn script_of_char_follows_unicode_blocks(#[case] c: char, #[case] expected: Script) {
    assert_that!(Script::of(c)).is_equal_to(expected);
}

#[rstest]
#[case("en-US", Some("en-US"))]
#[case("fr", Some("fr"))]
#[case("ZH-hant-tw", Some("zh-Hant-TW"))]
#[case("ar-SA", Some("ar-SA"))]
#[case("es-419", Some("es"))]
#[case("x-none", None)]
#[case("", None)]
#[case("english", None)]
fn lang_tag_keeps_what_typst_accepts(#[case] raw: &str, #[case] expected: Option<&str>) {
    assert_that!(lang_tag(raw).as_deref()).is_equal_to(expected);
}

fn lang_run(text: &str, lang: Option<&str>) -> crate::source::Run {
    run_with(text, |s| s.lang = lang.map(str::to_owned))
}

#[test]
fn language_counts_slide_characters_per_tag() {
    let tb = text_box(vec![para(vec![
        lang_run("Bonjour à tous", Some("fr-FR")),
        lang_run("Hello", Some("en-US")),
        lang_run("sans langue du tout", None),
    ])]);
    let deck = source_deck(vec![source_slide(1, None, vec![item("s1/2", (0.0, 0.0, 10.0, 10.0), ItemKind::Text(tb))])]);
    assert_that!(deck.language().as_deref()).is_equal_to(Some("fr-FR"));
}

#[test]
fn language_without_tagged_text_is_none() {
    let tb = text_box(vec![para(vec![lang_run("texte", Some("x-none"))])]);
    let deck = source_deck(vec![source_slide(1, None, vec![item("s1/2", (0.0, 0.0, 10.0, 10.0), ItemKind::Text(tb))])]);
    assert_that!(deck.language()).is_none();
}
