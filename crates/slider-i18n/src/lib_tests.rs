use std::collections::BTreeSet;

use speculoos::prelude::*;

use super::*;

fn keys(locale: &str) -> BTreeSet<String> {
    let (_, sources) = LOCALES.iter().find(|(l, _)| *l == locale).unwrap();
    sources
        .iter()
        .flat_map(|s| s.lines())
        .filter_map(|line| line.split_once(" = ").map(|(k, _)| k.trim().to_owned()))
        .filter(|k| !k.starts_with('#') && !k.starts_with(' ') && !k.is_empty())
        .collect()
}

#[test]
fn every_locale_parses_without_error() {
    for (locale, sources) in LOCALES {
        for source in *sources {
            asserting(&format!("{locale} : ressource invalide"))
                .that(&FluentResource::try_new((*source).to_owned()))
                .is_ok();
        }
    }
}

#[test]
fn every_locale_has_exactly_the_reference_keys() {
    let reference = keys(REFERENCE);
    assert_that!(reference.len()).is_greater_than(50);
    for (locale, _) in LOCALES {
        asserting(&format!("{locale} : clés différentes de {REFERENCE}")).that(&keys(locale)).is_equal_to(&reference);
    }
}

#[test]
fn new_region_tag_selects_its_language() {
    assert_that!(Localizer::new(Some("fr-CA")).locale()).is_equal_to("fr");
    assert_that!(Localizer::new(Some("en_GB")).locale()).is_equal_to("en");
}

#[test]
fn tr_with_args_formats_without_isolation_marks() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(fr.tr_with("cli-deck-valid", &[("slides", 8.into())]))
        .is_equal_to("deck valide : 8 slides".to_owned());
    assert_that!(english().tr_with("session-read", &[("path", "/a/b".into())]))
        .is_equal_to("Cannot read /a/b".to_owned());
}

#[test]
fn tr_unknown_id_returns_the_id() {
    assert_that!(Localizer::new(Some("fr")).tr("does-not-exist")).is_equal_to("does-not-exist".to_owned());
}
