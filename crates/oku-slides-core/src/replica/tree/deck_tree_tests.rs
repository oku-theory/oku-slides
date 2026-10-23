//! L'arbre du deck : slides en racine, appariement, entrée, coût.

use crate::kinds::essentiel::CONTENT;
use loro::{Container, LoroMap, LoroMovableList, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::super::super::schema::{ORPHANS, SLOT, mark_key};
use super::super::super::{DeckReplica, ROOT};
use super::super::tree_tests::*;
use super::super::{name, node_of};
use super::*;
use crate::error::ReplicaErrorKind;
use crate::{Deck, MAX_SLOT_DEPTH, Moved, Op, Slide, apply_all_moving};

/// Ajoute `element` en `index` du slot `param` de `owner` (`None` : la slide) de `slide`.
fn add_to(slide: &str, owner: Option<&str>, param: &str, index: Option<usize>, element: Value) -> Op {
    let owner = match owner {
        Some(e) => json!({ "slide": slide, "element": e }),
        None => json!({ "slide": slide }),
    };
    op(json!({ "op": "add_element", "slot": { "owner": owner, "param": param }, "index": index, "element": element }))
}

fn remove_from(slide: &str, id: &str) -> Op {
    op(json!({ "op": "remove_element", "slide": slide, "element": id }))
}

/// L'élément `id` de la slide `slide`, en JSON.
fn element_of(deck: &Deck, slide: &str, id: &str) -> Value {
    let s = deck.slide(slide).unwrap_or_else(|| panic!("{slide} absente"));
    ok(serde_json::to_value(s.locate(id).unwrap_or_else(|| panic!("{id} absent de {slide}")).element))
}

/// `deck` où l'élément `id` passe de `from` en tête de `blocks` de `to`.
fn moved_to(deck: &Deck, id: &str, from: &str, to: &str) -> Deck {
    let e = element_of(deck, from, id);
    with_ops(deck, vec![remove_from(from, id), add_to(to, None, "blocks", Some(0), e)])
}

fn slide_list(deck: &Deck) -> Vec<String> {
    deck.slides.iter().map(|s| s.id.clone()).collect()
}

/// `deck` sans la slide `id`.
fn without_slide(deck: &Deck, id: &str) -> Deck {
    let mut d = deck.clone();
    d.slides.retain(|s| s.id != id);
    d
}

/// Vrai si l'arbre du deck de `r` relu brut a une racine vivante à méta vide.
fn has_empty_root(r: &DeckReplica) -> bool {
    let tree = deck_tree(r);
    tree.children(TreeParentId::Root)
        .unwrap_or_default()
        .into_iter()
        .filter(|n| !ok(tree.is_node_deleted(n)))
        .any(|n| ok(tree.get_meta(n)).keys().next().is_none())
}

/// Le `len_ops` de `r` est-il inchangé par `apply(deck())` ?
fn assert_reapply_writes_nothing(r: &mut DeckReplica, case: &str) {
    let before = ops_of(r);
    let read = r.deck().0;
    apply(r, &read);
    assert_that!(ops_of(r)).named(case).is_equal_to(before);
}

/// Deux slides `a` et `b` de contenu, chacune avec un texte `t` de corps différent.
fn two_slides_with_t() -> Deck {
    deck_of(json!({ "slides": [
        { "id": "a", "kind": "essentiel/content", "params": { "blocks": [text("t", "Dans a")] } },
        { "id": "b", "kind": "essentiel/content", "params": { "blocks": [text("t", "Dans b")] } }
    ] }))
}

#[test]
fn slides_are_root_nodes_of_the_deck_tree() {
    let r = new(&roadmap());
    let tree = deck_tree(&r);
    let roots = tree.children(TreeParentId::Root).unwrap_or_default();
    assert_that!(roots.len()).is_equal_to(8);
    for n in &roots {
        let slide = ok(tree.get_meta(*n)).get("@slide");
        assert_that!(matches!(slide, Some(ValueOrContainer::Value(LoroValue::Bool(true))))).is_true();
    }
    let constat = slide_node(&r, "s-constat");
    let (_, points) = node_of(&r.doc, "s-constat", "b-points");
    assert_that!(tree.parent(points)).is_equal_to(Some(TreeParentId::Node(constat)));
    let mut want = serde_json::Map::new();
    want.insert(constat.to_string(), json!("blocks"));
    assert_that!(slots(&tree, points)).is_equal_to(Value::Object(want));
}

#[test]
fn nested_slot_keys_are_tree_ids() {
    let deck = with_ops(&with_pile(), vec![add(Some("pile"), "children", None, text("k", "K"))]);
    let r = new(&deck);
    let (tree, k) = node_of(&r.doc, "s-constat", "k");
    let (_, pile) = node_of(&r.doc, "s-constat", "pile");
    let mut want = serde_json::Map::new();
    want.insert(pile.to_string(), json!("children"));
    assert_that!(slots(&tree, k)).is_equal_to(Value::Object(want));
}

#[test]
fn round_trip_of_the_roadmap_is_identical() {
    let deck = roadmap();
    let r = new(&deck);
    assert_that!(slide_ids(&r)).is_equal_to(slide_list(&deck));
    assert_that!(reread(&r)).is_equal_to((deck, Vec::new()));
}

#[test]
fn apply_move_between_slides_keeps_the_tree_node() {
    let deck = roadmap();
    let mut r = new(&deck);
    let (_, before) = node_of(&r.doc, "s-constat", "b-points");
    let (constat, prio) = (slide_node(&r, "s-constat"), slide_node(&r, "s-priorites"));
    let mut both = serde_json::Map::new();
    both.insert(constat.to_string(), json!("blocks"));
    both.insert(prio.to_string(), json!("blocks"));
    let target = moved_to(&deck, "b-points", "s-constat", "s-priorites");
    apply(&mut r, &target);
    let (tree, after) = node_of(&r.doc, "s-priorites", "b-points");
    assert_that!(after).is_equal_to(before);
    assert_that!(tree.parent(after)).is_equal_to(Some(TreeParentId::Node(prio)));
    assert_that!(slots(&tree, after)).is_equal_to(Value::Object(both.clone()));
    assert_that!(r.deck()).is_equal_to((target, Vec::new()));

    apply(&mut r, &deck);
    let (tree, back) = node_of(&r.doc, "s-constat", "b-points");
    assert_that!(back).is_equal_to(before);
    assert_that!(tree.parent(back)).is_equal_to(Some(TreeParentId::Node(constat)));
    assert_that!(slots(&tree, back)).is_equal_to(Value::Object(both));
    assert_that!(r.deck()).is_equal_to((deck, Vec::new()));
    assert_reapply_writes_nothing(&mut r, "retour");
}

#[test]
fn apply_reorder_of_slides_is_one_change_and_keeps_the_nodes() {
    let deck = roadmap();
    let mut r = new(&deck);
    let nodes: Vec<TreeID> = slide_list(&deck).iter().map(|id| slide_node(&r, id)).collect();
    let mut reversed = deck.clone();
    reversed.slides.reverse();
    let changes = r.len_changes();
    apply(&mut r, &reversed);
    assert_that!(r.len_changes()).is_equal_to(changes + 1);
    for (id, node) in slide_list(&deck).iter().zip(&nodes) {
        assert_that!(slide_node(&r, id)).named(id).is_equal_to(*node);
    }
    assert_that!(slide_ids(&r)).is_equal_to(slide_list(&reversed));
    assert_that!(r.deck()).is_equal_to((reversed, Vec::new()));
}

/// Garde : une plus longue sous-suite ordonnée des slides ne bouge pas.
#[test]
fn moving_the_first_slide_to_the_end_is_one_move() {
    let deck = roadmap();
    let mut r = new(&deck);
    let mut target = deck.clone();
    let first = target.slides.remove(0);
    target.slides.push(first);
    let before = ops_of(&r);
    apply(&mut r, &target);
    assert_that!(ops_of(&r) - before).is_equal_to(1);
    assert_that!(slide_ids(&r)).is_equal_to(slide_list(&target));
}

#[test]
fn removing_a_slide_marks_only_its_node() {
    let deck = roadmap();
    let mut r = new(&deck);
    let constat = slide_node(&r, "s-constat");
    let (tree, points) = node_of(&r.doc, "s-constat", "b-points");
    let before = ops_of(&r);
    apply(&mut r, &without_slide(&deck, "s-constat"));
    assert_that!(ops_of(&r) - before).is_equal_to(1);
    assert_that!(ok(tree.is_node_deleted(&constat))).is_false();
    assert_that!(marks(&tree, constat)).is_equal_to(vec![mark_key(r.peer_id())]);
    assert_that!(marks(&tree, points)).is_empty();
    assert_that!(r.deck()).is_equal_to((without_slide(&deck, "s-constat"), Vec::new()));
}

#[test]
fn undo_of_a_slide_removal_unmarks_the_same_node() {
    let deck = roadmap();
    let mut r = new(&deck);
    let constat = slide_node(&r, "s-constat");
    let (tree, points) = node_of(&r.doc, "s-constat", "b-points");
    apply(&mut r, &without_slide(&deck, "s-constat"));
    assert_that!(ok(r.undo())).is_true();
    assert_that!(slide_node(&r, "s-constat")).is_equal_to(constat);
    assert_that!(node_of(&r.doc, "s-constat", "b-points").1).is_equal_to(points);
    assert_that!(marks(&tree, constat)).is_empty();
    assert_that!(r.deck()).is_equal_to((deck, Vec::new()));
}

#[test]
fn reapply_after_a_move_between_slides_writes_nothing() {
    let deck = roadmap();
    let mut r = new(&deck);
    let target = moved_to(&deck, "b-points", "s-constat", "s-priorites");
    apply(&mut r, &target);
    assert_that!(r.deck().0).is_equal_to(&target);
    assert_reapply_writes_nothing(&mut r, "après le déplacement");
}

/// Passe 1 : les clés sont propres à chaque slide.
#[test]
fn same_element_id_in_two_slides_keeps_each_node() {
    let deck = two_slides_with_t();
    let mut r = new(&deck);
    let (ta, tb) = (node_of(&r.doc, "a", "t").1, node_of(&r.doc, "b", "t").1);
    let mut swapped = deck.clone();
    swapped.slides.reverse();
    apply(&mut r, &swapped);
    assert_that!(node_of(&r.doc, "a", "t").1).is_equal_to(ta);
    assert_that!(node_of(&r.doc, "b", "t").1).is_equal_to(tb);
    let read = r.deck().0;
    let body = |slide: &str| {
        first_text(
            read.slide(slide)
                .and_then(|s| s.locate("t"))
                .map(|l| l.element)
                .unwrap_or_else(|| panic!("t absent de {slide}")),
        )
    };
    assert_that!(body("a")).is_equal_to("Dans a".to_owned());
    assert_that!(body("b")).is_equal_to("Dans b".to_owned());
    assert_that!(read).is_equal_to(&swapped);
}

/// R7 de la revue : un id présent dans deux slides ne passe pas d'une slide à l'autre.
#[test]
fn element_ids_repeated_across_slides_are_not_paired_between_slides() {
    let mut deck = two_slides_with_t();
    deck.slides.push(Slide::new("c", CONTENT));
    let deck = with_ops(
        &deck,
        vec![
            add_to("a", None, "blocks", None, text("body", "A")),
            add_to("b", None, "blocks", None, text("body", "B")),
        ],
    );
    let mut r = new(&deck);
    let (tree, old) = node_of(&r.doc, "a", "body");
    let (_, kept) = node_of(&r.doc, "b", "body");
    let target =
        with_ops(&deck, vec![remove_from("a", "body"), add_to("c", None, "blocks", None, text("body", "Neuf"))]);
    apply(&mut r, &target);
    let (_, fresh) = node_of(&r.doc, "c", "body");
    assert_that!(fresh).is_not_equal_to(old);
    assert_that!(marks(&tree, old)).is_equal_to(vec![mark_key(r.peer_id())]);
    assert_that!(node_of(&r.doc, "b", "body").1).is_equal_to(kept);
    assert_that!(r.deck()).is_equal_to((target, Vec::new()));
}

#[test]
fn element_moved_with_another_kind_is_not_paired_between_slides() {
    let deck = deck_of(json!({ "slides": [
        { "id": "a", "kind": "essentiel/content", "params": { "blocks": [text("x", "X")] } },
        { "id": "b", "kind": "essentiel/content" }
    ] }));
    let mut r = new(&deck);
    let (tree, old) = node_of(&r.doc, "a", "x");
    let target = with_ops(
        &deck,
        vec![remove_from("a", "x"), add_to("b", None, "blocks", None, json!({ "id": "x", "kind": "base/stack" }))],
    );
    apply(&mut r, &target);
    assert_that!(node_of(&r.doc, "b", "x").1).is_not_equal_to(old);
    assert_that!(marks(&tree, old)).is_equal_to(vec![mark_key(r.peer_id())]);
    assert_that!(r.deck()).is_equal_to((target, Vec::new()));
}

/// Trois slides `a`, `b`, `c` avec chacune un texte `title`, et une slide `d` vide.
fn title_in_three_slides() -> Deck {
    deck_of(json!({ "slides": [
        { "id": "a", "kind": "essentiel/content", "params": { "blocks": [text("title", "A")] } },
        { "id": "b", "kind": "essentiel/content", "params": { "blocks": [text("title", "B")] } },
        { "id": "c", "kind": "essentiel/content", "params": { "blocks": [text("title", "C")] } },
        { "id": "d", "kind": "essentiel/content" }
    ] }))
}

/// `deck` où `move_element` fait passer `title` de `a` à `blocks` de `d`, et ses déplacements.
fn title_moved_to_d(deck: &Deck) -> (Deck, Vec<Moved>) {
    let mut target = deck.clone();
    let mv = op(json!({ "op": "move_element", "slide": "a", "element": "title",
        "to": { "owner": { "slide": "d" }, "param": "blocks" } }));
    let moves = ok(apply_all_moving(&mut target, vec![mv], &registry()));
    (target, moves)
}

/// R5, point 3 : un déplacement annoncé apparie le nœud, même si son id est répété.
#[test]
fn announced_move_keeps_the_node_even_with_a_repeated_id() {
    let deck = title_in_three_slides();
    let (target, moves) = title_moved_to_d(&deck);
    assert_that!(moves).is_equal_to(vec![Moved { element: "title".into(), from: "a".into(), to: "d".into() }]);

    let mut r = new(&deck);
    let (tree, old) = node_of(&r.doc, "a", "title");
    ok(r.apply_moving(&target, "test", None, &moves));
    assert_that!(node_of(&r.doc, "d", "title").1).is_equal_to(old);
    assert_that!(marks(&tree, old)).is_empty();
    assert_that!(tree.parent(old)).is_equal_to(Some(TreeParentId::Node(slide_node(&r, "d"))));
    assert_that!(r.deck()).is_equal_to((target.clone(), Vec::new()));
    assert_reapply_writes_nothing(&mut r, "après le déplacement annoncé");

    // Contre-épreuve (R7) : sans `moves`, l'id répété crée un nœud neuf.
    let mut plain = new(&deck);
    let (tree, old) = node_of(&plain.doc, "a", "title");
    apply(&mut plain, &target);
    assert_that!(node_of(&plain.doc, "d", "title").1).is_not_equal_to(old);
    assert_that!(marks(&tree, old)).is_equal_to(vec![mark_key(plain.peer_id())]);
    assert_that!(plain.deck()).is_equal_to((target, Vec::new()));
}

/// Une entrée de `moves` qui ne se résout pas est ignorée : même résultat qu'`apply`.
#[test]
fn stale_move_hint_is_ignored() {
    let deck = title_in_three_slides();
    let (target, _) = title_moved_to_d(&deck);
    let stale = [
        Moved { element: "title".into(), from: "absente".into(), to: "d".into() },
        Moved { element: "title".into(), from: "a".into(), to: "absente".into() },
        Moved { element: "inconnu".into(), from: "a".into(), to: "d".into() },
    ];
    let mut hinted = new(&deck);
    let (tree, old) = node_of(&hinted.doc, "a", "title");
    ok(hinted.apply_moving(&target, "test", None, &stale));
    let mut plain = new(&deck);
    apply(&mut plain, &target);
    assert_that!(node_of(&hinted.doc, "d", "title").1).is_not_equal_to(old);
    assert_that!(marks(&tree, old)).is_equal_to(vec![mark_key(hinted.peer_id())]);
    assert_that!(hinted.deck()).is_equal_to((target, Vec::new()));
    assert_that!(ops_of(&hinted)).is_equal_to(ops_of(&plain));

    // Un `kind` différent dans la slide voulue : l'entrée est ignorée aussi.
    let other_kind = with_ops(
        &deck,
        vec![
            remove_from("a", "title"),
            add_to("d", None, "blocks", None, json!({ "id": "title", "kind": "base/stack" })),
        ],
    );
    let hint = [Moved { element: "title".into(), from: "a".into(), to: "d".into() }];
    let mut r = new(&deck);
    let (tree, old) = node_of(&r.doc, "a", "title");
    ok(r.apply_moving(&other_kind, "test", None, &hint));
    assert_that!(node_of(&r.doc, "d", "title").1).is_not_equal_to(old);
    assert_that!(marks(&tree, old)).is_equal_to(vec![mark_key(r.peer_id())]);
    assert_that!(r.deck()).is_equal_to((other_kind, Vec::new()));
}

/// R5, point 5 : pas d'appariement depuis une slide retirée.
#[test]
fn element_of_a_removed_slide_is_created_anew_elsewhere() {
    let deck = roadmap();
    let mut r = new(&deck);
    let constat = slide_node(&r, "s-constat");
    let (tree, old) = node_of(&r.doc, "s-constat", "b-points");
    let points = element_of(&deck, "s-constat", "b-points");
    let target =
        with_ops(&without_slide(&deck, "s-constat"), vec![add_to("s-priorites", None, "blocks", Some(0), points)]);
    apply(&mut r, &target);
    assert_that!(node_of(&r.doc, "s-priorites", "b-points").1).is_not_equal_to(old);
    assert_that!(tree.parent(old)).is_equal_to(Some(TreeParentId::Node(constat)));
    assert_that!(marks(&tree, constat)).is_equal_to(vec![mark_key(r.peer_id())]);
    assert_that!(r.deck()).is_equal_to((target, Vec::new()));
}

/// R5, point 4 : les descendants d'une racine déplacée sont appariés dans son sous-arbre.
#[test]
fn moved_owner_keeps_its_children_nodes() {
    let deck = with_ops(
        &with_pile(),
        vec![
            add(Some("pile"), "children", None, text("k1", "Un")),
            add(Some("pile"), "children", None, text("k2", "Deux")),
        ],
    );
    let mut r = new(&deck);
    let ids = ["pile", "k1", "k2"];
    let before: Vec<TreeID> = ids.iter().map(|id| node_of(&r.doc, "s-constat", id).1).collect();
    let target = moved_to(&deck, "pile", "s-constat", "s-priorites");
    apply(&mut r, &target);
    let after: Vec<TreeID> = ids.iter().map(|id| node_of(&r.doc, "s-priorites", id).1).collect();
    assert_that!(after).is_equal_to(before);
    assert_that!(r.deck()).is_equal_to((target, Vec::new()));
    assert_reapply_writes_nothing(&mut r, "après le déplacement");
}

/// Forge `x` (texte) sous `s-title`, de `@slot` `slot`, et vérifie qu'il y est.
fn forge_x_on_title(r: &DeckReplica, slot: &[(&str, &str)]) {
    let x = forge(r, "s-title", None, &text_meta("x"), Some(slot));
    assert_that!(deck_tree(r).parent(x)).is_equal_to(Some(TreeParentId::Node(slide_node(r, "s-title"))));
}

#[test]
fn read_rule_three_ignores_keys_that_are_not_canonical_tree_ids() {
    let r = new(&roadmap());
    forge_x_on_title(&r, &[("12", "zzz"), ("+7@1", "zzz"), ("7@1", "elements")]);
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, "elements")).is_equal_to(vec!["x".to_owned()]);
    assert_that!(issues).is_empty();

    let r = new(&roadmap());
    forge_x_on_title(&r, &[("x@1", "blocks")]);
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, ORPHANS)).is_equal_to(vec!["x".to_owned()]);
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-slot-repaired"]);
}

