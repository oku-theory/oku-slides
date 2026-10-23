use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::SessionErrorKind;
use crate::test_support::{library_with_child, library_with_themes, ns};

#[test]
fn theme_closure_of_a_child_pins_it_and_its_parent() {
    let (themes, _dir) = library_with_child("marque", &["papier"]);
    let (pins, files) = theme_closure(&themes, &ns("marque")).unwrap();
    assert_that!(pins.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["marque", "papier"]);
    assert_that!(files.len()).is_equal_to(2);
}

#[test]
fn theme_closure_with_an_unknown_parent_is_refused() {
    let (themes, _dir) = library_with_child("marque", &["absent"]);
    assert_that!(theme_closure(&themes, &ns("marque")).unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownTheme);
}

#[test]
fn theme_closure_of_an_unknown_theme_is_refused() {
    let (themes, _dir) = library_with_child("marque", &[]);
    assert_that!(theme_closure(&themes, &ns("absent")).unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownTheme);
}

#[test]
fn theme_closure_of_a_builtin_pins_it_alone_with_its_current_version() {
    let (themes, _dir) = library_with_child("marque", &[]);
    let (pins, files) = theme_closure(&themes, &ns("pro")).unwrap();
    let pro = themes.current_version(crate::PRO_THEME).unwrap();
    assert_that!(pins.get(&ns("pro")).map(|p| (p.id, p.version.clone())))
        .is_equal_to(Some((crate::PRO_THEME, pro.version)));
    assert_that!(files.len()).is_equal_to(1);
}

/// Une boucle de deux thèmes.
const TWO: [(&str, &[&str]); 2] = [("a", &["b"]), ("b", &["a"])];
/// Un thème qui s'étend lui-même.
const SELF_LOOP: [(&str, &[&str]); 1] = [("a", &["a"])];

#[rstest]
#[case::self_loop(&SELF_LOOP, "a → a")]
#[case::two(&TWO, "a → b → a")]
fn theme_closure_of_a_cycle_is_refused_with_its_chain(#[case] themes: &[(&str, &[&str])], #[case] chain: &str) {
    let (library, _dir) = library_with_themes(themes);
    let err = theme_closure(&library, &ns("a")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeCycle);
    assert_that!(err.to_string()).contains(chain);
}

#[test]
fn theme_closure_of_a_diamond_pins_each_theme_once() {
    let (library, _dir) = library_with_themes(&[("b", &["papier"]), ("c", &["papier"]), ("a", &["b", "c"])]);
    let (pins, files) = theme_closure(&library, &ns("a")).unwrap();
    assert_that!(pins.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["a", "b", "c", "papier"]);
    assert_that!(files.len()).is_equal_to(4);
}

#[rstest]
#[case::cycle(&TWO, "a", "session-theme-cycle")]
#[case::closure_extra(&[], "papier", "session-theme-closure")]
fn theme_chain_errors_are_localized(#[case] themes: &[(&str, &[&str])], #[case] theme: &str, #[case] key: &str) {
    use oku_slides_i18n::{Localize, Localizer};
    let (library, _dir) = library_with_themes(themes);
    let err = match theme_closure(&library, &ns(theme)) {
        Err(e) => e,
        // Une fermeture valide, rendue invalide par un épinglage de trop.
        Ok((mut pins, _)) => {
            pins.insert(ns("pro"), pins[&ns(theme)].clone());
            crate::error::theme_chain_error(oku_slides_core::check_closure(Some(&ns(theme)), &pins).unwrap_err())
        }
    };
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr).is_not_equal_to(&en);
    let shown = Localizer::new(Some("fr")).tr_with(key, &[("chain", "a → b → a".into()), ("namespace", "pro".into())]);
    assert_that!(fr).is_equal_to(shown);
}
