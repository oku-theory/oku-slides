//! t-747 : un retrait d'élément est une marque par pair (`@deleted/<pair>`),
//! jamais un `tree.delete`. Annuler son retrait ne défait pas celui d'un autre
//! pair, et le nœud rendu garde le déplacement et l'édition reçus entre-temps.

use std::sync::Arc;

use oku_slides_core::{Deck, DeckReplica, Element, Frame, Op, ParamValue, WidgetRegistry, apply_all};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &common::registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn peers(deck: &Deck, registry: &Arc<WidgetRegistry>) -> (DeckReplica, DeckReplica) {
    let r = DeckReplica::new(deck, registry.clone()).unwrap_or_else(|e| panic!("replica : {e}"));
    let snap = r.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
    let open = || DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone()).unwrap_or_else(|e| panic!("{e}")).0;
    (open(), open())
}

fn exchange(a: &mut DeckReplica, b: &mut DeckReplica) {
    a.merge(b).unwrap_or_else(|e| panic!("merge : {e}"));
    b.merge(a).unwrap_or_else(|e| panic!("merge : {e}"));
}

fn apply(r: &mut DeckReplica, deck: &Deck, label: &str) {
    r.apply(deck, label, None).unwrap_or_else(|e| panic!("apply : {e}"));
}

fn undo(r: &mut DeckReplica) -> bool {
    r.undo().unwrap_or_else(|e| panic!("undo : {e}"))
}

fn remove(id: &str) -> Op {
    op(json!({ "op": "remove_element", "slide": "s-constat", "element": id }))
}

fn add(owner: Option<&str>, param: &str, element: &Element) -> Op {
    let owner = match owner {
        Some(e) => json!({ "slide": "s-constat", "element": e }),
        None => json!({ "slide": "s-constat" }),
    };
    op(json!({ "op": "add_element", "slot": { "owner": owner, "param": param }, "element": element }))
}

/// Les éléments de `s-constat` d'id `id`, à toutes les profondeurs.
fn find(deck: &Deck, id: &str) -> Vec<Element> {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    slide.walk().filter(|l| l.element.id == id).map(|l| l.element.clone()).collect()
}

fn bolded(e: &Element) -> Element {
    let Some(ParamValue::Rich(mut body)) = e.params.get("body").cloned() else { panic!("body absent") };
    body[0].runs[0].bold = true;
    let mut e = e.clone();
    e.params.set("body", ParamValue::Rich(body));
    e
}

fn is_bold(e: &Element) -> bool {
    matches!(e.params.get("body"), Some(ParamValue::Rich(b)) if b.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold))
}

fn framed(e: &Element) -> Element {
    Element { frame: Some(Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 }), ..e.clone() }
}

/// Les ids du slot `elements` de `s-constat`.
fn free_ids(deck: &Deck) -> Vec<String> {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    slide.params.slot("elements").unwrap_or_default().iter().map(|e| e.id.clone()).collect()
}

/// Les deux pairs lisent le même deck, sans `Issue` et sans `b-points` ; le
/// réécrire n'écrit rien.
fn removed_on_both(a: &mut DeckReplica, b: &mut DeckReplica, case: &str) {
    let (merged, issues) = a.deck();
    assert_that!(issues).named(case).is_empty();
    assert_that!(find(&merged, "b-points")).named(case).is_empty();
    let (other, issues) = b.deck();
    assert_that!(issues).named(case).is_empty();
    assert_that!(other).named(case).is_equal_to(&merged);
    for r in [a, b] {
        let before = r.len_ops();
        let read = r.deck().0;
        apply(r, &read, "relire");
        assert_that!(r.len_ops()).named(case).is_equal_to(before);
    }
}

/// Parade de la sonde (`p1_concurrent_double_soft_delete_then_one_undo`) :
/// chaque pair pose sa propre marque, l'annulation de A ne retire que la sienne.
#[test]
fn concurrent_removals_then_one_undo_keep_the_element_removed() {
    let deck = roadmap();
    let removed = with_ops(&deck, vec![remove("b-points")]);

    let (mut a, mut b) = peers(&deck, &common::shared());
    apply(&mut a, &removed, "Retirer");
    apply(&mut b, &removed, "Retirer");
    exchange(&mut a, &mut b);
    assert_that!(undo(&mut a)).is_true();
    exchange(&mut a, &mut b);
    removed_on_both(&mut a, &mut b, "retraits croisés, puis A annule");

    let (mut a, mut b) = peers(&deck, &common::shared());
    apply(&mut a, &removed, "Retirer");
    assert_that!(undo(&mut a)).is_true();
    apply(&mut b, &removed, "Retirer");
    exchange(&mut a, &mut b);
    removed_on_both(&mut a, &mut b, "A retire et annule, B retire");

    // A reçoit le retrait de B avant de retirer : son retrait n'a plus rien à
    // marquer, donc rien à annuler, et le retrait de B reste.
    let (mut a, mut b) = peers(&deck, &common::shared());
    apply(&mut b, &removed, "Retirer");
    exchange(&mut a, &mut b);
    apply(&mut a, &removed, "Retirer");
    exchange(&mut a, &mut b);
    assert_that!(undo(&mut a)).is_false();
    exchange(&mut a, &mut b);
    removed_on_both(&mut a, &mut b, "B retire, puis A retire et annule");
}

/// Sonde, `p1_undo_soft_delete_after_receiving_move_does_not_move_back` :
/// annuler ne retire que la marque, l'élément reste où B l'a mis.
#[test]
fn undo_remove_after_receiving_the_peer_move_keeps_the_move() {
    let deck = roadmap();
    let points = find(&deck, "b-points").pop().unwrap_or_else(|| panic!("b-points absent"));
    let (mut a, mut b) = peers(&deck, &common::shared());
    apply(&mut a, &with_ops(&deck, vec![remove("b-points")]), "Retirer");
    let moved = with_ops(&deck, vec![remove("b-points"), add(None, "elements", &framed(&bolded(&points)))]);
    apply(&mut b, &moved, "Déplacer");
    a.merge(&b).unwrap_or_else(|e| panic!("merge : {e}"));
    assert_that!(undo(&mut a)).is_true();
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        let (read, issues) = r.deck();
        assert_that!(issues).is_empty();
        let found = find(&read, "b-points");
        assert_that!(found.len()).is_equal_to(1);
        assert_that!(free_ids(&read)).contains("b-points".to_owned());
        assert_that!(is_bold(&found[0])).is_true();
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
}