/// Règle 3b : la seule entrée désigne un nœud vivant qui n'est pas le parent.
#[test]
fn read_rule_three_with_a_live_other_parent_reports_the_repair() {
    let r = new(&with_pile());
    let pile = node_of(&r.doc, "s-constat", "pile").1.to_string();
    forge_x_on_title(&r, &[(pile.as_str(), "children")]);
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, "children")).is_equal_to(vec!["x".to_owned()]);
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-slot-repaired"]);
}

/// Règle 3b : deux nœuds disparus, deux slots ; la première clé l'emporte.
#[test]
fn read_rule_three_with_absent_parents_of_two_slots_reports_the_repair() {
    let r = new(&roadmap());
    forge_x_on_title(&r, &[("7@1", "blocks"), ("8@1", "elements")]);
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, "blocks")).is_equal_to(vec!["x".to_owned()]);
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-slot-repaired"]);
}

/// Le genre d'erreur de `from_snapshot` sur le snapshot de `r`, s'il échoue.
fn import_error(r: &DeckReplica) -> Option<ReplicaErrorKind> {
    DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry()).err().map(|e| e.kind())
}

/// Une slide déplacée par `forge_on(r)`, refusée à l'import et à la fusion.
fn assert_refused(forge_on: impl Fn(&DeckReplica), case: &str) {
    let r = new(&roadmap());
    forge_on(&r);
    assert_that!(import_error(&r)).named(case).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    let (mut a, b) = fixed_peers(&roadmap(), 1, 2);
    forge_on(&b);
    let before = a.deck();
    assert_that!(a.merge(&b).err().map(|e| e.kind())).named(case).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    assert_that!(a.deck()).named(case).is_equal_to(before);
}

