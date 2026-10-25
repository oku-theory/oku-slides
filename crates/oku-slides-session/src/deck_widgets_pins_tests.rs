//! Épingles incohérentes avec leur manifeste, et assemblage raté du registre des packs.

use std::sync::Arc;

use speculoos::prelude::*;

use super::deck_widgets_tests::{
    QUOTE, add_quote, assert_registry_agrees, data_dir, kind_of, pin_pack, session_in, user_pack,
};
use crate::SessionErrorKind;
use crate::test_support::{namespace_of, ns};

#[test]
fn resolve_widgets_pin_under_another_namespace_than_its_manifest_is_disabled_alone() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Pack", vec![pin_pack(&quote)], None).unwrap();
    let mut pins = s.deck().widgets.clone();
    // La même version, aussi épinglée sous un autre espace de noms : même dossier `/packs/<hex>/`.
    pins.insert(ns("autre"), quote.clone());
    let (widgets, notice) = s.resolve_widgets(&pins, &s.doc.sources.packs).unwrap();
    assert_that!(widgets.disabled.keys().map(oku_slides_core::Namespace::as_str).collect::<Vec<_>>())
        .is_equal_to(vec!["autre"]);
    assert_that!(widgets.registry.get(&kind_of(&quote, "quote"))).is_some();
    assert_that!(widgets.registry.get("autre/quote")).is_none();
    let expected = s.packs_disabled_notice(["Citation"].into_iter());
    assert_that!(notice.unwrap_or_default().as_str()).contains(expected.as_str());
}

#[test]
fn resolve_widgets_pin_whose_requires_differ_from_its_manifest_is_disabled() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Pack", vec![pin_pack(&quote)], None).unwrap();
    let mut pins = s.deck().widgets.clone();
    let forged = oku_slides_core::PackPin { requires: vec![ns("essentiel")], ..quote.clone() };
    pins.insert(namespace_of(quote.id), forged);
    let (widgets, _) = s.resolve_widgets(&pins, &s.doc.sources.packs).unwrap();
    assert_that!(widgets.disabled.contains_key(&namespace_of(quote.id))).is_true();
    assert_that!(widgets.registry.get(&kind_of(&quote, "quote"))).is_none();
}

#[test]
fn apply_with_a_registry_assembly_failure_keeps_the_installed_widgets_and_disables_no_pack() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let first = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Pack", vec![pin_pack(&first)], None).unwrap();
    let (registry, pins) = (s.registry.clone(), s.deck().widgets.clone());
    let second = user_pack(&s, "Autre", &[("quote", QUOTE)]);
    s.failpoints.registry = true;
    let err = s.apply("Pack", vec![pin_pack(&second)], None).unwrap_err();
    s.failpoints.registry = false;
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Render);
    assert_that!(Arc::ptr_eq(&s.registry, &registry)).is_true();
    assert_that!(s.deck().widgets.clone()).is_equal_to(pins);
    assert_that!(s.widgets_disabled.is_empty()).is_true();
    assert_registry_agrees(&s);
    // Rien n'est retenu contre les packs : l'assemblage suivant les prend tous.
    s.apply("Pack", vec![pin_pack(&second), add_quote(&second)], None).unwrap();
    assert_that!(s.registry.get(&kind_of(&first, "quote"))).is_some();
    assert_that!(s.registry.get(&kind_of(&second, "quote"))).is_some();
}
