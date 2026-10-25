//! Convergence des déplacements dans l'arbre du deck : dans une slide (t-694),
//! d'une slide à l'autre et face aux retraits annulés (t-746, t-747).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use oku_slides_core::kinds::base;
use oku_slides_core::kinds::essentiel::CONTENT;
use std::sync::Arc;

use oku_slides_core::{
    Deck, DeckReplica, Element, Frame, Located, Moved, Op, ParamValue, Params, Slide, apply_all, apply_all_moving,
    validate,
};
use serde_json::{Value, json};

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

/// La valeur d'un résultat attendu réussi.
fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &common::registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

fn roadmap() -> Deck {
    ok(Deck::from_json(include_str!("../../../fixtures/roadmap.json")))
}

/// Deux pairs partis du même replica de `deck`.
fn peers(deck: &Deck) -> (DeckReplica, DeckReplica) {
    let registry = Arc::new(common::registry());
    let snap = ok(ok(DeckReplica::new(deck, registry.clone())).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone())).0;
    (open(), open())
}

/// Ajoute `element` au slot `param` de `owner` (`None` : la slide `s-constat`).
fn add(owner: Option<&str>, param: &str, element: &Element) -> Op {
    let owner = match owner {
        Some(e) => json!({ "slide": "s-constat", "element": e }),
        None => json!({ "slide": "s-constat" }),
    };
    op(json!({ "op": "add_element", "slot": { "owner": owner, "param": param }, "element": element }))
}

fn remove(id: &str) -> Op {
    op(json!({ "op": "remove_element", "slide": "s-constat", "element": id }))
}

/// `deck` et un `stack` d'id `id` à la fin de `blocks` de `s-constat`.
fn with_pile(deck: &Deck, id: &str) -> Deck {
    with_ops(deck, vec![add(None, "blocks", &Element::new(id, base::STACK))])
}

/// Les occurrences de `id` dans `s-constat`, à toutes les profondeurs.
fn located<'a>(deck: &'a Deck, id: &str) -> Vec<Located<'a>> {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    slide.walk().filter(|l| l.element.id == id).collect()
}

/// L'élément `id` de `s-constat`, sans cadre ni cellule.
fn bare(deck: &Deck, id: &str) -> Element {
    let Some(l) = located(deck, id).pop() else { panic!("{id} absent") };
    Element { frame: None, cell: None, ..l.element.clone() }
}

/// Retire `id` et ajoute sa copie dans le slot `children` de `owner`.
fn nest(deck: &Deck, id: &str, owner: &str) -> Vec<Op> {
    vec![remove(id), add(Some(owner), "children", &bare(deck, id))]
}

/// Le premier run du `body` de `e` est-il gras ?
fn first_run_bold(e: &Element) -> bool {
    match e.params.get("body") {
        Some(ParamValue::Rich(body)) => body.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold),
        _ => false,
    }
}

/// Bascule le gras du premier run de l'élément `id` de `params`, à toutes les
/// profondeurs ; `true` si l'élément a été trouvé avec un run.
fn toggle_bold_of(params: &mut Params, id: &str) -> bool {
    for (_, list) in params.slots_mut() {
        for el in list.iter_mut() {
            if el.id == id {
                let Some(ParamValue::Rich(body)) = el.params.0.get_mut("body") else { return false };
                let Some(run) = body.first_mut().and_then(|p| p.runs.first_mut()) else { return false };
                run.bold = !run.bold;
                return true;
            }
            if toggle_bold_of(&mut el.params, id) {
                return true;
            }
        }
    }
    false
}

/// A et B appliquent chacun leur deck, puis échangent : le deck commun, relu
/// sans `Issue` et identique sur les deux pairs.
fn exchange(a: &mut DeckReplica, b: &mut DeckReplica) -> Deck {
    ok(a.merge(b));
    ok(b.merge(a));
    let (merged, issues) = a.deck();
    assert!(issues.is_empty(), "lecture partielle : {issues:?}");
    assert_eq!(b.deck().0, merged);
    merged
}

/// Ni `issue-unknown-param` ni id en double au `validate` de `deck`.
fn assert_valid_slots(deck: &Deck) {
    let refused = ["issue-unknown-param", "issue-duplicate-element-id"];
    let issues = validate(deck, &common::registry(), None);
    assert!(issues.iter().all(|i| !refused.contains(&i.message.id())), "{issues:?}");
}