#[test]
fn slide_outside_the_root_is_refused() {
    let under = |parent: fn(&DeckReplica) -> TreeID| {
        move |r: &DeckReplica| {
            ok(deck_tree(r).mov(slide_node(r, "s-title"), parent(r)));
            r.doc.commit();
        }
    };
    assert_refused(under(|r| node_of(&r.doc, "s-constat", "b-points").1), "sous un élément");
    assert_refused(under(|r| slide_node(r, "s-constat")), "sous une slide");
}

/// Une écriture forgée dans une méta.
type Forge = fn(&LoroMap);

#[test]
fn root_without_slide_mark_is_ignored_and_accepted() {
    let unmark: [(&str, Forge); 2] =
        [("sans @slide", |m| ok(m.delete("@slide"))), ("@slide en chaîne", |m| ok(m.insert("@slide", "true")))];
    let deck = roadmap();
    let expected = without_slide(&deck, "s-title");
    for (case, edit) in unmark {
        let r = new(&deck);
        let (mut a, mut b) = fixed_peers(&deck, 1, 2);
        for forged in [&r, &b] {
            edit(&ok(deck_tree(forged).get_meta(slide_node(forged, "s-title"))));
            forged.doc.commit();
        }
        let (read, issues) = reread(&r);
        assert_that!(read).named(case).is_equal_to(&expected);
        assert_that!(issue_ids(&issues)).named(case).is_equal_to(vec!["issue-unreadable-slide"]);
        assert_that!(issues[0].slide.as_deref()).named(case).is_equal_to(Some("s-title"));
        ok(a.merge(&b));
        assert_that!(a.deck().0).named(case).is_equal_to(&expected);
        let before = ok(deck_tree(&a).get_meta(slide_node(&a, "s-title"))).get_value();
        assert_reapply_writes_nothing(&mut a, case);
        assert_that!(ok(deck_tree(&a).get_meta(slide_node(&a, "s-title"))).get_value()).is_equal_to(before);
        exchange(&mut a, &mut b);
    }

    let r = new(&deck);
    ok(deck_tree(&r).create(TreeParentId::Root));
    r.doc.commit();
    assert_that!(has_empty_root(&r)).is_true();
    let mut opened = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry()));
    assert_that!(opened.1.clone()).is_equal_to((deck.clone(), Vec::new()));
    assert_reapply_writes_nothing(&mut opened.0, "racine vide");
    assert_that!(has_empty_root(&opened.0)).is_true();
}

