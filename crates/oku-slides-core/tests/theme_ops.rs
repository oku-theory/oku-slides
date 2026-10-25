//! `set_theme` : le thème du deck et la fermeture de ses `extends` (t-1095).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::collections::BTreeMap;

use common::registry;
use oku_slides_core::*;
use rstest::rstest;
use speculoos::prelude::*;

fn ns(s: &str) -> Namespace {
    Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"))
}

fn pin(extends: &[&str]) -> ThemePin {
    let version = PackVersion::from_hex(&"d".repeat(64)).unwrap_or_else(|| panic!("hex invalide"));
    ThemePin { id: uuid::Uuid::from_u128(9), version, extends: extends.iter().map(|n| ns(n)).collect() }
}

fn pins(entries: &[(&str, &[&str])]) -> BTreeMap<Namespace, ThemePin> {
    entries.iter().map(|(n, e)| (ns(n), pin(e))).collect()
}

fn set_theme(theme: Option<&str>, themes: BTreeMap<Namespace, ThemePin>) -> Op {
    Op::SetTheme { theme: theme.map(ns), themes }
}

#[test]
fn set_theme_with_its_closure_pins_theme_and_parents() {
    let mut deck = Deck::empty();
    let themes = pins(&[("marque", &["papier"]), ("papier", &[])]);
    set_theme(Some("marque"), themes.clone()).apply(&mut deck, &registry()).unwrap();
    assert_that!((deck.theme.clone(), deck.themes.clone())).is_equal_to((Some(ns("marque")), themes));
}

#[rstest]
#[case::cycle(OpErrorKind::ThemeCycle, pins(&[("a", &["b"]), ("b", &["a"])]))]
#[case::closure(OpErrorKind::ThemeClosure, pins(&[("a", &[]), ("b", &[])]))]
fn set_theme_bad_closure_is_refused(#[case] kind: OpErrorKind, #[case] themes: BTreeMap<Namespace, ThemePin>) {
    let mut deck = Deck::empty();
    let err = set_theme(Some("a"), themes).apply(&mut deck, &registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(kind);
    assert_that!(deck).is_equal_to(Deck::empty());
}

#[test]
fn set_theme_missing_parent_names_it() {
    let mut deck = Deck::empty();
    let err = set_theme(Some("a"), pins(&[("a", &["b"])])).apply(&mut deck, &registry()).unwrap_err();
    assert_that!((err.kind(), err.subject())).is_equal_to((OpErrorKind::ThemeClosure, Some("b")));
}

#[test]
fn set_theme_none_with_pins_is_refused() {
    let mut deck = Deck::empty();
    let err = set_theme(None, pins(&[("a", &[])])).apply(&mut deck, &registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ThemeClosure);
}

#[test]
fn set_theme_none_clears_the_pins() {
    let mut deck = Deck::empty();
    set_theme(Some("a"), pins(&[("a", &[])])).apply(&mut deck, &registry()).unwrap();
    set_theme(None, BTreeMap::new()).apply(&mut deck, &registry()).unwrap();
    assert_that!((deck.theme.clone(), deck.themes.is_empty())).is_equal_to((None, true));
}

#[test]
fn set_theme_without_themes_reads_as_an_empty_map() {
    let op: Op = serde_json::from_value(serde_json::json!({"op": "set_theme", "theme": null})).unwrap();
    assert_that!(op).is_equal_to(set_theme(None, BTreeMap::new()));
}

#[test]
fn deck_version_nine_reads_and_ten_is_refused() {
    assert_that!(Deck::from_json(r#"{"version": 9, "slides": []}"#).map(|d| d.version).ok()).is_equal_to(Some(9));
    assert_that!(Deck::from_json(r#"{"version": 10, "slides": []}"#).is_err()).is_true();
}
