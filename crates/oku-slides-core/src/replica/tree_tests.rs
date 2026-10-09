//! L'arbre des éléments d'une slide : écriture, lecture, placement.

use std::sync::Arc;

use loro::{Container, LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::super::schema::ELEMENTS;
use super::super::{DeckReplica, ROOT};
use super::*;
use crate::error::ReplicaErrorKind;
use crate::{Deck, Element, MAX_SLOT_DEPTH, Op, ParamValue, Slide, WidgetRegistry, apply_all};

pub(super) fn registry() -> Arc<WidgetRegistry> {
    crate::test_registry::shared()
}

pub(super) fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

pub(super) fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

pub(super) fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

/// `ops` appliquées à une copie de `deck`, sous forme canonique.
pub(super) fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    ok(apply_all(&mut d, ops, &crate::test_registry::registry()));
    d
}

pub(super) fn deck_of(v: Value) -> Deck {
    ok(serde_json::from_value(v))
}

pub(super) fn ops_of(r: &DeckReplica) -> usize {
    r.len_ops()
}

/// Un texte `id` de corps `t`.
pub(super) fn text(id: &str, t: &str) -> Value {
    json!({ "id": id, "kind": "text", "params": { "body": [{ "runs": [{ "t": t }] }] } })
}

/// Ajoute `element` au slot `param` de `owner` (`None` : la slide `s-constat`).
fn add(owner: Option<&str>, param: &str, index: Option<usize>, element: Value) -> Op {
    let owner = match owner {
        Some(e) => json!({ "slide": "s-constat", "element": e }),
        None => json!({ "slide": "s-constat" }),
    };
    op(json!({ "op": "add_element", "slot": { "owner": owner, "param": param }, "index": index, "element": element }))
}

fn remove(id: &str) -> Op {
    op(json!({ "op": "remove_element", "slide": "s-constat", "element": id }))
}

/// `roadmap` et un texte cadré `n` dans `elements` de `s-constat`.
pub(super) fn two_slots() -> Deck {
    let mut n = text("n", "Note libre");
    n["frame"] = json!({ "x": 100, "y": 100, "w": 400, "h": 200 });
    with_ops(&roadmap(), vec![add(None, "elements", None, n)])
}

/// `roadmap` et un `stack` `pile` dans `blocks` de `s-constat`.
fn with_pile() -> Deck {
    with_ops(&roadmap(), vec![add(None, "blocks", None, json!({ "id": "pile", "kind": "stack" }))])
}

pub(super) fn new(deck: &Deck) -> DeckReplica {
    ok(DeckReplica::new(deck, registry()))
}

/// Le deck relu d'un snapshot de `r`, et ses `Issue`.
pub(super) fn reread(r: &DeckReplica) -> crate::DeckRead {
    ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry())).1
}

pub(super) fn apply(r: &mut DeckReplica, deck: &Deck) {
    ok(r.apply(deck, "test", None));
}

pub(super) fn exchange(a: &mut DeckReplica, b: &mut DeckReplica) {
    ok(a.merge(b));
    ok(b.merge(a));
}

pub(super) fn as_map(v: Option<ValueOrContainer>) -> LoroMap {
    match v {
        Some(ValueOrContainer::Container(Container::Map(m))) => m,
        other => panic!("carte attendue : {other:?}"),
    }
}

/// La carte de la slide `slide`.
pub(super) fn slide_map(r: &DeckReplica, slide: &str) -> LoroMap {
    let Some(ValueOrContainer::Container(Container::MovableList(slides))) = r.doc.get_map(ROOT).get("slides") else {
        panic!("pas de liste `slides`")
    };
    (0..slides.len())
        .map(|i| as_map(slides.get(i)))
        .find(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if *s == slide))
        .unwrap_or_else(|| panic!("slide {slide} absente"))
}

/// L'arbre de la slide `slide`.
pub(super) fn slide_tree(r: &DeckReplica, slide: &str) -> LoroTree {
    match slide_map(r, slide).get(ELEMENTS) {
        Some(ValueOrContainer::Container(Container::Tree(t))) => t,
        other => panic!("arbre attendu : {other:?}"),
    }
}

/// La carte `@slot` de la méta de `node`, en JSON.
pub(super) fn slots(tree: &LoroTree, node: TreeID) -> Value {
    match ok(tree.get_meta(node)).get(SLOT) {
        Some(ValueOrContainer::Container(Container::Map(m))) => crate::replica::read::loro_to_json(&m.get_value()),
        other => panic!("carte `@slot` attendue : {other:?}"),
    }
}