/// (propriétaire, slot) de l'unique occurrence de `id`.
fn place<'a>(deck: &'a Deck, id: &str) -> (Option<&'a str>, &'a str) {
    let found = located(deck, id);
    assert_eq!(found.len(), 1, "{id} : {:?}", found.iter().map(|l| (l.owner, l.slot)).collect::<Vec<_>>());
    (found[0].owner, found[0].slot)
}

#[test]
fn concurrent_move_into_a_child_slot_and_edit_converge() {
    let deck = with_pile(&roadmap(), "pile");
    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&with_ops(&deck, nest(&deck, "b-points", "pile")), "A", None));
    let mut bold = deck.clone();
    let Some(slide) = bold.slide_mut("s-constat") else { panic!("s-constat absente") };
    assert!(toggle_bold_of(&mut slide.params, "b-points"));
    assert!(located(&bold, "b-points").iter().all(|l| first_run_bold(l.element)), "B n'a pas basculé b-points");
    ok(b.apply(&bold, "B", None));
    let merged = exchange(&mut a, &mut b);
    assert_eq!(place(&merged, "b-points"), (Some("pile"), "children"));
    assert!(first_run_bold(located(&merged, "b-points")[0].element), "gras de B perdu");
}

#[test]
fn concurrent_moves_of_one_element_to_two_places_agree() {
    let deck = with_pile(&roadmap(), "pile");
    let (mut a, mut b) = peers(&deck);
    let framed = Element { frame: Some(Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 }), ..bare(&deck, "b-points") };
    ok(a.apply(&with_ops(&deck, vec![remove("b-points"), add(None, "elements", &framed)]), "A", None));
    ok(b.apply(&with_ops(&deck, nest(&deck, "b-points", "pile")), "B", None));
    let merged = exchange(&mut a, &mut b);
    let got = place(&merged, "b-points");
    assert!(got == (None, "elements") || got == (Some("pile"), "children"), "couple incohérent : {got:?}");
    assert_valid_slots(&merged);
}

#[test]
fn concurrent_nest_and_reorder_keep_the_element_in_a_declared_slot() {
    let x: Element = ok(serde_json::from_value(
        json!({ "id": "x", "kind": "base/text", "params": { "body": [{ "runs": [{ "t": "Texte x" }] }] } }),
    ));
    let deck = with_ops(&with_pile(&roadmap(), "pile"), vec![add(None, "blocks", &x)]);
    let blocks = |d: &Deck| -> Vec<String> {
        d.slide("s-constat")
            .and_then(|s| s.params.slot("blocks"))
            .unwrap_or_default()
            .iter()
            .map(|e| e.id.clone())
            .collect()
    };
    assert_eq!(blocks(&deck), ["b-points", "pile", "x"]);
    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&with_ops(&deck, nest(&deck, "x", "pile")), "A", None));
    let mut first = deck.clone();
    let Some(list) = first.slide_mut("s-constat").and_then(|s| s.params.slot_mut("blocks")) else { panic!() };
    let moved = list.remove(2);
    list.insert(0, moved);
    ok(b.apply(&first, "B", None));
    let merged = exchange(&mut a, &mut b);
    let got = place(&merged, "x");
    assert!(got == (None, "blocks") || got == (Some("pile"), "children"), "x hors d'un slot déclaré : {got:?}");
    assert_valid_slots(&merged);
}

/// ADR-0006 : l'élément est caché avec le propriétaire retiré ; l'annulation
/// du retrait le rend (sonde du deck en arbre, point 3).
#[test]
fn move_under_a_concurrently_removed_owner_is_hidden_until_undo() {
    let deck = with_pile(&roadmap(), "pile");
    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&with_ops(&deck, vec![remove("pile")]), "A", None));
    ok(b.apply(&with_ops(&deck, nest(&deck, "b-points", "pile")), "B", None));
    let merged = exchange(&mut a, &mut b);
    assert!(located(&merged, "pile").is_empty(), "pile encore là");
    assert!(located(&merged, "b-points").is_empty(), "b-points encore là");
    for r in [&mut a, &mut b] {
        let read = r.deck().0;
        ok(r.apply(&read, "relire", None));
    }
    let again = exchange(&mut a, &mut b);
    assert_eq!(again, merged);
    assert!(ok(a.undo()), "le retrait de A devait s'annuler");
    let restored = exchange(&mut a, &mut b);
    assert_eq!(place(&restored, "pile"), (None, "blocks"));
    assert_eq!(place(&restored, "b-points"), (Some("pile"), "children"));
}

/// Les slides où `id` apparaît, à toutes les profondeurs, une fois par occurrence.
fn slide_of<'a>(deck: &'a Deck, id: &str) -> Vec<&'a str> {
    deck.slides.iter().flat_map(|s| s.walk().filter(|l| l.element.id == id).map(move |_| s.id.as_str())).collect()
}