/// t-776 : défaire le déplacement d'une slide dont le pair a défait l'ajout
/// ne crée plus de racine à méta vide : la slide, marquée par le pair, est replacée.
#[test]
fn undo_of_a_slide_move_after_the_peer_undid_its_creation_leaves_no_empty_root() {
    let deck = roadmap();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    let mut added = deck.clone();
    added.slides.push(Slide::new("s-neuve", CONTENT));
    apply(&mut b, &added);
    exchange(&mut a, &mut b);
    let mut moved = added.clone();
    let neuve = moved.slides.remove(moved.slides.len() - 1);
    moved.slides.insert(0, neuve);
    apply(&mut a, &moved);
    assert_that!(ok(b.undo())).is_true();
    exchange(&mut a, &mut b);
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    assert_that!(has_empty_root(&a)).is_false();
    for r in [&a, &b] {
        assert_that!(reread(r)).is_equal_to((deck.clone(), Vec::new()));
        assert_that!(r.deck()).is_equal_to((deck.clone(), Vec::new()));
    }
    exchange(&mut a, &mut b);
    for (r, case) in [(&mut a, "a"), (&mut b, "b")] {
        assert_reapply_writes_nothing(r, case);
    }
}

#[test]
fn slides_of_another_type_are_refused() {
    let forge_list = |r: &DeckReplica| {
        ok(r.doc.get_map(ROOT).insert_container("slides", LoroMovableList::new()));
        r.doc.commit();
    };
    let r = new(&roadmap());
    forge_list(&r);
    assert_that!(import_error(&r)).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    let (mut a, b) = fixed_peers(&roadmap(), 1, 2);
    forge_list(&b);
    let before = a.deck();
    assert_that!(a.merge(&b).err().map(|e| e.kind())).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    assert_that!(a.deck()).is_equal_to(before);

    let mut r = r;
    let Err(e) = r.apply(&roadmap(), "test", None) else {
        panic!("des slides d'un autre type devaient être refusées")
    };
    assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
    let slides = r.doc.get_map(ROOT).get("slides");
    assert_that!(matches!(slides, Some(ValueOrContainer::Container(Container::MovableList(_))))).is_true();
}