/// Forge un nœud sous `parent` (id d'un élément, ou la racine), avec la méta
/// `meta` puis, si `slot` est donné, la carte `@slot`.
pub(super) fn forge(
    r: &DeckReplica,
    slide: &str,
    parent: Option<&str>,
    meta: &[(&str, LoroValue)],
    slot: Option<&[(&str, &str)]>,
) -> TreeID {
    let tree = slide_tree(r, slide);
    let parent = match parent {
        Some(id) => TreeParentId::Node(node_of(&r.doc, slide, id).1),
        None => TreeParentId::Root,
    };
    let node = ok(tree.create(parent));
    let m = ok(tree.get_meta(node));
    for (k, v) in meta {
        ok(m.insert(k, v.clone()));
    }
    if let Some(slot) = slot {
        let s = ok(m.insert_container(SLOT, LoroMap::new()));
        for (k, v) in slot {
            ok(s.insert(k, *v));
        }
    }
    r.doc.commit();
    node
}

pub(super) fn text_meta(id: &str) -> Vec<(&str, LoroValue)> {
    vec![("id", LoroValue::from(id)), ("kind", LoroValue::from("text"))]
}

/// Les ids du slot `name` de l'élément `owner` (ou de la slide) de `slide`.
pub(super) fn ids_in(deck: &Deck, slide: &str, owner: Option<&str>, name: &str) -> Vec<String> {
    let s = deck.slide(slide).unwrap_or_else(|| panic!("slide {slide} absente"));
    let params = match owner {
        Some(o) => &s.locate(o).unwrap_or_else(|| panic!("{o} absent")).element.params,
        None => &s.params,
    };
    params.slot(name).unwrap_or_default().iter().map(|e| e.id.clone()).collect()
}

/// Les ids des racines de l'arbre de `slide`, dans l'ordre brut de Loro.
fn root_ids(r: &DeckReplica, slide: &str) -> Vec<String> {
    let tree = slide_tree(r, slide);
    let roots = tree.children(TreeParentId::Root).unwrap_or_default();
    roots.into_iter().map(|n| name(ok(tree.get_meta(n)).get("id")).unwrap_or_default()).collect()
}