/// Un `move_element` de `id` (présent dans une seule slide) vers `slot` de `slide`.
fn to_slide(deck: &Deck, id: &str, slide: &str, slot: &str, at: Option<usize>) -> Vec<Op> {
    let from = slide_of(deck, id);
    assert_eq!(from.len(), 1, "{id} : {from:?}");
    vec![op(json!({ "op": "move_element", "slide": from[0], "element": id,
        "to": { "owner": { "slide": slide }, "param": slot }, "index": at }))]
}

/// `deck` après `ops`, et les déplacements entre slides qu'elles annoncent.
fn moving(deck: &Deck, ops: Vec<Op>) -> (Deck, Vec<Moved>) {
    let mut d = deck.clone();
    let moves = apply_all_moving(&mut d, ops, &common::registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    (d, moves)
}

/// `r` écrit `deck` après `ops`, déplacements annoncés compris.
fn apply_ops(r: &mut DeckReplica, deck: &Deck, ops: Vec<Op>, label: &str) {
    let (target, moves) = moving(deck, ops);
    ok(r.apply_moving(&target, label, None, &moves));
}

/// (slide, propriétaire, slot) de l'unique occurrence de `id` dans le deck.
fn spot<'a>(deck: &'a Deck, id: &str) -> (&'a str, Option<&'a str>, &'a str) {
    let found: Vec<_> = deck
        .slides
        .iter()
        .flat_map(|s| s.walk().filter(|l| l.element.id == id).map(move |l| (s.id.as_str(), l.owner, l.slot)))
        .collect();
    assert_eq!(found.len(), 1, "{id} : {found:?}");
    found[0]
}

/// F1 : relire puis réécrire le deck n'écrit rien, sur chaque pair.
fn assert_settled(peers: &mut [&mut DeckReplica], case: &str) {
    for (i, r) in peers.iter_mut().enumerate() {
        let before = r.len_ops();
        let read = r.deck().0;
        ok(r.apply(&read, "relire", None));
        assert_eq!(r.len_ops(), before, "{case} : le pair {i} a écrit en réécrivant le deck relu");
    }
}

fn slide_ids(deck: &Deck) -> Vec<&str> {
    deck.slides.iter().map(|s| s.id.as_str()).collect()
}

fn without_slide(deck: &Deck, id: &str) -> Deck {
    let mut d = deck.clone();
    d.slides.retain(|s| s.id != id);
    d
}

/// Les ids du slot `slot` de la slide `slide`.
fn slot_ids(deck: &Deck, slide: &str, slot: &str) -> Vec<String> {
    let s = deck.slide(slide).unwrap_or_else(|| panic!("{slide} absente"));
    s.params.slot(slot).unwrap_or_default().iter().map(|e| e.id.clone()).collect()
}

#[test]
fn concurrent_move_between_slides_and_nest_agree() {
    let deck = with_pile(&roadmap(), "pile");
    let (mut a, mut b) = peers(&deck);
    apply_ops(&mut a, &deck, to_slide(&deck, "b-points", "s-priorites", "blocks", Some(0)), "A");
    ok(b.apply(&with_ops(&deck, nest(&deck, "b-points", "pile")), "B", None));
    let merged = exchange(&mut a, &mut b);
    let got = spot(&merged, "b-points");
    assert!(
        got == ("s-priorites", None, "blocks") || got == ("s-constat", Some("pile"), "children"),
        "couple incohérent : {got:?}"
    );
    assert_valid_slots(&merged);
    assert_settled(&mut [&mut a, &mut b], "déplacement et imbrication");
}

#[test]
fn concurrent_moves_of_one_element_to_two_slides_agree() {
    let deck = roadmap();
    let (mut a, mut b) = peers(&deck);
    apply_ops(&mut a, &deck, to_slide(&deck, "b-points", "s-priorites", "blocks", None), "A");
    apply_ops(&mut b, &deck, to_slide(&deck, "b-points", "s-libre", "elements", None), "B");
    let merged = exchange(&mut a, &mut b);
    let got = spot(&merged, "b-points");
    assert!(
        got == ("s-priorites", None, "blocks") || got == ("s-libre", None, "elements"),
        "couple incohérent : {got:?}"
    );
    assert_valid_slots(&merged);
    assert_settled(&mut [&mut a, &mut b], "deux déplacements");
}

