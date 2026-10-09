//! Convergence des déplacements dans l'arbre des éléments d'une slide (t-694).

mod common;

use std::sync::Arc;

use oku_slides_core::{Deck, DeckReplica, Element, Frame, Located, Op, ParamValue, Params, apply_all, validate};
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
fn with_pile(deck: Deck, id: &str) -> Deck {
    with_ops(&deck, vec![add(None, "blocks", &Element::new(id, "stack"))])
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
    let deck = with_pile(roadmap(), "pile");
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
    let deck = with_pile(roadmap(), "pile");
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
        json!({ "id": "x", "kind": "text", "params": { "body": [{ "runs": [{ "t": "Texte x" }] }] } }),
    ));
    let deck = with_ops(&with_pile(roadmap(), "pile"), vec![add(None, "blocks", &x)]);
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

/// Comportement documenté (ADR-0005, Limites ; sonde, point 3) : un élément
/// déplacé sous un propriétaire retiré en parallèle disparaît avec lui.
#[test]
fn move_under_a_concurrently_removed_owner_is_a_documented_loss() {
    let deck = with_pile(roadmap(), "pile");
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
}
