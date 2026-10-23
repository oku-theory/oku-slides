//! Ops sur le deck et le type des slides ; forme canonique et JSON.

use oku_slides_core::kinds::base;
use oku_slides_core::kinds::essentiel::CONTENT;
use oku_slides_core::*;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::registry;
use crate::*;

// ---- set_deck_param, set_kind ----------------------------------------------------------

#[test]
fn set_deck_param_sets_title() {
    let mut deck = fixture();
    apply(&mut deck, Op::SetDeckParam { key: "title".into(), value: json!(" Nouveau ") }).unwrap();
    assert_that!(deck.params.title).is_equal_to(Some("Nouveau".to_owned()));
}

#[rstest]
#[case("")]
#[case("   ")]
fn set_deck_param_blank_author_stores_none(#[case] blank: &str) {
    let mut deck = fixture();
    apply(&mut deck, Op::SetDeckParam { key: "author".into(), value: json!(blank) }).unwrap();
    assert_that!(deck.params.author).is_none();
}

#[test]
fn set_kind_keeps_params() {
    let mut deck = fixture();
    let before = deck.slide("s-ch1").unwrap().params.clone();
    apply(&mut deck, Op::SetKind { target: slide("s-ch1"), kind: CONTENT.into() }).unwrap();
    let s = deck.slide("s-ch1").unwrap();
    assert_that!(s.kind.as_str()).is_equal_to(CONTENT);
    assert_that!(s.params).is_equal_to(before);
}

#[test]
fn set_kind_on_element_sets_the_kind() {
    let mut deck = fixture();
    apply(&mut deck, Op::SetKind { target: at("s-constat", "b-points"), kind: base::CARD.into() }).unwrap();
    let e = deck.slide("s-constat").unwrap().locate("b-points").unwrap().element;
    assert_that!(e.kind.as_str()).is_equal_to(base::CARD);
}

#[test]
fn set_kind_to_slide_widget_on_element_returns_wrong_category() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, Op::SetKind { target: at("s-constat", "b-points"), kind: CONTENT.into() });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::WrongCategory);
    assert_that!(err.subject()).is_equal_to(Some(CONTENT));
}

#[test]
fn set_kind_to_unknown_widget_returns_unknown_widget() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, Op::SetKind { target: slide("s-ch1"), kind: "mystere".into() });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownWidget);
}

// ---- Canonique, JSON, schéma ---------------------------------------------------------------

#[test]
fn apply_all_add_slide_with_blank_prose_returns_canonical_deck() {
    let mut deck = fixture();
    let mut slide = Slide::new("s-neuve", CONTENT);
    slide.params.set("title", ParamValue::Text(" ".into()));
    slide.notes = "\n".into();
    apply_all(&mut deck, vec![Op::AddSlide { index: None, defaults: false, slide }], &registry()).unwrap();
    let added = deck.slide("s-neuve").unwrap();
    assert_that!(added.params.get("title")).is_none();
    assert_that!(added.notes).is_empty();
    let mut canonical = deck.clone();
    canonicalize(&mut canonical);
    assert_that!(deck).is_equal_to(canonical);
}

#[test]
fn deck_to_json_then_from_json_returns_equal_valid_deck() {
    let deck = fixture();
    let again = Deck::from_json(&deck.to_json().unwrap()).unwrap();
    assert_that!(deck).is_equal_to(&again);
    assert_that!(validate(&deck, &registry(), None)).is_empty();
}
