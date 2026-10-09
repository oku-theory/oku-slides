//! t-694 : annuler un retrait face à un déplacement concurrent garde des ids uniques.
//! A retire un élément (ou un conteneur et ses enfants) puis annule pendant
//! que B le déplace vers un autre slot. Après fusion, les deux pairs lisent le
//! même deck, sans `Issue`, et chaque id n'y figure qu'une fois, à toutes les
//! profondeurs ; une réécriture du deck relu puis un échange ne changent rien.

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

/// A applique `removed` et l'annule, B applique `moved` ; ils échangent. Puis
/// chacun réécrit le deck relu et ils échangent encore : les deux convergent.
/// Le deck commun après la première fusion.
fn undo_vs_move(deck: &Deck, removed: &Deck, moved: &Deck) -> Deck {
    let (mut a, mut b) = peers(deck, &common::shared());
    apply(&mut a, removed, "Retirer");
    assert_that!(a.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
    apply(&mut b, moved, "Déplacer");
    exchange(&mut a, &mut b);
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&merged);
    apply(&mut a, &merged, "A");
    assert_that!(a.deck().0).is_equal_to(&merged);
    apply(&mut b, &merged, "B");
    exchange(&mut a, &mut b);
    assert_that!(a.deck().0).is_equal_to(&merged);
    assert_that!(b.deck().0).is_equal_to(&merged);
    merged
}

/// Les éléments de `s-constat`, à toutes les profondeurs : (id, slot).
fn elements_of(deck: &Deck) -> Vec<(String, Element)> {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    slide.walk().map(|l| (l.element.id.clone(), l.element.clone())).collect()
}

fn ids_unique(deck: &Deck) -> bool {
    let mut ids: Vec<String> = elements_of(deck).into_iter().map(|(id, _)| id).collect();
    let n = ids.len();
    ids.sort();
    ids.dedup();
    ids.len() == n
}

fn find(deck: &Deck, id: &str) -> Option<Element> {
    elements_of(deck).into_iter().find(|(i, _)| i == id).map(|(_, e)| e)
}

fn bolded(e: &Element) -> Element {
    let Some(ParamValue::Rich(mut body)) = e.params.get("body").cloned() else { panic!("body absent") };
    body[0].runs[0].bold = true;
    let mut e = e.clone();
    e.params.set("body", ParamValue::Rich(body));
    e
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

fn framed(e: &Element) -> Element {
    Element { frame: Some(Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 }), ..e.clone() }
}

#[test]
fn undo_remove_while_peer_moves_and_bolds_element_across_slots_keeps_one_unique_copy() {
    let deck = roadmap();
    let points = find(&deck, "b-points").unwrap_or_else(|| panic!("b-points absent"));
    let removed = with_ops(&deck, vec![remove("b-points")]);
    let moved = with_ops(&deck, vec![remove("b-points"), add(None, "elements", &framed(&bolded(&points)))]);
    let merged = undo_vs_move(&deck, &removed, &moved);
    assert_that!(ids_unique(&merged)).is_true();
    // Limite acceptée (t-747, voir `issue_0652`) : Loro ne ressuscite jamais un
    // nœud supprimé, donc le `mov` de B perd face au retrait de A et son gras
    // aussi. Annuler recrée une copie de la version de A : un seul `b-points`.
    let kept = ["b-points", "b-points-2"].iter().filter(|id| find(&merged, id).is_some()).count();
    assert_that!(kept).is_equal_to(1);
}

#[test]
fn undo_remove_while_peer_moves_owner_with_children_keeps_ids_unique_at_every_depth() {
    let text = |id: &str| -> Element {
        serde_json::from_value(json!({ "id": id, "kind": "text", "params": { "body": [{ "runs": [{ "t": id }] }] } }))
            .unwrap_or_else(|e| panic!("texte : {e}"))
    };
    let pile = Element::new("pile", "stack");
    let deck = with_ops(
        &roadmap(),
        vec![
            add(None, "blocks", &pile),
            add(Some("pile"), "children", &text("k1")),
            add(Some("pile"), "children", &text("k2")),
        ],
    );
    let removed = with_ops(&deck, vec![remove("pile")]);
    let k1 = find(&deck, "k1").unwrap_or_else(|| panic!("k1 absent"));
    let moved = with_ops(
        &deck,
        vec![
            remove("pile"),
            add(None, "elements", &framed(&pile)),
            add(Some("pile"), "children", &bolded(&k1)),
            add(Some("pile"), "children", &text("k2")),
        ],
    );
    let merged = undo_vs_move(&deck, &removed, &moved);
    assert_that!(ids_unique(&merged)).is_true();
    let ids: Vec<String> = elements_of(&merged).into_iter().map(|(id, _)| id).collect();
    assert_that!(ids.iter().any(|id| id.starts_with("pile"))).is_true();
    assert_that!(ids.iter().filter(|id| id.starts_with('k')).count()).is_greater_than_or_equal_to(2);
}
