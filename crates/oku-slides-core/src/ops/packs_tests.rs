use crate::kinds::essentiel::BLANK;
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::kinds;
use crate::test_registry::registry;
use crate::{Deck, Element, Namespace, Op, OpErrorKind, PackKind, PackPin, PackVersion, ParamValue, Slide, apply_all};

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
    Op::AddPack { kind: PackKind::Widgets, namespace: ns(n), pin: p }
}

/// Le retrait du pack de widgets `n`.
fn remove(n: Namespace) -> Op {
    Op::RemovePack { kind: PackKind::Widgets, namespace: n }
}

/// `base` parmi les widgets, `essentiel` (qui a besoin de `base`) parmi les layouts.
fn deck_with_essentiel() -> Deck {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    deck.layouts.insert(kinds::essentiel::namespace(), pin(&["base"]));
    deck
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
    let kind = refusal(&mut deck, vec![remove(ns("acme"))]);
    assert_that!(kind).is_equal_to(OpErrorKind::UnknownNamespace);
}

#[test]
fn remove_pack_required_by_another_is_refused() {
    let mut deck = Deck::empty();
    apply_all(&mut deck, vec![add("deck", pin(&[])), add("acme", pin(&["deck"]))], &registry()).unwrap();
    let kind = refusal(&mut deck, vec![remove(ns("deck"))]);
    assert_that!(kind).is_equal_to(OpErrorKind::PackRequired);
}

#[test]
fn remove_pack_used_by_an_element_is_refused() {
    let mut deck = Deck::empty();
    deck.widgets.insert(ns("acme"), pin(&[]));
    deck.slides.push(slide_with(BLANK, vec![Element::new("e1", "acme/card")]));
    let kind = refusal(&mut deck, vec![remove(ns("acme"))]);
    assert_that!(kind).is_equal_to(OpErrorKind::PackInUse);
}

#[test]
fn remove_pack_unused_unpins_it() {
    let mut deck = Deck::empty();
    deck.widgets.insert(ns("acme"), pin(&[]));
    apply_all(&mut deck, vec![remove(ns("acme"))], &registry()).unwrap();
    assert_that!(deck.widgets.is_empty()).is_true();
}

#[test]
fn remove_pack_base_is_refused_even_unused() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let kind = refusal(&mut deck, vec![remove(kinds::base::namespace())]);
    assert_that!(kind).is_equal_to(OpErrorKind::BaseRequired);
}

#[test]
fn remove_pack_base_message_is_localized_in_french() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let err = apply_all(&mut deck, vec![remove(kinds::base::namespace())], &registry()).unwrap_err();
    let fr = err.op_error().localize(&Localizer::new(Some("fr")));
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

#[test]
fn add_pack_layouts_pins_into_deck_layouts() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace(), pin: pin(&["base"]) };
    apply_all(&mut deck, vec![op], &registry()).unwrap();
    assert_that!((deck.layouts.contains_key(&kinds::essentiel::namespace()), deck.widgets.len()))
        .is_equal_to((true, 1));
}

#[test]
fn add_pack_layouts_requiring_a_missing_widget_pack_is_refused() {
    let mut deck = Deck::empty();
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace(), pin: pin(&["base"]) };
    assert_that!(refusal(&mut deck, vec![op])).is_equal_to(OpErrorKind::PackRequirementMissing);
}

#[test]
fn add_pack_layouts_requirement_met_only_by_a_layout_pack_is_refused() {
    let mut deck = Deck::empty();
    deck.layouts.insert(ns("grid"), pin(&[]));
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: ns("acme"), pin: pin(&["grid"]) };
    assert_that!(refusal(&mut deck, vec![op])).is_equal_to(OpErrorKind::PackRequirementMissing);
}

#[test]
fn add_pack_layouts_requiring_itself_is_refused() {
    // Seul un pack de widgets peut se nommer dans ses besoins : un layout n'est pas un widget.
    let mut deck = Deck::empty();
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: ns("acme"), pin: pin(&["acme"]) };
    assert_that!(refusal(&mut deck, vec![op])).is_equal_to(OpErrorKind::PackRequirementMissing);
}

