//! t-694 : A fait passer un élément d'un slot à l'autre pendant que B le met
//! en gras ; après fusion, le gras de B était perdu (retrait puis insertion
//! d'une copie). Les éléments d'une slide sont désormais les nœuds d'un
//! `LoroTree` : le déplacement est un `mov`, l'édition concurrente reste.

use oku_slides_core::{Deck, DeckReplica, Element, Frame, Op, ParamValue, apply_all};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &common::registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

/// L'élément `b-points` de `s-constat`, et son slot.
fn points(deck: &Deck) -> Vec<(String, Element)> {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    slide.walk().filter(|l| l.element.id == "b-points").map(|l| (l.slot.to_owned(), l.element.clone())).collect()
}

/// Le premier run du `body` de `e` est-il gras ?
fn first_run_bold(e: &Element) -> bool {
    match e.params.get("body") {
        Some(ParamValue::Rich(body)) => body.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold),
        _ => false,
    }
}

#[test]
fn move_across_slots_keeps_the_concurrent_bold() {
    let deck = roadmap();
    let registry = common::shared();
    let snap = DeckReplica::new(&deck, registry.clone())
        .unwrap_or_else(|e| panic!("replica : {e}"))
        .snapshot()
        .unwrap_or_else(|e| panic!("snapshot : {e}"));
    let open = || DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone()).unwrap_or_else(|e| panic!("{e}")).0;
    let (mut a, mut b) = (open(), open());
    let frame = Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 };

    // A : `b-points` quitte `blocks` pour `elements`, cadré.
    let original = deck.slide("s-constat").and_then(|s| s.locate("b-points")).map(|l| l.element.clone());
    let Some(original) = original else { panic!("b-points absent") };
    assert_that!(first_run_bold(&original)).is_false();
    let remove = op(json!({ "op": "remove_element", "slide": "s-constat", "element": "b-points" }));
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "elements" },
        "element": Element { frame: Some(frame), ..original.clone() } }));
    a.apply(&with_ops(&deck, vec![remove, add]), "A", None).unwrap_or_else(|e| panic!("apply : {e}"));

    // B : met en gras le premier run du `body` de `b-points`, visé par id.
    let Some(ParamValue::Rich(mut body)) = original.params.get("body").cloned() else { panic!("body absent") };
    body[0].runs[0].bold = true;
    let bold = op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "b-points" },
        "param": "body", "value": body }));
    let bold = with_ops(&deck, vec![bold]);
    assert_that!(points(&bold).iter().all(|(_, e)| first_run_bold(e))).is_true();
    b.apply(&bold, "B", None).unwrap_or_else(|e| panic!("apply : {e}"));

    a.merge(&b).unwrap_or_else(|e| panic!("merge : {e}"));
    b.merge(&a).unwrap_or_else(|e| panic!("merge : {e}"));
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&merged);
    let found = points(&merged);
    assert_that!(found.len()).is_equal_to(1);
    assert_that!(found[0].0.as_str()).is_equal_to("elements");
    assert_that!(found[0].1.frame).is_equal_to(Some(frame));
    assert_that!(first_run_bold(&found[0].1)).is_true();
}