pub(super) fn issue_ids(issues: &[crate::Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

/// Le texte du premier run du `body` de `e`.
pub(super) fn first_text(e: &Element) -> String {
    match e.params.get("body") {
        Some(ParamValue::Rich(b)) => b.first().and_then(|p| p.runs.first()).map(|r| r.t.clone()).unwrap_or_default(),
        _ => String::new(),
    }
}

/// Un deck d'une slide `s` dont `blocks` porte les textes `ids`.
fn texts(ids: &[&str]) -> Deck {
    let blocks: Vec<Value> = ids.iter().map(|id| text(id, id)).collect();
    deck_of(json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": blocks } }] }))
}

/// `deck` dont la slide `s` porte les textes `ids` dans `blocks`, dans cet ordre.
fn reordered(deck: &Deck, ids: &[&str]) -> Deck {
    let mut d = deck.clone();
    let Some(blocks) = d.slide_mut("s").and_then(|s| s.params.slot_mut("blocks")) else { panic!("blocks absent") };
    let mut by_id: Vec<Element> = std::mem::take(blocks);
    for id in ids {
        let at = by_id.iter().position(|e| e.id == *id).unwrap_or_else(|| panic!("{id} absent"));
        blocks.push(by_id.remove(at));
    }
    d
}

#[test]
fn reserved_keys_never_collide_with_model_fields() {
    let element: Element = ok(serde_json::from_value(json!({
        "id": "e", "kind": "text",
        "frame": { "x": 1, "y": 2, "w": 3, "h": 4 },
        "cell": { "col": 1, "row": 1, "cols": 2, "rows": 1 },
        "style": { "color": "accent", "size": "lead" },
        "params": { "body": [{ "runs": [{ "t": "x" }] }] }
    })));
    let slide: Slide = ok(serde_json::from_value(json!({
        "id": "s", "kind": "content", "name": "Modèle", "description": "Une slide", "notes": "Notes",
        "params": { "title": "Titre", "blocks": [ok(serde_json::to_value(&element))] }
    })));
    for v in [ok(serde_json::to_value(&element)), ok(serde_json::to_value(&slide))] {
        let keys: Vec<String> = v.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
        assert_that!(keys.len()).is_greater_than(3);
        assert_that!(keys.iter().any(|k| k.starts_with('@'))).is_false();
    }
}

#[test]
fn param_named_like_a_reserved_key_stays_in_params() {
    let mut deck = roadmap();
    let Some(blocks) = deck.slide_mut("s-constat").and_then(|s| s.params.slot_mut("blocks")) else { panic!() };
    blocks[0].params.0.insert(SLOT.to_owned(), ParamValue::Text("pas un slot".into()));
    let r = new(&deck);
    let (read, issues) = reread(&r);
    assert_that!(read).is_equal_to(&deck);
    assert_that!(issues).is_empty();
    let (tree, node) = node_of(&r.doc, "s-constat", "b-points");
    assert_that!(slots(&tree, node)).is_equal_to(json!({ "": "blocks" }));
}

#[test]
fn reconcile_slide_elements_live_in_the_slide_tree() {
    let r = new(&roadmap());
    let slide = slide_map(&r, "s-constat");
    assert_that!(matches!(slide.get(ELEMENTS), Some(ValueOrContainer::Container(Container::Tree(_))))).is_true();
    assert_that!(as_map(slide.get("params")).get("blocks").is_none()).is_true();
    let (tree, node) = node_of(&r.doc, "s-constat", "b-points");
    assert_that!(slots(&tree, node)).is_equal_to(json!({ "": "blocks" }));
}

#[test]
fn round_trip_elements_in_two_slots_is_identical() {
    let deck = two_slots();
    let (read, issues) = reread(&new(&deck));
    assert_that!(read).is_equal_to(&deck);
    assert_that!(issues).is_empty();
}

#[test]
fn round_trip_nested_children_is_identical() {
    let deck = with_ops(
        &with_pile(),
        vec![
            add(Some("pile"), "children", None, text("k1", "Un")),
            add(Some("pile"), "children", None, text("k2", "Deux")),
        ],
    );
    let (read, issues) = reread(&new(&deck));
    assert_that!(read).is_equal_to(&deck);
    assert_that!(issues).is_empty();
    assert_that!(ids_in(&read, "s-constat", Some("pile"), "children")).is_equal_to(vec!["k1".into(), "k2".into()]);
    let json = ok(serde_json::to_value(read.slide("s-constat")));
    for params in [&json["params"], &json["params"]["blocks"][1]["params"]] {
        let keys: Vec<&String> = params.as_object().map(|o| o.keys().collect()).unwrap_or_default();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_that!(keys).is_equal_to(sorted);
    }
}

#[test]
fn apply_move_across_slots_keeps_the_tree_node() {
    let deck = two_slots();
    let mut r = new(&deck);
    let (_, before) = node_of(&r.doc, "s-constat", "b-points");
    let Some(mut moved) = deck.slide("s-constat").and_then(|s| s.locate("b-points")).map(|l| l.element.clone()) else {
        panic!("b-points absent")
    };
    moved.frame = Some(crate::Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 });
    let target =
        with_ops(&deck, vec![remove("b-points"), add(None, "elements", None, ok(serde_json::to_value(moved)))]);
    apply(&mut r, &target);
    let (tree, after) = node_of(&r.doc, "s-constat", "b-points");
    assert_that!(after).is_equal_to(before);
    assert_that!(slots(&tree, after)[""]).is_equal_to(json!("elements"));
    assert_that!(r.deck().0).is_equal_to(&target);
}

#[test]
fn apply_move_into_a_child_slot_keeps_the_tree_node() {
    let deck = with_pile();
    let mut r = new(&deck);
    let (_, before) = node_of(&r.doc, "s-constat", "b-points");
    let Some(moved) = deck.slide("s-constat").and_then(|s| s.locate("b-points")).map(|l| l.element.clone()) else {
        panic!("b-points absent")
    };
    let target =
        with_ops(&deck, vec![remove("b-points"), add(Some("pile"), "children", None, ok(serde_json::to_value(moved)))]);
    apply(&mut r, &target);
    let (tree, after) = node_of(&r.doc, "s-constat", "b-points");
    let (_, pile) = node_of(&r.doc, "s-constat", "pile");
    assert_that!(after).is_equal_to(before);
    assert_that!(tree.parent(after)).is_equal_to(Some(TreeParentId::Node(pile)));
    assert_that!(slots(&tree, after)).is_equal_to(json!({ "": "blocks", "pile": "children" }));
    assert_that!(r.deck().0).is_equal_to(&target);
}

#[test]
fn apply_reorder_of_elements_is_one_change() {
    let deck = texts(&["a", "b", "c", "d"]);
    let mut r = new(&deck);
    let c0 = r.len_changes();
    let mut titled = deck.clone();
    let Some(s) = titled.slide_mut("s") else { panic!() };
    s.params.0.insert("title".into(), ParamValue::Text("Titre".into()));
    apply(&mut r, &titled);
    let c1 = r.len_changes();
    let reversed = reordered(&titled, &["d", "c", "b", "a"]);
    apply(&mut r, &reversed);
    assert_that!(r.len_changes() - c1).is_equal_to(c1 - c0);
    assert_that!(ids_in(&r.deck().0, "s", None, "blocks")).is_equal_to(vec![
        "d".to_owned(),
        "c".into(),
        "b".into(),
        "a".into(),
    ]);
}

#[test]
fn moving_the_first_of_three_to_the_end_is_one_move() {
    let deck = texts(&["a", "b", "c"]);
    let mut r = new(&deck);
    let (tree, a) = node_of(&r.doc, "s", "a");
    let before = ops_of(&r);
    apply(&mut r, &reordered(&deck, &["b", "c", "a"]));
    assert_that!(ops_of(&r) - before).is_equal_to(1);
    assert_that!(slots(&tree, a)).is_equal_to(json!({ "": "blocks" }));
    assert_that!(ids_in(&r.deck().0, "s", None, "blocks")).is_equal_to(vec!["b".to_owned(), "c".into(), "a".into()]);
}

#[test]
fn lis_marks_a_longest_increasing_subsequence() {
    assert_that!(lis(&[2, 0, 1])).is_equal_to(vec![false, true, true]);
    assert_that!(lis(&[])).is_equal_to(Vec::<bool>::new());
    assert_that!(lis(&[3, 2, 1, 0]).iter().filter(|s| **s).count()).is_equal_to(1);
    assert_that!(lis(&[0, 1, 2])).is_equal_to(vec![true, true, true]);
}

/// Un doublon imbriqué : la clé `(x, 0)` est le `x` de `blocks`, comme le
/// garde `unique_ids`, et le `TreeID` de celui-ci ne change pas. L'enfant de
/// `pile`, `(x, 1)`, est relu `x-2` : le premier `apply` du deck relu le
/// recrée une fois sous ce nom (réparation écrite une fois, comme les listes
/// avec t-652) ; le second `apply` n'écrit plus rien.
#[test]
fn nested_duplicate_keeps_the_outer_tree_id() {
    let pile = json!({ "id": "pile", "kind": "stack", "params": { "children": [text("x", "A")] } });
    let deck = deck_of(json!({ "slides": [{ "id": "s", "kind": "content", "params": {
        "blocks": [pile, text("x", "B")] } }] }));
    let mut r = new(&deck);
    let tree = slide_tree(&r, "s");
    let root_x = |tree: &LoroTree| {
        tree.children(TreeParentId::Root)
            .unwrap_or_default()
            .into_iter()
            .find(|n| tree.get_meta(*n).is_ok_and(|m| name(m.get("id")).as_deref() == Some("x")))
            .unwrap_or_else(|| panic!("x absent de la racine"))
    };
    let outer = root_x(&tree);
    let (read, _) = reread(&r);
    assert_that!(ids_in(&read, "s", None, "blocks")).is_equal_to(vec!["pile".to_owned(), "x".into()]);
    assert_that!(ids_in(&read, "s", Some("pile"), "children")).is_equal_to(vec!["x-2".to_owned()]);
    let slide = read.slide("s").unwrap_or_else(|| panic!());
    assert_that!(first_text(slide.locate("x").unwrap_or_else(|| panic!()).element)).is_equal_to("B".to_owned());
    assert_that!(first_text(slide.locate("x-2").unwrap_or_else(|| panic!()).element)).is_equal_to("A".to_owned());

    let before = ops_of(&r);
    apply(&mut r, &read);
    assert_that!(ops_of(&r)).is_greater_than(before);
    assert_that!(root_x(&tree)).is_equal_to(outer);
    let (_, inner) = node_of(&r.doc, "s", "x-2");
    assert_that!(tree.parent(inner)).is_equal_to(Some(TreeParentId::Node(node_of(&r.doc, "s", "pile").1)));
    let after_first = ops_of(&r);
    apply(&mut r, &read);
    assert_that!(ops_of(&r)).is_equal_to(after_first);
    assert_that!(reread(&r)).is_equal_to((read, Vec::new()));
}

/// Un deck d'une slide `blank` portant une chaîne de `n` canvas imbriqués
/// (copie de `sources_tests.rs`).
fn canvas_chain(n: usize) -> Deck {
    let mut tree = json!({ "id": format!("c{}", n - 1), "kind": "canvas" });
    for i in (0..n - 1).rev() {
        tree = json!({ "id": format!("c{i}"), "kind": "canvas", "params": { "children": [tree] } });
    }
    deck_of(json!({ "slides": [{ "id": "s", "kind": "blank", "params": { "elements": [tree] } }] }))
}

#[test]
fn apply_of_a_too_deep_deck_twice_does_not_grow() {
    let mut r = new(&canvas_chain(MAX_SLOT_DEPTH));
    let deep = canvas_chain(MAX_SLOT_DEPTH + 1);
    let Err(first) = r.apply(&deep, "trop", None) else { panic!("un deck trop profond devait être refusé") };
    assert_that!(first.kind()).is_equal_to(ReplicaErrorKind::Write);
    let after_first = ops_of(&r);
    let Err(second) = r.apply(&deep, "trop", None) else { panic!("un deck trop profond devait être refusé") };
    assert_that!(second.kind()).is_equal_to(ReplicaErrorKind::Write);
    assert_that!(ops_of(&r)).is_equal_to(after_first);
}

#[test]
fn apply_of_a_too_deep_deck_leaves_the_slide_untouched() {
    let mut r = new(&canvas_chain(MAX_SLOT_DEPTH));
    let before = r.deck().0;
    let mut deep = canvas_chain(MAX_SLOT_DEPTH + 1);
    let Some(slide) = deep.slide_mut("s") else { panic!("slide s absente") };
    slide.name = Some("Renommée".into());
    slide.notes = "Notes changées".into();
    let ops = ops_of(&r);
    assert_that!(r.apply(&deep, "trop", None).is_err()).is_true();
    assert_that!(ops_of(&r)).is_equal_to(ops);
    assert_that!(r.deck().0).is_equal_to(&before);
}

#[test]
fn removing_the_first_of_three_writes_one_delete() {
    let deck = texts(&["a", "b", "c"]);
    let mut r = new(&deck);
    let before = ops_of(&r);
    let mut target = deck.clone();
    let Some(blocks) = target.slide_mut("s").and_then(|s| s.params.slot_mut("blocks")) else { panic!() };
    blocks.remove(0);
    apply(&mut r, &target);
    assert_that!(ops_of(&r) - before).is_equal_to(1);
    assert_that!(r.deck().0).is_equal_to(&target);
}

#[test]
fn reapply_after_a_move_across_slots_writes_nothing() {
    let deck = two_slots();
    let mut r = new(&deck);
    let Some(moved) = deck.slide("s-constat").and_then(|s| s.locate("n")).map(|l| l.element.clone()) else {
        panic!("n absent")
    };
    let moved = Element { frame: None, ..moved };
    let target = with_ops(&deck, vec![remove("n"), add(None, "blocks", Some(0), ok(serde_json::to_value(moved)))]);
    apply(&mut r, &target);
    assert_that!(ids_in(&r.deck().0, "s-constat", None, "blocks")).is_equal_to(vec!["n".to_owned(), "b-points".into()]);
    assert_that!(ids_in(&r.deck().0, "s-constat", None, "elements")).is_empty();
    let before = ops_of(&r);
    let read = r.deck().0;
    apply(&mut r, &read);
    assert_that!(ops_of(&r)).is_equal_to(before);
    assert_that!(r.deck().0).is_equal_to(&target);
}

#[test]
fn reapply_after_concurrent_inserts_in_two_slots_writes_nothing() {
    let deck = two_slots();
    let snap = ok(new(&deck).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
    let (mut a, mut b) = (open(), open());
    ok(a.doc.set_peer_id(1));
    ok(b.doc.set_peer_id(2));
    let mut free = text("na", "Libre A");
    free["frame"] = json!({ "x": 0, "y": 0, "w": 100, "h": 100 });
    apply(&mut a, &with_ops(&deck, vec![add(None, "elements", Some(0), free)]));
    apply(&mut b, &with_ops(&deck, vec![add(None, "blocks", None, text("nb", "Flux B"))]));
    exchange(&mut a, &mut b);
    // Les deux slots entrelacent leurs nœuds dans l'ordre brut des enfants.
    for r in [&a, &b] {
        assert_that!(root_ids(r, "s-constat")).is_equal_to(vec![
            "b-points".to_owned(),
            "na".into(),
            "nb".into(),
            "n".into(),
        ]);
    }
    for r in [&a, &b] {
        let merged = r.deck().0;
        assert_that!(ids_in(&merged, "s-constat", None, "blocks"))
            .is_equal_to(vec!["b-points".to_owned(), "nb".into()]);
        assert_that!(ids_in(&merged, "s-constat", None, "elements")).is_equal_to(vec!["na".to_owned(), "n".into()]);
    }
    for r in [&mut a, &mut b] {
        let before = ops_of(r);
        let read = r.deck().0;
        apply(r, &read);
        assert_that!(ops_of(r)).is_equal_to(before);
    }
    let (ops_a, ops_b) = (ops_of(&a), ops_of(&b));
    exchange(&mut a, &mut b);
    assert_that!((ops_of(&a), ops_of(&b))).is_equal_to((ops_a, ops_b));
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
}

#[test]
fn concurrent_first_elements_on_a_new_slide_are_both_kept() {
    let mut deck = roadmap();
    deck.slides.push(Slide::new("s-neuve", "content"));
    let snap = ok(new(&deck).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
    let (mut a, mut b) = (open(), open());
    let add_to = |id: &str| {
        op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-neuve" }, "param": "blocks" },
            "element": text(id, id) }))
    };
    apply(&mut a, &with_ops(&deck, vec![add_to("e1")]));
    apply(&mut b, &with_ops(&deck, vec![add_to("e2")]));
    exchange(&mut a, &mut b);
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&merged);
    let mut got = ids_in(&merged, "s-neuve", None, "blocks");
    got.sort();
    assert_that!(got).is_equal_to(vec!["e1".to_owned(), "e2".into()]);
}

