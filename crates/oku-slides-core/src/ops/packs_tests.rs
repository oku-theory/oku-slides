use oku_slides_i18n::Localize;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::kinds;
use crate::test_registry::registry;
use crate::{Deck, Element, Namespace, Op, OpErrorKind, PackPin, PackVersion, ParamValue, Slide, apply_all};

fn ns(s: &str) -> Namespace {
    Namespace::parse(s).unwrap()
}

fn pin(requires: &[&str]) -> PackPin {
    PackPin {
        id: Uuid::new_v4(),
        version: PackVersion::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        requires: requires.iter().map(|r| ns(r)).collect(),
    }
}

fn add(n: &str, p: PackPin) -> Op {
    Op::AddPack { namespace: ns(n), pin: p }
}

/// La nature de l'erreur d'une transaction refusée.
fn refusal(deck: &mut Deck, ops: Vec<Op>) -> OpErrorKind {
    apply_all(deck, ops, &registry()).unwrap_err().op_error().kind()
}

/// Une slide `s1` de widget `kind` dont le slot `elements` porte `elements`.
fn slide_with(kind: &str, elements: Vec<Element>) -> Slide {
    let mut slide = Slide::new("s1", kind);
    slide.params.set("elements", ParamValue::Slot(elements));
    slide
}

#[test]
fn add_pack_new_namespace_pins_it() {
    let mut deck = Deck::empty();
    apply_all(&mut deck, vec![add("acme", pin(&[]))], &registry()).unwrap();
    assert_that!(deck.widgets.contains_key(&ns("acme"))).is_true();
}

#[test]
fn add_pack_same_namespace_replaces_the_pin() {
    let mut deck = Deck::empty();
    let (a, b) = (pin(&[]), pin(&[]));
    apply_all(&mut deck, vec![add("acme", a), add("acme", b.clone())], &registry()).unwrap();
    assert_that!(deck.widgets.get(&ns("acme"))).is_equal_to(Some(&b));
}

#[test]
fn add_pack_missing_requirement_is_refused() {
    let mut deck = Deck::empty();
    let kind = refusal(&mut deck, vec![add("acme", pin(&["deck"]))]);
    assert_that!(kind).is_equal_to(OpErrorKind::PackRequirementMissing);
}

#[test]
fn add_pack_requirement_added_first_in_same_transaction_is_accepted() {
    let mut deck = Deck::empty();
    let ops = vec![add("deck", pin(&[])), add("acme", pin(&["deck"]))];
    assert_that!(apply_all(&mut deck, ops, &registry())).is_ok();
}

#[test]
fn remove_pack_unknown_namespace_is_refused() {
    let mut deck = Deck::empty();
    let kind = refusal(&mut deck, vec![Op::RemovePack { namespace: ns("acme") }]);
    assert_that!(kind).is_equal_to(OpErrorKind::UnknownNamespace);
}

#[test]
fn remove_pack_required_by_another_is_refused() {
    let mut deck = Deck::empty();
    apply_all(&mut deck, vec![add("deck", pin(&[])), add("acme", pin(&["deck"]))], &registry()).unwrap();
    let kind = refusal(&mut deck, vec![Op::RemovePack { namespace: ns("deck") }]);
    assert_that!(kind).is_equal_to(OpErrorKind::PackRequired);
}

#[test]
fn remove_pack_used_by_an_element_is_refused() {
    let mut deck = Deck::empty();
    deck.widgets.insert(ns("acme"), pin(&[]));
    deck.slides.push(slide_with("blank", vec![Element::new("e1", "acme/card")]));
    let kind = refusal(&mut deck, vec![Op::RemovePack { namespace: ns("acme") }]);
    assert_that!(kind).is_equal_to(OpErrorKind::PackInUse);
}

#[test]
fn remove_pack_unused_unpins_it() {
    let mut deck = Deck::empty();
    deck.widgets.insert(ns("acme"), pin(&[]));
    apply_all(&mut deck, vec![Op::RemovePack { namespace: ns("acme") }], &registry()).unwrap();
    assert_that!(deck.widgets.is_empty()).is_true();
}

#[test]
fn remove_pack_base_is_refused_even_unused() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let kind = refusal(&mut deck, vec![Op::RemovePack { namespace: kinds::base::namespace() }]);
    assert_that!(kind).is_equal_to(OpErrorKind::BaseRequired);
}

#[test]
fn remove_pack_base_message_is_localized_in_french() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let err =
        apply_all(&mut deck, vec![Op::RemovePack { namespace: kinds::base::namespace() }], &registry()).unwrap_err();
    let fr = err.op_error().localize(&oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(err.op_error().localize(oku_slides_i18n::english()));
}

#[test]
fn pack_users_lists_slides_and_nested_elements() {
    let mut deck = Deck::empty();
    let mut card = Element::new("e1", "acme/card");
    card.params.set("body", ParamValue::Slot(vec![Element::new("e3", "acme/badge")]));
    deck.slides.push(slide_with("acme/cover", vec![card, Element::new("e2", "base/text")]));
    let users = pack_users(&deck, "acme");
    assert_that!(users).is_equal_to(vec!["s1".to_owned(), "e1".to_owned(), "e3".to_owned()]);
}
