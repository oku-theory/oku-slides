use oku_slides_core::{Deck, PackFiles, PackVersion, ThemePin};
use speculoos::prelude::*;

use super::deck_tokens;
use crate::import::ImportErrorKind;
use crate::test_support::ns;
use crate::{Localize, Localizer, PRO_THEME};

/// Des sources vides dont le deck épingle Pro dans la version `version`, et qui embarquent `files`.
fn bundle_pinning(files: PackFiles, version: Option<PackVersion>) -> crate::SourceBundle {
    let mut bundle = crate::SourceBundle::new(Deck::empty(), crate::test_support::registry()).unwrap();
    let embedded = bundle.add_theme(files);
    let mut deck = bundle.deck().clone();
    deck.theme = Some(ns("pro"));
    let pin = ThemePin { id: PRO_THEME, version: version.unwrap_or(embedded), extends: Vec::new() };
    deck.themes = [(ns("pro"), pin)].into();
    bundle.apply_silent(deck).unwrap();
    bundle
}

#[test]
fn deck_tokens_of_an_unreadable_embedded_theme_is_refused() {
    let serde_json::Value::Object(manifest) = serde_json::json!({ "name": 3 }) else { unreachable!() };
    let bundle = bundle_pinning(PackFiles::theme(manifest), None);
    let err = deck_tokens(&crate::ThemeLibrary::builtin_only(), &bundle).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::BadTheme);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn deck_tokens_of_a_theme_missing_from_the_sources_is_refused() {
    let absent = PackVersion::from_hex(&"2".repeat(64)).unwrap();
    let bundle = bundle_pinning(PackFiles::theme(Default::default()), Some(absent));
    let err = deck_tokens(&crate::ThemeLibrary::builtin_only(), &bundle).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::BadTheme);
}