#[test]
fn removing_an_owner_removes_its_subtree() {
    let deck = with_ops(
        &with_pile(),
        vec![
            add(Some("pile"), "children", None, text("k1", "Un")),
            add(Some("pile"), "children", None, text("k2", "Deux")),
        ],
    );
    let mut r = new(&deck);
    let target = with_ops(&deck, vec![remove("pile")]);
    apply(&mut r, &target);
    let tree = slide_tree(&r, "s-constat");
    let live: Vec<Option<String>> = tree
        .nodes()
        .into_iter()
        .filter(|n| !ok(tree.is_node_deleted(n)))
        .map(|n| name(ok(tree.get_meta(n)).get("id")))
        .collect();
    assert_that!(live).is_equal_to(vec![Some("b-points".to_owned())]);
    assert_that!(reread(&r)).is_equal_to((target, Vec::new()));
}

#[test]
fn removing_an_owner_while_moving_its_child_out_keeps_the_child() {
    let deck = with_ops(&with_pile(), vec![add(Some("pile"), "children", None, text("k", "Gardé"))]);
    let mut r = new(&deck);
    let (_, k) = node_of(&r.doc, "s-constat", "k");
    let target = with_ops(&deck, vec![remove("k"), remove("pile"), add(None, "blocks", None, text("k", "Gardé"))]);
    apply(&mut r, &target);
    assert_that!(ids_in(&r.deck().0, "s-constat", None, "blocks")).is_equal_to(vec!["b-points".to_owned(), "k".into()]);
    assert_that!(node_of(&r.doc, "s-constat", "k").1).is_equal_to(k);
    assert_that!(reread(&r)).is_equal_to((target, Vec::new()));
}

