use std::collections::BTreeSet;

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
            assert!(FluentResource::try_new((*source).to_owned()).is_ok(), "{locale} : ressource invalide");
        }
    }
}

#[test]
fn every_locale_has_exactly_the_reference_keys() {
    let reference = keys(REFERENCE);
    assert!(reference.len() > 50);
    for (locale, _) in LOCALES {
        assert_eq!(keys(locale), reference, "{locale} : clés différentes de {REFERENCE}");
    }
}

#[test]
fn new_region_tag_selects_its_language() {
    assert_eq!(Localizer::new(Some("fr-CA")).locale(), "fr");
    assert_eq!(Localizer::new(Some("en_GB")).locale(), "en");
}

#[test]
fn tr_with_args_formats_without_isolation_marks() {
    let fr = Localizer::new(Some("fr"));
    assert_eq!(fr.tr_with("cli-deck-valid", &[("slides", 8.into())]), "deck valide : 8 slides");
    assert_eq!(english().tr_with("session-read", &[("path", "/a/b".into())]), "Cannot read /a/b");
}

#[test]
fn tr_unknown_id_returns_the_id() {
    assert_eq!(Localizer::new(Some("fr")).tr("does-not-exist"), "does-not-exist");
}