/// ADR-0006 : l'élément envoyé dans une slide retirée est caché avec elle,
/// et l'annulation du retrait le rend à sa place.
#[test]
fn move_into_a_concurrently_removed_slide_hides_the_element_until_undo() {
    let deck = roadmap();
    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&without_slide(&deck, "s-priorites"), "A", None));
    apply_ops(&mut b, &deck, to_slide(&deck, "b-points", "s-priorites", "blocks", Some(0)), "B");
    let merged = exchange(&mut a, &mut b);
    assert!(merged.slide("s-priorites").is_none(), "s-priorites encore là");
    assert!(slide_of(&merged, "b-points").is_empty(), "b-points encore là");
    assert_settled(&mut [&mut a, &mut b], "slide retirée");
    assert!(ok(a.undo()), "le retrait de A devait s'annuler");
    let restored = exchange(&mut a, &mut b);
    assert_eq!(slot_ids(&restored, "s-priorites", "blocks"), ["b-points", "c1", "c2", "c3"]);
    assert_eq!(slide_of(&restored, "b-points"), ["s-priorites"]);
    assert_settled(&mut [&mut a, &mut b], "retrait annulé");
}

/// Parade par pair, pour une slide : l'annulation de A ne retire que sa marque.
#[test]
fn concurrent_removals_of_a_slide_then_one_undo_keep_it_removed() {
    let deck = roadmap();
    let removed = without_slide(&deck, "s-ch2");
    let check = |a: &mut DeckReplica, b: &mut DeckReplica, case: &str| {
        let merged = exchange(a, b);
        assert!(merged.slide("s-ch2").is_none(), "{case} : s-ch2 revenue");
        assert_eq!(merged, removed, "{case}");
        assert_settled(&mut [a, b], case);
    };

    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&removed, "A", None));
    ok(b.apply(&removed, "B", None));
    exchange(&mut a, &mut b);
    assert!(ok(a.undo()));
    check(&mut a, &mut b, "retraits croisés, puis A annule");

    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&removed, "A", None));
    assert!(ok(a.undo()));
    ok(b.apply(&removed, "B", None));
    check(&mut a, &mut b, "A retire et annule, B retire");

    // A reçoit le retrait de B avant de retirer : rien à marquer, rien à annuler.
    let (mut a, mut b) = peers(&deck);
    ok(b.apply(&removed, "B", None));
    exchange(&mut a, &mut b);
    ok(a.apply(&removed, "A", None));
    exchange(&mut a, &mut b);
    assert!(!ok(a.undo()));
    check(&mut a, &mut b, "B retire, puis A retire et annule");
}

/// Annuler un retrait ne retire que la marque : l'élément reste où B l'a
/// envoyé, avec le gras de B.
#[test]
fn undo_of_an_element_removal_keeps_the_peer_move_to_another_slide() {
    let deck = roadmap();
    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&with_ops(&deck, vec![remove("b-points")]), "A", None));
    let (mut sent, moves) = moving(&deck, to_slide(&deck, "b-points", "s-priorites", "blocks", Some(0)));
    let Some(slide) = sent.slide_mut("s-priorites") else { panic!("s-priorites absente") };
    assert!(toggle_bold_of(&mut slide.params, "b-points"), "B n'a pas basculé b-points");
    ok(b.apply_moving(&sent, "B", None, &moves));
    ok(a.merge(&b));
    assert!(ok(a.undo()), "le retrait de A devait s'annuler");
    let merged = exchange(&mut a, &mut b);
    assert_eq!(slide_of(&merged, "b-points"), ["s-priorites"]);
    let points = merged.slide("s-priorites").and_then(|s| s.locate("b-points")).map(|l| l.element);
    assert!(points.is_some_and(first_run_bold), "gras de B perdu");
    assert_settled(&mut [&mut a, &mut b], "retrait annulé");
}