#[test]
fn slots_of_takes_declared_and_shaped_slots_sorted() {
    let el = json!({ "id": "e", "kind": "text" });
    let params = json!({ "zzz": [el], "blocks": [el], "title": "x", "": [el] });
    let registry = registry();
    let schema = registry.get("content").map(|w| w.params.as_slice());
    let got: Vec<&str> = slots_of(params.as_object(), schema).into_iter().map(|(name, _)| name).collect();
    assert_that!(got).is_equal_to(vec!["blocks", "zzz"]);
}

/// Une chaîne de `n` `stack` forgée dans `blocks` de `s-constat` : `c0` à la racine.
fn forge_chain(r: &DeckReplica, n: usize) {
    forge(
        r,
        "s-constat",
        None,
        &[("id", LoroValue::from("c0")), ("kind", LoroValue::from("stack"))],
        Some(&[("", "blocks")]),
    );
    for i in 1..n {
        let meta = [("id", LoroValue::from(format!("c{i}").as_str())), ("kind", LoroValue::from("stack"))];
        let owner = format!("c{}", i - 1);
        forge(r, "s-constat", Some(&owner), &meta, Some(&[(owner.as_str(), "children")]));
    }
}

#[test]
fn apply_on_a_forged_deep_chain_leaves_unread_nodes_untouched() {
    let r = new(&roadmap());
    forge_chain(&r, 200);
    let (r, outcome) = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut r = r;
            let deck = r.deck().0;
            let outcome = r.apply(&deck, "test", None);
            (r, outcome)
        })
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("l'écriture a paniqué ou débordé la pile"));
    ok(outcome);
    let tree = slide_tree(&r, "s-constat");
    let live: Vec<String> = tree
        .nodes()
        .into_iter()
        .filter(|n| !ok(tree.is_node_deleted(n)))
        .filter_map(|n| name(ok(tree.get_meta(n)).get("id")))
        .collect();
    for i in 0..200 {
        assert_that!(live.contains(&format!("c{i}"))).named(&format!("c{i}")).is_true();
    }
    assert_that!(live.len()).is_equal_to(201);
    assert_that!(MAX_SLOT_DEPTH).is_less_than(200);
}

