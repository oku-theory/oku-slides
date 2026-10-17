use oku_slides_core::{Deck, PackFiles, ThemeRef};
use speculoos::prelude::*;

use super::deck_tokens;
use crate::import::ImportErrorKind;
use crate::{Localize, Localizer, PRO_THEME};

/// Des sources vides qui embarquent `files` comme version du thème Pro.
fn bundle_with_theme(files: PackFiles) -> (crate::SourceBundle, ThemeRef) {
    let mut bundle = crate::SourceBundle::new(Deck::empty(), crate::test_support::registry()).unwrap();
    let r = ThemeRef { id: PRO_THEME, version: bundle.add_theme(files) };
    (bundle, r)
}

#[test]
fn deck_tokens_of_an_unreadable_embedded_theme_is_refused() {
    let serde_json::Value::Object(manifest) = serde_json::json!({ "name": 3 }) else { unreachable!() };
    let (bundle, r) = bundle_with_theme(PackFiles::new(manifest, Default::default()));
    let err = deck_tokens(&bundle, &r).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::BadTheme);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn deck_tokens_of_a_theme_missing_from_the_sources_is_refused() {
    let (bundle, mut r) = bundle_with_theme(PackFiles::new(Default::default(), Default::default()));
    r.version =
        PackFiles::new(Default::default(), [(oku_slides_core::WidgetFile::parse("x").unwrap(), String::new())].into())
            .version();
    let err = deck_tokens(&bundle, &r).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::BadTheme);
}