/// Jitter 0 sur les slides (ADR-0006) : des insertions concurrentes au même
/// rang et un déplacement ne perdent ni ne doublent aucune slide.
///
/// La place de `s-fin` n'est pas vérifiée : le `Rearrange` peut ramener une
/// slide qu'un pair déplaçait, selon l'ordre des pairs (ADR-0006, Limites).
#[test]
fn concurrent_slide_inserts_at_one_place_and_a_reorder_keep_every_slide() {
    let deck = roadmap();
    let registry = Arc::new(common::registry());
    let snap = ok(ok(DeckReplica::new(&deck, registry.clone())).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone())).0;
    let (mut a, mut b, mut c) = (open(), open(), open());
    let inserted = |id: &str| {
        let mut d = deck.clone();
        d.slides.insert(2, Slide::new(id, CONTENT));
        d
    };
    ok(a.apply(&inserted("s-a"), "A", None));
    ok(b.apply(&inserted("s-b"), "B", None));
    let mut first = deck.clone();
    let Some(at) = first.slides.iter().position(|s| s.id == "s-fin") else { panic!("s-fin absente") };
    let fin = first.slides.remove(at);
    first.slides.insert(0, fin);
    ok(c.apply(&first, "C", None));
    let all = |a: &mut DeckReplica, b: &mut DeckReplica, c: &mut DeckReplica| {
        for _ in 0..2 {
            ok(a.merge(b));
            ok(a.merge(c));
            ok(b.merge(a));
            ok(c.merge(a));
        }
        let (merged, issues) = a.deck();
        assert!(issues.is_empty(), "lecture partielle : {issues:?}");
        assert_eq!(b.deck().0, merged);
        assert_eq!(c.deck().0, merged);
        merged
    };
    let merged = all(&mut a, &mut b, &mut c);
    let ids = slide_ids(&merged);
    let (Some(ia), Some(ib)) = (ids.iter().position(|s| *s == "s-a"), ids.iter().position(|s| *s == "s-b")) else {
        panic!("s-a ou s-b perdue : {ids:?}")
    };
    let mut between = merged.clone();
    between.slides.insert(ia.max(ib), Slide::new("s-c", CONTENT));
    ok(a.apply(&between, "A", None));
    let last = all(&mut a, &mut b, &mut c);
    let mut got: Vec<&str> = slide_ids(&last);
    got.sort_unstable();
    let mut want: Vec<&str> = slide_ids(&deck);
    want.extend(["s-a", "s-b", "s-c"]);
    want.sort_unstable();
    assert_eq!(got, want);
    assert_settled(&mut [&mut a, &mut b, &mut c], "trois pairs");
}

/// Vider une slide dans une autre puis la retirer, dans une transaction :
/// le déplacement annoncé fait foi, l'élément garde le gras de B.
#[test]
fn move_then_removal_of_the_source_slide_keeps_the_peer_edit() {
    let deck = roadmap();
    let (mut a, mut b) = peers(&deck);
    let mut ops = to_slide(&deck, "b-points", "s-priorites", "blocks", Some(0));
    ops.push(op(json!({ "op": "remove_slide", "slide": "s-constat" })));
    apply_ops(&mut a, &deck, ops, "A");
    let mut bold = deck.clone();
    let Some(slide) = bold.slide_mut("s-constat") else { panic!("s-constat absente") };
    assert!(toggle_bold_of(&mut slide.params, "b-points"), "B n'a pas basculé b-points");
    ok(b.apply(&bold, "B", None));
    let merged = exchange(&mut a, &mut b);
    assert!(merged.slide("s-constat").is_none(), "s-constat encore là");
    assert_eq!(slide_of(&merged, "b-points"), ["s-priorites"]);
    let points = merged.slide("s-priorites").and_then(|s| s.locate("b-points")).map(|l| l.element);
    assert!(points.is_some_and(first_run_bold), "gras de B perdu");
    assert_settled(&mut [&mut a, &mut b], "déplacement puis retrait de la source");
}

/// Le même geste, avec le retrait d'un enfant de l'élément déplacé : seul
/// cet enfant disparaît, et les deux pairs relisent le deck écrit.
#[test]
fn move_then_removal_of_the_source_slide_drops_a_removed_child() {
    let child = |id: &str| -> Element {
        ok(serde_json::from_value(
            json!({ "id": id, "kind": "base/text", "params": { "body": [{ "runs": [{ "t": id }] }] } }),
        ))
    };
    let deck = with_pile(&roadmap(), "pile");
    let deck =
        with_ops(&deck, vec![add(Some("pile"), "children", &child("k1")), add(Some("pile"), "children", &child("k2"))]);
    let (mut a, mut b) = peers(&deck);
    let mut ops = to_slide(&deck, "pile", "s-priorites", "blocks", Some(0));
    ops.push(op(json!({ "op": "remove_element", "slide": "s-priorites", "element": "k1" })));
    ops.push(op(json!({ "op": "remove_slide", "slide": "s-constat" })));
    let (target, moves) = moving(&deck, ops);
    ok(a.apply_moving(&target, "A", None, &moves));
    assert_eq!(a.deck().0, target);
    let merged = exchange(&mut a, &mut b);
    assert_eq!(merged, target);
    assert!(slide_of(&merged, "k1").is_empty(), "k1 encore là");
    assert_eq!(slide_of(&merged, "k2"), ["s-priorites"]);
    assert_settled(&mut [&mut a, &mut b], "déplacement, retrait d'un enfant et de la source");
}