#[test]
fn apply_with_elements_of_another_type_fails_without_overwriting() {
    let deck = roadmap();
    let mut r = new(&deck);
    ok(slide_map(&r, "s-constat").insert_container(ELEMENTS, LoroMap::new()));
    r.doc.commit();
    let mut titled = deck.clone();
    let Some(slide) = titled.slide_mut("s-constat") else { panic!("s-constat absente") };
    slide.params.set("title", ParamValue::Text("Autre titre".into()));
    let Err(e) = r.apply(&titled, "test", None) else { panic!("un `@elements` d'un autre type devait être refusé") };
    assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
    assert_that!(matches!(
        slide_map(&r, "s-constat").get(ELEMENTS),
        Some(ValueOrContainer::Container(Container::Map(_)))
    ))
    .is_true();
    let read = r.deck().0;
    let title = |d: &Deck| d.slide("s-constat").and_then(|s| s.params.get("title").cloned());
    assert_that!(title(&read)).is_equal_to(title(&deck));
}

/// Un snapshot dont la slide `s-constat` porte un `@elements` qui n'est pas un arbre.
fn snapshot_with_map_elements(r: &DeckReplica) -> Vec<u8> {
    ok(slide_map(r, "s-constat").insert_container(ELEMENTS, LoroMap::new()));
    r.doc.commit();
    ok(r.snapshot())
}

