//! t-776 : B crée une slide, A y déplace un élément existant
//! (`move_element`), B annule sa création. Loro inversait la création par
//! `tree.delete` : l'élément de A partait avec la slide, et le ⌘Z de A ne le
//! rendait pas. La création naît marquée : annuler remet la marque, la slide et
//! l'élément sont cachés, et le ⌘Z de A ou le rétablir de B les rend.

use std::sync::Arc;

use loro::{Container, LoroDoc, ValueOrContainer};
use oku_slides_core::{Deck, DeckReplica, Op, WidgetRegistry, apply_all, apply_all_moving};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
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

fn undo(r: &mut DeckReplica) {
    assert_that!(r.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
}

/// Les slides de `r` qui portent l'élément `id`.
fn where_is(r: &DeckReplica, id: &str) -> Vec<String> {
    r.deck().0.slides.iter().filter(|s| s.locate(id).is_some()).map(|s| s.id.clone()).collect()
}

fn has_slide(r: &DeckReplica, id: &str) -> bool {
    r.deck().0.slide(id).is_some()
}

/// Le nombre de racines de l'arbre du deck de `r`, lu brut, qui portent une marque de suppression.
///
/// Défaire la création remet la marque sur le nœud et en retire le contenu (`id` compris) :
/// la survie du nœud se lit à sa marque, non à son `id`.
fn marked_roots(r: &DeckReplica) -> usize {
    let doc = LoroDoc::new();
    doc.import(&r.snapshot().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("arbre du deck attendu")
    };
    tree.roots()
        .into_iter()
        .filter(|n| tree.get_meta(*n).is_ok_and(|m| m.keys().any(|k| k.starts_with("@deleted/"))))
        .count()
}

/// B ajoute `s-neuve` ; A y déplace `b-points` (de `s-constat`) ; échanges.
fn scenario() -> (DeckReplica, DeckReplica) {
    let registry = common::shared();
    let (mut a, mut b) = peers(&roadmap(), &registry);
    let mut added = b.deck().0;
    let slide = json!({ "id": "s-neuve", "kind": "essentiel/content", "params": { "title": "Neuve" } });
    apply_all(&mut added, vec![op(json!({ "op": "add_slide", "slide": slide }))], &registry)
        .unwrap_or_else(|e| panic!("{e:?}"));
    b.apply(&added, "Ajouter une slide", None).unwrap_or_else(|e| panic!("{e}"));
    exchange(&mut a, &mut b);
    let mut moved = a.deck().0;
    let mv = op(json!({ "op": "move_element", "slide": "s-constat", "element": "b-points",
        "to": { "owner": { "slide": "s-neuve" }, "param": "blocks" }, "index": 0 }));
    let moves = apply_all_moving(&mut moved, vec![mv], &registry).unwrap_or_else(|e| panic!("{e:?}"));
    a.apply_moving(&moved, "Déplacer", None, &moves).unwrap_or_else(|e| panic!("{e}"));
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        assert_that!(where_is(r, "b-points")).named("précondition").is_equal_to(vec!["s-neuve".to_owned()]);
    }
    (a, b)
}

#[test]
fn undo_of_a_slide_creation_hides_the_slide_without_deleting_its_node() {
    let (mut a, mut b) = scenario();
    undo(&mut b);
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        assert_that!(has_slide(r, "s-neuve")).is_false();
        assert_that!(where_is(r, "b-points")).is_empty();
        assert_that!(marked_roots(r)).named("nœud de s-neuve marqué, non supprimé").is_equal_to(1);
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
}

#[test]
fn undo_by_the_peer_after_the_creation_undo_brings_the_element_back() {
    let (mut a, mut b) = scenario();
    undo(&mut b);
    exchange(&mut a, &mut b);
    undo(&mut a);
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        assert_that!(where_is(r, "b-points")).is_equal_to(vec!["s-constat".to_owned()]);
    }
}

#[test]
fn redo_of_the_creation_brings_the_slide_back_with_the_element() {
    let (mut a, mut b) = scenario();
    undo(&mut b);
    exchange(&mut a, &mut b);
    assert_that!(b.redo().unwrap_or_else(|e| panic!("redo : {e}"))).is_true();
    exchange(&mut a, &mut b);
    assert_that!(where_is(&a, "b-points")).is_equal_to(vec!["s-neuve".to_owned()]);
    assert_that!(b.deck().0).is_equal_to(a.deck().0);
}