#[rstest]
#[case(PackKind::Widgets, PackKind::Layouts)]
#[case(PackKind::Layouts, PackKind::Widgets)]
fn add_pack_namespace_pinned_under_the_other_kind_is_refused(#[case] first: PackKind, #[case] second: PackKind) {
    let mut deck = Deck::empty();
    apply_all(&mut deck, vec![Op::AddPack { kind: first, namespace: ns("acme"), pin: pin(&[]) }], &registry()).unwrap();
    let op = Op::AddPack { kind: second, namespace: ns("acme"), pin: pin(&[]) };
    assert_that!(refusal(&mut deck, vec![op])).is_equal_to(OpErrorKind::PackKindConflict);
}

#[test]
fn remove_pack_essentiel_used_by_a_slide_is_refused_in_use() {
    let mut deck = deck_with_essentiel();
    deck.slides.push(Slide::new("s", kinds::essentiel::CONTENT));
    let op = Op::RemovePack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace() };
    assert_that!(refusal(&mut deck, vec![op])).is_equal_to(OpErrorKind::PackInUse);
}

#[test]
fn remove_pack_essentiel_unused_is_removed() {
    let mut deck = deck_with_essentiel();
    let op = Op::RemovePack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace() };
    apply_all(&mut deck, vec![op], &registry()).unwrap();
    assert_that!(deck.layouts.is_empty()).is_true();
}

#[test]
fn remove_pack_widgets_required_by_a_layout_pack_is_refused() {
    let mut deck = Deck::empty();
    deck.widgets.insert(ns("acme"), pin(&[]));
    deck.layouts.insert(ns("grid"), pin(&["acme"]));
    assert_that!(refusal(&mut deck, vec![remove(ns("acme"))])).is_equal_to(OpErrorKind::PackRequired);
}

#[test]
fn remove_pack_under_the_wrong_kind_is_unknown_namespace() {
    let mut deck = deck_with_essentiel();
    let op = Op::RemovePack { kind: PackKind::Widgets, namespace: kinds::essentiel::namespace() };
    assert_that!(refusal(&mut deck, vec![op])).is_equal_to(OpErrorKind::UnknownNamespace);
}

#[test]
fn pack_kind_conflict_message_is_localized_in_french() {
    let mut deck = deck_with_essentiel();
    let op = Op::AddPack { kind: PackKind::Widgets, namespace: kinds::essentiel::namespace(), pin: pin(&[]) };
    let err = apply_all(&mut deck, vec![op], &registry()).unwrap_err();
    let fr = err.op_error().localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(err.op_error().localize(oku_slides_i18n::english()));
}

/// `base`, `acme` utilisé par un élément, `dep` requis par `acme`, `solo` inutilisé.
fn deck_for_refusals() -> Deck {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    deck.widgets.insert(ns("dep"), pin(&[]));
    deck.widgets.insert(ns("acme"), pin(&["dep"]));
    deck.widgets.insert(ns("solo"), pin(&[]));
    deck.slides.push(slide_with(BLANK, vec![Element::new("e1", "acme/card")]));
    deck
}

#[rstest]
#[case::base("base", OpErrorKind::BaseRequired)]
#[case::in_use("acme", OpErrorKind::PackInUse)]
#[case::required("dep", OpErrorKind::PackRequired)]
#[case::unknown("ghost", OpErrorKind::UnknownNamespace)]
fn remove_pack_refusal_names_the_reason(#[case] pack: &str, #[case] expected: OpErrorKind) {
    let deck = deck_for_refusals();
    let err = remove_pack_refusal(&deck, PackKind::Widgets, &ns(pack)).expect("refus attendu");
    assert_that!(err.kind()).is_equal_to(expected);
}

#[test]
fn remove_pack_refusal_unused_pack_is_none() {
    let deck = deck_for_refusals();
    assert_that!(remove_pack_refusal(&deck, PackKind::Widgets, &ns("solo"))).is_none();
}