#[test]
fn from_snapshot_with_elements_of_another_type_is_refused_as_corrupted() {
    let r = new(&roadmap());
    let bytes = snapshot_with_map_elements(&r);
    let Err(e) = DeckReplica::from_snapshot(&bytes, 1 << 24, registry()) else {
        panic!("un `@elements` d'un autre type devait être refusé à l'import")
    };
    assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

#[test]
fn merge_of_elements_of_another_type_is_refused_and_keeps_self() {
    let r = new(&roadmap());
    let mut a = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry())).0;
    let b = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry())).0;
    snapshot_with_map_elements(&b);
    let before = a.deck();
    let Err(e) = a.merge(&b) else { panic!("un `@elements` d'un autre type devait être refusé à la fusion") };
    assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
    assert_that!(a.deck()).is_equal_to(before);
    assert_that!(matches!(
        slide_map(&a, "s-constat").get(ELEMENTS),
        Some(ValueOrContainer::Container(Container::Tree(_)))
    ))
    .is_true();
}

#[test]
fn empty_param_name_with_elements_round_trips() {
    let host = json!({ "id": "host", "kind": "stack", "params": { "": [text("dedans", "Dedans")] } });
    let deck = deck_of(json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [host] } }] }));
    let mut r = new(&deck);
    let tree = slide_tree(&r, "s");
    let live: Vec<Option<String>> = tree
        .nodes()
        .into_iter()
        .filter(|n| !ok(tree.is_node_deleted(n)))
        .map(|n| name(ok(tree.get_meta(n)).get("id")))
        .collect();
    assert_that!(live).is_equal_to(vec![Some("host".to_owned())]);
    assert_that!(reread(&r)).is_equal_to((deck.clone(), Vec::new()));
    let before = ops_of(&r);
    apply(&mut r, &deck);
    assert_that!(ops_of(&r)).is_equal_to(before);
    assert_that!(r.deck().0).is_equal_to(&deck);
}

fn meta_of(r: &DeckReplica) -> LoroMap {
    r.doc.get_map("meta")
}

fn format_of_meta(r: &DeckReplica) -> Option<LoroValue> {
    match meta_of(r).get("format") {
        Some(ValueOrContainer::Value(v)) => Some(v),
        _ => None,
    }
}

/// Le snapshot de `r` après `edit` sur sa carte `meta`.
fn snapshot_with_meta(r: &DeckReplica, edit: impl FnOnce(&LoroMap)) -> Vec<u8> {
    edit(&meta_of(r));
    r.doc.commit();
    ok(r.snapshot())
}

fn refused_kind(bytes: &[u8]) -> ReplicaErrorKind {
    match DeckReplica::from_snapshot(bytes, 1 << 24, registry()) {
        Err(e) => e.kind(),
        Ok(_) => panic!("un replica d'un autre format devait être refusé"),
    }
}

#[test]
fn new_replica_carries_the_replica_format() {
    let r = new(&roadmap());
    assert_that!(format_of_meta(&r)).is_equal_to(Some(LoroValue::I64(2)));
}