/// Une chaîne de `n` `base/stack` imbriqués, d'ids `c0` à `c{n-1}`.
fn stack_chain(n: usize) -> Value {
    let mut e = json!({ "id": format!("c{}", n - 1), "kind": "base/stack" });
    for i in (0..n - 1).rev() {
        e = json!({ "id": format!("c{i}"), "kind": "base/stack", "params": { "children": [e] } });
    }
    e
}

/// Temps 0 : un deck trop profond n'écrit rien, même ailleurs que dans la slide fautive.
#[test]
fn too_deep_slide_leaves_the_whole_deck_untouched() {
    let deck = roadmap();
    let mut r = new(&deck);
    let mut target = without_slide(&deck, "s-fin");
    target.slides.swap(0, 1);
    let Some(title) = target.slide_mut("s-title") else { panic!("s-title absente") };
    title.notes = "Renommée".into();
    let mut json = ok(serde_json::to_value(&target));
    let Some(slides) = json["slides"].as_array_mut() else { panic!("slides attendues") };
    let Some(constat) = slides.iter_mut().find(|s| s["id"] == "s-constat") else { panic!("s-constat absente") };
    constat["params"]["elements"] = json!([stack_chain(MAX_SLOT_DEPTH + 1)]);
    let target = deck_of(json);
    let before = ops_of(&r);
    let Err(e) = r.apply(&target, "trop", None) else { panic!("un deck trop profond devait être refusé") };
    assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::Write);
    assert_that!(ops_of(&r)).is_equal_to(before);
    assert_that!(r.deck()).is_equal_to((deck, Vec::new()));
}

