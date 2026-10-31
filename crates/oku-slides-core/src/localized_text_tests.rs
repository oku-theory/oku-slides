use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;

/// A dictionary of the four tested languages.
fn by_locale() -> LocalizedText {
    serde_json::from_value(json!({"en": "Value", "fr": "Valeur", "zh-Hant": "數值", "pt": "Valor"})).unwrap()
}

#[rstest]
#[case::exact("fr", Some("Valeur"))]
#[case::exact_region("zh-Hant", Some("數值"))]
#[case::case_insensitive("ZH-hant", Some("數值"))]
#[case::language_without_region("pt-BR", Some("Valor"))]
#[case::english_fallback("de", Some("Value"))]
fn localized_text_by_locale_picks_the_closest_entry(#[case] locale: &str, #[case] shown: Option<&str>) {
    assert_that!(by_locale().pick(locale)).is_equal_to(shown);
}

#[test]
fn localized_text_without_english_falls_back_to_the_lowest_code() {
    let t: LocalizedText = serde_json::from_value(json!({"it": "Valore", "fr": "Valeur"})).unwrap();
    assert_that!(t.pick("de")).is_equal_to(Some("Valeur"));
}

#[test]
fn localized_text_region_entry_serves_its_bare_language() {
    let t: LocalizedText = serde_json::from_value(json!({"en": "Value", "zh-Hant": "數值"})).unwrap();
    assert_that!(t.pick("zh")).is_equal_to(Some("Value"));
}

#[test]
fn localized_text_plain_is_shown_in_every_locale() {
    let t: LocalizedText = serde_json::from_value(json!("KPI")).unwrap();
    assert_that!(t.pick("ja")).is_equal_to(Some("KPI"));
}

#[test]
fn localized_text_empty_dictionary_shows_nothing() {
    let t: LocalizedText = serde_json::from_value(json!({})).unwrap();
    assert_that!(t.pick("en")).is_none();
}

#[rstest]
#[case::plain(json!("KPI"))]
#[case::by_locale(json!({"en": "Value", "fr": "Valeur"}))]
fn localized_text_serializes_as_written(#[case] written: serde_json::Value) {
    let t: LocalizedText = serde_json::from_value(written.clone()).unwrap();
    assert_that!(serde_json::to_value(&t).unwrap()).is_equal_to(written);
}

#[test]
fn localized_text_with_a_non_text_entry_is_refused() {
    assert_that!(serde_json::from_value::<LocalizedText>(json!({"en": 42}))).is_err();
}