#[test]
fn from_snapshot_without_replica_format_is_refused() {
    let r = new(&roadmap());
    let bytes = snapshot_with_meta(&r, |m| ok(m.delete("format")));
    assert_that!(refused_kind(&bytes)).is_equal_to(ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn from_snapshot_of_another_replica_format_is_refused() {
    let r = new(&roadmap());
    let bytes = snapshot_with_meta(&r, |m| ok(m.insert("format", 3_i64)));
    assert_that!(refused_kind(&bytes)).is_equal_to(ReplicaErrorKind::UnsupportedVersion);
}

/// Un replica v7 : pas de `meta.format`, éléments dans une `LoroMovableList`.
#[test]
fn from_snapshot_of_a_v7_shaped_replica_is_refused() {
    let doc = loro::LoroDoc::new();
    ok(doc.get_map("meta").insert("deck_uid", "uid-v7"));
    let deck = doc.get_map("deck");
    ok(deck.insert("version", 5_i64));
    let slides = ok(deck.insert_container("slides", loro::LoroMovableList::new()));
    let slide = ok(slides.push_container(LoroMap::new()));
    ok(slide.insert("id", "s"));
    ok(slide.insert("kind", "content"));
    let params = ok(slide.insert_container("params", LoroMap::new()));
    let blocks = ok(params.insert_container("blocks", loro::LoroMovableList::new()));
    let element = ok(blocks.push_container(LoroMap::new()));
    ok(element.insert("id", "t"));
    ok(element.insert("kind", "text"));
    doc.commit();
    let bytes = ok(doc.export(loro::ExportMode::Snapshot));
    assert_that!(refused_kind(&bytes)).is_equal_to(ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn merge_of_a_peer_that_rewrites_the_format_is_refused() {
    let deck = roadmap();
    let snap = ok(new(&deck).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
    let (mut a, b) = (open(), open());
    ok(meta_of(&b).insert("format", 3_i64));
    b.doc.commit();
    let Err(e) = a.merge(&b) else { panic!("un pair qui réécrit le format devait être refusé") };
    assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::UnsupportedVersion);
    assert_that!(format_of_meta(&a)).is_equal_to(Some(LoroValue::I64(2)));
    assert_that!(a.deck().0).is_equal_to(&deck);
}

/// Imbrications croisées (R8j) : A met `pile-b` dans `pile-a`, B l'inverse.
/// À lamport égal, le `mov` du pair 1 gagne ; celui du pair 2 créerait un
/// cycle et reste sans effet. `pile-a` garde son entrée `@slot[""]` et reste
/// visible à sa place dans `blocks`.
#[test]
fn concurrent_crossed_nesting_moves_converge_without_cycle() {
    let pile = |id: &str| json!({ "id": id, "kind": "stack" });
    let deck = with_ops(
        &roadmap(),
        vec![add(None, "blocks", None, pile("pile-a")), add(None, "blocks", None, pile("pile-b"))],
    );
    assert_that!(ids_in(&deck, "s-constat", None, "blocks")).is_equal_to(vec![
        "b-points".to_owned(),
        "pile-a".into(),
        "pile-b".into(),
    ]);
    let snap = ok(new(&deck).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
    let (mut a, mut b) = (open(), open());
    ok(a.doc.set_peer_id(1));
    ok(b.doc.set_peer_id(2));
    let nest = |id: &str, owner: &str| {
        let Some(moved) = deck.slide("s-constat").and_then(|s| s.locate(id)).map(|l| l.element.clone()) else {
            panic!("{id} absent")
        };
        with_ops(&deck, vec![remove(id), add(Some(owner), "children", None, ok(serde_json::to_value(moved)))])
    };
    apply(&mut a, &nest("pile-b", "pile-a"));
    apply(&mut b, &nest("pile-a", "pile-b"));
    exchange(&mut a, &mut b);
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&merged);
    assert_that!(ids_in(&merged, "s-constat", None, "blocks"))
        .is_equal_to(vec!["b-points".to_owned(), "pile-a".into()]);
    assert_that!(ids_in(&merged, "s-constat", Some("pile-a"), "children")).is_equal_to(vec!["pile-b".to_owned()]);
    assert_that!(ids_in(&merged, "s-constat", Some("pile-b"), "children")).is_empty();
    assert_that!(crate::validate(&merged, &crate::test_registry::registry(), None)).is_empty();
    for r in [&mut a, &mut b] {
        let before = ops_of(r);
        let read = r.deck().0;
        apply(r, &read);
        assert_that!(ops_of(r)).is_equal_to(before);
    }
}