#[test]
fn pair_has_one_entry_per_wanted_slide() {
    let deck = roadmap();
    let r = new(&deck);
    let mut target = without_slide(&without_slide(&deck, "s-ch1"), "s-fin");
    target.slides.insert(0, Slide::new("s-a", CONTENT));
    target.slides.push(Slide::new("s-b", CONTENT));
    let target = with_ops(
        &target,
        vec![
            add_to("s-a", None, "blocks", None, text("na", "A")),
            add_to("s-b", None, "blocks", None, text("nb", "B")),
        ],
    );
    let json = ok(serde_json::to_value(&target));
    let slides = json["slides"].as_array().map(Vec::as_slice).unwrap_or_default();
    let tree = deck_tree(&r);
    let wanted = ok(wanted_slides(&registry(), slides));
    assert_that!(wanted.len()).is_equal_to(target.slides.len());
    let pairing = pair(&live_slides(&tree), &wanted, &[]);
    assert_that!(pairing.slides.len()).is_equal_to(wanted.len());
    assert_that!(pairing.elements.len()).is_equal_to(wanted.len());
    for (i, w) in wanted.iter().enumerate() {
        let id = w.obj.get("id").and_then(Value::as_str).unwrap_or_default();
        assert_that!(pairing.elements[i].len()).named(id).is_equal_to(w.elements.len());
        let fresh = id == "s-a" || id == "s-b";
        assert_that!(pairing.slides[i].is_none()).named(id).is_equal_to(fresh);
    }
}

/// Imbrications croisées d'une slide à l'autre : à lamport égal, le `mov` du
/// pair 1 gagne ; l'autre créerait un cycle et reste sans effet. Le perdant
/// garde son entrée `@slot` et reste visible dans `blocks` de sa slide.
#[test]
fn concurrent_crossed_nesting_across_slides_converges_without_cycle() {
    let pile = |id: &str| json!({ "id": id, "kind": "base/stack" });
    let deck = with_ops(
        &roadmap(),
        vec![
            add_to("s-constat", None, "blocks", None, pile("pile-a")),
            add_to("s-priorites", None, "blocks", None, pile("pile-b")),
        ],
    );
    let nest = |id: &str, from: &str, owner: &str, to: &str| {
        let e = element_of(&deck, from, id);
        with_ops(&deck, vec![remove_from(from, id), add_to(to, Some(owner), "children", None, e)])
    };
    let a_nests = nest("pile-b", "s-priorites", "pile-a", "s-constat");
    let b_nests = nest("pile-a", "s-constat", "pile-b", "s-priorites");
    for (pa, pb) in [(1, 2), (2, 1)] {
        let case = format!("pairs ({pa}, {pb})");
        let (mut a, mut b) = fixed_peers(&deck, pa, pb);
        apply(&mut a, &a_nests);
        apply(&mut b, &b_nests);
        exchange(&mut a, &mut b);
        let (merged, issues) = a.deck();
        assert_that!(issues).named(&case).is_empty();
        assert_that!(b.deck()).named(&case).is_equal_to((merged.clone(), Vec::new()));
        let winner = if pa == 1 { &a_nests } else { &b_nests };
        assert_that!(&merged).named(&case).is_equal_to(winner);
        for (r, who) in [(&mut a, "a"), (&mut b, "b")] {
            assert_reapply_writes_nothing(r, &format!("{case}, {who}"));
        }
    }
}

