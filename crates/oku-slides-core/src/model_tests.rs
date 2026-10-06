use speculoos::prelude::*;

use super::*;

#[test]
fn from_json_version_1_deck_returns_an_error() {
    let err = Deck::from_json(r#"{"version": 1, "slides": [{"id": "s", "kind": "content", "blocks": []}]}"#);
    assert_that!(err.map_err(|e| e.to_string()).unwrap_err()).contains("unsupported deck version 1");
}

#[test]
fn from_json_newer_version_returns_an_error() {
    assert_that!(Deck::from_json(r#"{"version": 5, "slides": []}"#)).is_err();
}

const REF: &str = r#"{"id":"6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10","version":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;

#[test]
fn deck_from_json_reads_templates_as_a_pack_ref() {
    let deck = Deck::from_json(&format!(r#"{{"version":4,"templates":{REF},"slides":[]}}"#)).unwrap();
    let r = deck.templates.unwrap();
    assert_that!(r.id.to_string()).is_equal_to("6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10".to_owned());
    assert_that!(r.version.hex()).is_equal_to(&*"a".repeat(64));
}

#[test]
fn deck_from_json_with_a_slug_templates_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":4,"templates":"essentiel","slides":[]}"#).is_err()).is_true();
}

#[test]
fn deck_from_json_reads_theme_as_a_theme_ref() {
    let deck = Deck::from_json(&format!(r#"{{"version":4,"theme":{REF},"slides":[]}}"#)).unwrap();
    let r = deck.theme.unwrap();
    assert_that!(r.id.to_string()).is_equal_to("6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10".to_owned());
    assert_that!(r.version.hex()).is_equal_to(&*"a".repeat(64));
}

#[test]
fn deck_from_json_with_a_theme_name_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":4,"theme":"papier","slides":[]}"#).is_err()).is_true();
}

#[test]
fn deck_from_json_version_3_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":3,"slides":[]}"#).is_err()).is_true();
}

#[test]
fn deck_from_json_version_2_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":2,"slides":[]}"#).is_err()).is_true();
}

#[test]
fn from_json_current_version_reads_the_deck() {
    let deck = Deck::from_json(r#"{"version": 4, "slides": [{"id": "s", "kind": "content"}]}"#).unwrap();
    assert_that!(deck.slides.len()).is_equal_to(1);
}

#[test]
fn from_json_without_version_reads_as_current() {
    assert_that!(Deck::from_json(r#"{"slides": []}"#).unwrap().version).is_equal_to(DECK_VERSION);
}

#[rstest::rstest]
#[case("title")]
#[case("subtitle")]
#[case("author")]
#[case("date")]
#[case("organization")]
#[case("lang")]
#[case("toc-title")]
fn deck_params_set_field_then_field_returns_the_text(#[case] key: &str) {
    let mut p = DeckParams::default();
    p.set_field(key, Some("valeur".into()));
    assert_that!(p.field(key)).is_equal_to(Some("valeur"));
    p.set_field(key, None);
    assert_that!(p.field(key)).is_none();
    assert_that!(p).is_equal_to(DeckParams::default());
}

#[test]
fn deck_params_field_of_non_string_extra_returns_none() {
    let mut p = DeckParams::default();
    p.extra.insert("numbering".into(), serde_json::json!(3));
    assert_that!(p.field("numbering")).is_none();
}

#[test]
fn deck_params_is_field_names_the_six_fields_only() {
    for key in ["title", "subtitle", "author", "date", "organization", "lang"] {
        assert_that!(DeckParams::is_field(key)).is_true();
    }
    assert_that!(DeckParams::is_field("numbering")).is_false();
}
