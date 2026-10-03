use speculoos::prelude::*;

use super::*;

/// Le français est prêt, l'allemand à télécharger, le reste inconnu.
fn engine(tag: &str) -> Availability {
    match language(tag).as_str() {
        "fr" => Availability::Ready,
        "de" => Availability::Missing,
        _ => Availability::Unsupported,
    }
}

#[test]
fn resolve_auto_takes_interface_language() {
    assert_that!(resolve(None, "fr", Some("de-DE"), engine)).is_equal_to(Some(("fr".to_owned(), Availability::Ready)));
}

#[test]
fn resolve_auto_unknown_interface_language_falls_back_to_system() {
    assert_that!(resolve(None, "th", Some("de-DE"), engine))
        .is_equal_to(Some(("de-DE".to_owned(), Availability::Missing)));
}

#[test]
fn resolve_chosen_language_ignores_interface() {
    assert_that!(resolve(Some("de"), "fr", None, engine)).is_equal_to(Some(("de".to_owned(), Availability::Missing)));
}

#[test]
fn resolve_chosen_unknown_language_returns_none() {
    assert_that!(resolve(Some("xx"), "fr", Some("fr-FR"), engine)).is_none();
}

#[test]
fn language_region_tag_returns_language_code() {
    assert_that!(language("fr_FR")).is_equal_to("fr".to_owned());
}