/// Même croisement par l'op `move_element` (t-777) : à lamport égal, le
/// déplacement du pair 1 gagne, l'autre reste sans effet.
#[test]
fn concurrent_crossed_move_element_nesting_converges_without_cycle() {
    let pile = |id: &str| json!({ "id": id, "kind": "base/stack" });
    let deck = with_ops(
        &roadmap(),
        vec![
            add_to("s-constat", None, "blocks", None, pile("pile-a")),
            add_to("s-priorites", None, "blocks", None, pile("pile-b")),
        ],
    );
    let nest = |id: &str, from: &str, owner: &str, to: &str| {
        let mv = op(json!({ "op": "move_element", "slide": from, "element": id,
            "to": { "owner": { "slide": to, "element": owner }, "param": "children" } }));
        let mut target = deck.clone();
        let moves = ok(apply_all_moving(&mut target, vec![mv], &registry()));
        (target, moves)
    };
    let a_nests = nest("pile-b", "s-priorites", "pile-a", "s-constat");
    let b_nests = nest("pile-a", "s-constat", "pile-b", "s-priorites");
    for (pa, pb) in [(1, 2), (2, 1)] {
        let case = format!("pairs ({pa}, {pb})");
        let (mut a, mut b) = fixed_peers(&deck, pa, pb);
        ok(a.apply_moving(&a_nests.0, "test", None, &a_nests.1));
        ok(b.apply_moving(&b_nests.0, "test", None, &b_nests.1));
        exchange(&mut a, &mut b);
        let (merged, issues) = a.deck();
        assert_that!(issues).named(&case).is_empty();
        assert_that!(b.deck()).named(&case).is_equal_to((merged.clone(), Vec::new()));
        let winner = if pa == 1 { &a_nests.0 } else { &b_nests.0 };
        assert_that!(&merged).named(&case).is_equal_to(winner);
        for (r, who) in [(&mut a, "a"), (&mut b, "b")] {
            assert_reapply_writes_nothing(r, &format!("{case}, {who}"));
        }
    }
}

/// Coût des nœuds marqués (R9), mesuré en métas lues : `deck()` en lit au
/// plus deux par nœud, `apply` au plus quatre.
fn assert_linear_cost(r: &mut DeckReplica, case: &str) {
    let nodes = deck_tree(r).nodes().len();
    reset_visited();
    let read = r.deck().0;
    assert_that!(visited()).named(case).is_less_than_or_equal_to(2 * nodes);
    let before = ops_of(r);
    reset_visited();
    apply(r, &read);
    assert_that!(visited()).named(case).is_less_than_or_equal_to(4 * nodes);
    assert_that!(ops_of(r)).named(case).is_equal_to(before);
}

#[test]
fn marked_slides_cost_stays_linear() {
    let mut many = Deck::empty();
    many.slides = (0..10_001).map(|i| Slide::new(format!("s{i}"), CONTENT)).collect();
    let mut r = new(&many);
    let mut one = many.clone();
    one.slides.truncate(1);
    apply(&mut r, &one);
    assert_that!(deck_tree(&r).nodes().len()).is_equal_to(10_001);
    assert_linear_cost(&mut r, "10 000 slides marquées");

    let blocks: Vec<Value> = (0..10_000).map(|i| text(&format!("t{i}"), "T")).collect();
    let full =
        deck_of(json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": { "blocks": blocks } }] }));
    let mut r = new(&full);
    apply(&mut r, &deck_of(json!({ "slides": [{ "id": "s", "kind": "essentiel/content" }] })));
    assert_that!(deck_tree(&r).nodes().len()).is_equal_to(10_001);
    assert_linear_cost(&mut r, "10 000 éléments marqués");
}

/// Garde de coût sur des éléments vivants : relire puis réécrire une slide de
/// 2 000 éléments, puis y ajouter un élément, visite O(n) nœuds, pas O(n²).
#[test]
fn live_elements_cost_stays_linear() {
    let blocks: Vec<Value> = (0..2_000).map(|i| text(&format!("t{i}"), "T")).collect();
    let full =
        deck_of(json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": { "blocks": blocks } }] }));
    let mut r = new(&full);
    assert_linear_cost(&mut r, "2 000 éléments vivants");
    let nodes = deck_tree(&r).nodes().len();
    let added = with_ops(&full, vec![add_to("s", None, "blocks", Some(0), text("neuf", "N"))]);
    reset_visited();
    apply(&mut r, &added);
    assert_that!(visited()).is_less_than_or_equal_to(4 * nodes);
    assert_that!(r.deck()).is_equal_to((added, Vec::new()));
}

/// Forge une chaîne de `n` `stack` (`d0` à `d{n-1}`) sous `s-constat`, sans réconciliation.
fn forge_deep_chain(r: &DeckReplica, n: usize) {
    let tree = deck_tree(r);
    let mut parent = slide_node(r, "s-constat");
    for i in 0..n {
        let node = ok(tree.create(TreeParentId::Node(parent)));
        let meta = ok(tree.get_meta(node));
        ok(meta.insert("id", format!("d{i}")));
        ok(meta.insert("kind", "base/stack"));
        let slot = if i == 0 { "blocks" } else { "children" };
        ok(ok(meta.ensure_mergeable_map(SLOT)).insert(&parent.to_string(), slot));
        parent = node;
    }
    r.doc.commit();
}

/// Exécute `f` sur un fil à pile de `size` octets.
fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement de pile"))
}

/// Un pair forgé en mémoire (jamais ouvert) : la fusion est refusée sur une petite pile, `self` intact (t-809).
#[test]
fn merge_of_a_forged_peer_too_deep_or_with_a_foreign_tree_is_refused() {
    for foreign in [false, true] {
        let mine = new(&roadmap());
        let snap = ok(mine.snapshot());
        let other = on_stack(1 << 30, move || {
            let other = ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
            // Une chaîne nue : `forge_deep_chain` (méta, `@slot`) coûte des
            // minutes à 20 000 niveaux, et la borne ne lit pas la méta.
            let (tree, mut at) = if foreign {
                (other.doc.get_tree("autre"), TreeParentId::Root)
            } else {
                (deck_tree(&other), TreeParentId::Node(slide_node(&other, "s-constat")))
            };
            for _ in 0..20_000 {
                at = TreeParentId::Node(ok(tree.create(at)));
            }
            other.doc.commit();
            other
        });
        let before = mine.deck().0;
        let (mine, other, kind) = on_stack(256 * 1024, move || {
            let mut mine = mine;
            let kind = mine.merge(&other).err().map(|e| e.kind());
            (mine, other, kind)
        });
        assert_that!(kind).named(&format!("arbre étranger : {foreign}")).is_equal_to(Some(ReplicaErrorKind::Corrupted));
        assert_that!(mine.deck().0).is_equal_to(before);
        // Le pair forgé se libère sur une grande pile (son état n'est pas rejoué).
        on_stack(1 << 30, move || drop(other));
    }
}

/// Les ids des nœuds de `tree` ni supprimés ni marqués, lus bruts.
fn raw_live_ids(tree: &LoroTree) -> Vec<String> {
    tree.nodes()
        .into_iter()
        .filter(|n| !ok(tree.is_node_deleted(n)) && marks(tree, *n).is_empty())
        .filter_map(|n| name(ok(tree.get_meta(n)).get("id")))
        .collect()
}

/// Une chaîne forgée bien plus profonde que `MAX_SLOT_DEPTH` : contrôle
/// d'entrée, lecture et écriture itératifs, nœuds au-delà de la borne intacts.
///
/// 300 niveaux, sous `MAX_TREE_DEPTH` : Loro 1.16.2 décode l'état d'un arbre
/// récursivement au premier accès, et le premier accès se fait sur un document
/// rejoué (t-809).
#[test]
fn deep_element_chain_in_the_deck_tree_reads_and_applies_in_a_small_stack() {
    const N: usize = 300;
    let r = new(&roadmap());
    forge_deep_chain(&r, N);
    let bytes = ok(r.snapshot());
    let r = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let (mut r, _) = ok(DeckReplica::from_snapshot(&bytes, 1 << 26, registry()));
            let read = r.deck().0;
            ok(r.apply(&read, "test", None));
            r
        })
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("la lecture ou l'écriture a paniqué ou débordé la pile"));
    let live = raw_live_ids(&deck_tree(&r));
    for i in [0, MAX_SLOT_DEPTH, N - 1] {
        assert_that!(live.contains(&format!("d{i}"))).named(&format!("d{i}")).is_true();
    }
    assert_that!(live.iter().filter(|id| id.starts_with('d')).count()).is_equal_to(N);
}

/// Un snapshot à la forme du format 2 : slides en `LoroMovableList`, arbre
/// `@elements` par slide, `meta.format = 2`.
fn v8_shaped_snapshot() -> Vec<u8> {
    let doc = loro::LoroDoc::new();
    ok(meta_of_doc(&doc).insert("deck_uid", "uid-v8"));
    ok(meta_of_doc(&doc).insert("format", 2_i64));
    let deck = doc.get_map(ROOT);
    ok(deck.insert("version", 5_i64));
    let slides = ok(deck.insert_container("slides", LoroMovableList::new()));
    let slide = ok(slides.push_container(LoroMap::new()));
    ok(slide.insert("id", "s"));
    ok(slide.insert("kind", CONTENT));
    let tree = ok(slide.insert_container("@elements", LoroTree::new()));
    let node = ok(tree.create(TreeParentId::Root));
    ok(ok(tree.get_meta(node)).insert("id", "t"));
    doc.commit();
    ok(doc.export(loro::ExportMode::Snapshot))
}

fn meta_of_doc(doc: &loro::LoroDoc) -> LoroMap {
    doc.get_map("meta")
}

#[test]
fn from_snapshot_of_a_format_2_replica_is_refused() {
    let r = new(&roadmap());
    let bytes = snapshot_with_meta(&r, |m| ok(m.insert("format", 2_i64)));
    assert_that!(refused_kind(&bytes)).is_equal_to(ReplicaErrorKind::UnsupportedVersion);
}

/// Le format se contrôle avant la forme : un replica v8 n'est pas `Corrupted`.
#[test]
fn from_snapshot_of_a_v8_shaped_replica_is_refused() {
    assert_that!(refused_kind(&v8_shaped_snapshot())).is_equal_to(ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn merge_of_a_format_2_peer_is_refused() {
    let deck = roadmap();
    let snap = ok(new(&deck).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
    let (mut a, b) = (open(), open());
    ok(meta_of(&b).insert("format", 2_i64));
    b.doc.commit();
    let Err(e) = a.merge(&b) else { panic!("un pair au format 2 devait être refusé") };
    assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::UnsupportedVersion);
    assert_that!(format_of_meta(&a)).is_equal_to(Some(LoroValue::I64(7)));
    assert_that!(a.deck().0).is_equal_to(&deck);
}
