//! Suppression douce dans l'arbre des éléments : marques, annulation, entrée.

use loro::{LoroMap, LoroText, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use serde_json::json;
use speculoos::prelude::*;

use super::super::DeckReplica;
use super::super::schema::mark_key;
use super::tree_tests::*;
use super::*;
use crate::error::ReplicaErrorKind;
use crate::{Deck, Element};

/// `with_pile` et, dans `children` de `pile`, les textes `k1` et `k2`.
fn with_pile_and_children() -> Deck {
    with_ops(
        &with_pile(),
        vec![
            add(Some("pile"), "children", None, text("k1", "Un")),
            add(Some("pile"), "children", None, text("k2", "Deux")),
        ],
    )
}

/// `deck` sans l'élément `id` de `s-constat`.
fn without(deck: &Deck, id: &str) -> Deck {
    with_ops(deck, vec![remove(id)])
}

/// L'élément `id` de `s-constat` dans `deck`, en JSON.
fn element(deck: &Deck, id: &str) -> serde_json::Value {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    ok(serde_json::to_value(slide.locate(id).unwrap_or_else(|| panic!("{id} absent")).element))
}

/// Vrai si `deck` porte un élément `id` dans `s-constat`.
fn has(deck: &Deck, id: &str) -> bool {
    deck.slide("s-constat").is_some_and(|s| s.locate(id).is_some())
}

/// L'élément `id` de `s-constat` dans `deck`.
fn located(deck: &Deck, id: &str) -> Element {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    slide.locate(id).unwrap_or_else(|| panic!("{id} absent")).element.clone()
}

/// Les enfants du nœud de `s-constat` d'id `id`, ni supprimés ni marqués,
/// dans l'ordre brut des enfants.
fn raw_live_roots(r: &DeckReplica, id: &str) -> Vec<TreeID> {
    let tree = slide_tree(r, "s-constat");
    tree.children(TreeParentId::Node(slide_node(r, "s-constat")))
        .unwrap_or_default()
        .into_iter()
        .filter(|n| !ok(tree.is_node_deleted(n)) && marks(&tree, *n).is_empty())
        .filter(|n| name(ok(tree.get_meta(*n)).get("id")).as_deref() == Some(id))
        .collect()
}

#[test]
fn removing_an_element_marks_its_node_without_deleting_it() {
    let deck = roadmap();
    let mut r = new(&deck);
    let (tree, n) = node_of(&r.doc, "s-constat", "b-points");
    apply(&mut r, &without(&deck, "b-points"));
    assert_that!(ok(tree.is_node_deleted(&n))).is_false();
    let key = mark_key(r.peer_id());
    assert_that!(marks(&tree, n)).is_equal_to(vec![key.clone()]);
    let value = ok(tree.get_meta(n)).get(&key);
    assert_that!(matches!(value, Some(ValueOrContainer::Value(LoroValue::Bool(true))))).is_true();
    assert_that!(has(&r.deck().0, "b-points")).is_false();
}

#[test]
fn undo_of_a_removal_unmarks_the_same_node() {
    let deck = roadmap();
    let mut r = new(&deck);
    let (tree, n) = node_of(&r.doc, "s-constat", "b-points");
    apply(&mut r, &without(&deck, "b-points"));
    assert_that!(ok(r.undo())).is_true();
    assert_that!(node_of(&r.doc, "s-constat", "b-points").1).is_equal_to(n);
    assert_that!(marks(&tree, n)).is_empty();
    assert_that!(r.deck().0).is_equal_to(&deck);
}

#[test]
fn redo_of_a_removal_marks_the_node_again() {
    let deck = roadmap();
    let mut r = new(&deck);
    let (tree, n) = node_of(&r.doc, "s-constat", "b-points");
    apply(&mut r, &without(&deck, "b-points"));
    assert_that!(ok(r.undo())).is_true();
    assert_that!(ok(r.redo())).is_true();
    assert_that!(ok(tree.is_node_deleted(&n))).is_false();
    assert_that!(marks(&tree, n)).is_equal_to(vec![mark_key(r.peer_id())]);
    assert_that!(has(&r.deck().0, "b-points")).is_false();
}

/// Garde : le sous-arbre d'un propriétaire retiré n'est pas visité.
#[test]
fn removing_an_owner_writes_one_mark() {
    let deck = with_pile_and_children();
    let mut r = new(&deck);
    let (tree, k1) = node_of(&r.doc, "s-constat", "k1");
    let (_, k2) = node_of(&r.doc, "s-constat", "k2");
    let before = ops_of(&r);
    apply(&mut r, &without(&deck, "pile"));
    assert_that!(ops_of(&r) - before).is_equal_to(1);
    assert_that!(marks(&tree, k1)).is_empty();
    assert_that!(marks(&tree, k2)).is_empty();
}

#[test]
fn read_hides_the_subtree_of_a_marked_owner() {
    let r = new(&with_pile_and_children());
    let (tree, pile) = node_of(&r.doc, "s-constat", "pile");
    ok(ok(tree.get_meta(pile)).insert("@deleted/9", true));
    r.doc.commit();
    let (read, issues) = reread(&r);
    for id in ["pile", "k1", "k2"] {
        assert_that!(has(&read, id)).named(id).is_false();
    }
    assert_that!(issues).is_empty();
}

/// R2, règle 2 : un frère marqué est écarté avant le calcul du slot hérité.
#[test]
fn marked_sibling_does_not_give_its_slot() {
    let r = new(&roadmap());
    let key = owner_key(&r, "s-title", None);
    let mut marked = text_meta("y");
    marked.push(("@deleted/9", LoroValue::Bool(true)));
    forge(&r, "s-title", None, &marked, Some(&[(key.as_str(), "elements")]));
    forge(&r, "s-title", None, &text_meta("x"), Some(&[]));
    forge(&r, "s-title", None, &text_meta("z"), Some(&[(key.as_str(), "blocks")]));
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, "blocks")).is_equal_to(vec!["x".to_owned(), "z".into()]);
    assert_that!(ids_in(&read, "s-title", None, "elements")).is_empty();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-slot-repaired"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("x"));
}

/// Garde (F1) : réécrire un deck relu n'écrit rien, nœuds marqués compris.
#[test]
fn reapply_with_marked_nodes_writes_nothing() {
    let deck = with_pile_and_children();
    let mut r = new(&deck);
    apply(&mut r, &without(&deck, "pile"));
    let before = ops_of(&r);
    let read = r.deck().0;
    apply(&mut r, &read);
    assert_that!(ops_of(&r)).is_equal_to(before);
}

#[test]
fn readding_a_removed_id_creates_a_new_node() {
    let deck = roadmap();
    let mut r = new(&deck);
    let (tree, old) = node_of(&r.doc, "s-constat", "b-points");
    apply(&mut r, &without(&deck, "b-points"));
    apply(&mut r, &deck);
    assert_that!(node_of(&r.doc, "s-constat", "b-points").1).is_not_equal_to(old);
    assert_that!(ok(tree.is_node_deleted(&old))).is_false();
    assert_that!(marks(&tree, old)).is_equal_to(vec![mark_key(r.peer_id())]);
    assert_that!(r.deck().0).is_equal_to(&deck);
    let before = ops_of(&r);
    apply(&mut r, &deck);
    assert_that!(ops_of(&r)).is_equal_to(before);
}

/// Un id réajouté, puis l'annulation de l'ancien retrait : deux nœuds au même
/// id, réparés à la lecture par `unique_ids` (t-652) et écrits une fois.
#[test]
fn undo_of_a_removal_after_the_id_was_readded_reads_unique_ids() {
    let deck = roadmap();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    let (_, old) = node_of(&a.doc, "s-constat", "b-points");
    let removed = without(&deck, "b-points");
    apply(&mut a, &removed);
    exchange(&mut a, &mut b);
    apply(&mut b, &with_ops(&removed, vec![add(None, "blocks", None, text("b-points", "neuf"))]));
    exchange(&mut a, &mut b);
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    let live = raw_live_roots(&a, "b-points");
    assert_that!(live.len()).is_equal_to(2);
    assert_that!(live.contains(&old)).is_true();
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&read);
    assert_that!(ids_in(&read, "s-constat", None, "blocks"))
        .is_equal_to(vec!["b-points".to_owned(), "b-points-2".into()]);
    let original = first_text(&located(&deck, "b-points"));
    let (first, second) = if live[0] == old { (original, "neuf".to_owned()) } else { ("neuf".to_owned(), original) };
    assert_that!(first_text(&located(&read, "b-points"))).is_equal_to(first);
    assert_that!(first_text(&located(&read, "b-points-2"))).is_equal_to(second);
    let before = ops_of(&a);
    apply(&mut a, &read);
    assert_that!(ops_of(&a)).is_greater_than(before);
    assert_that!(a.deck().0).is_equal_to(&read);
    let settled = ops_of(&a);
    apply(&mut a, &read);
    assert_that!(ops_of(&a)).is_equal_to(settled);
}

/// Sonde, point 3 : un élément déplacé sous un propriétaire marqué est caché
/// avec lui ; l'annulation du retrait les rend tous deux.
#[test]
fn undo_of_an_owner_removal_brings_back_a_child_moved_in_by_the_peer() {
    let deck = with_pile_and_children();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    apply(&mut a, &without(&deck, "pile"));
    let points = element(&deck, "b-points");
    apply(&mut b, &with_ops(&deck, vec![remove("b-points"), add(Some("pile"), "children", None, points)]));
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        let (read, issues) = r.deck();
        assert_that!(issues).is_empty();
        assert_that!(has(&read, "pile")).is_false();
        assert_that!(has(&read, "b-points")).is_false();
    }
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        let (read, issues) = r.deck();
        assert_that!(issues).is_empty();
        assert_that!(ids_in(&read, "s-constat", None, "blocks")).contains("pile".to_owned());
        assert_that!(ids_in(&read, "s-constat", Some("pile"), "children")).is_equal_to(vec![
            "k1".to_owned(),
            "k2".into(),
            "b-points".into(),
        ]);
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
}

/// Garde. Limite documentée (ADR-0006) : défaire un ajout est un `delete` de Loro.
#[test]
fn undo_of_an_add_deletes_the_node_and_a_child_moved_in_by_the_peer() {
    let deck = roadmap();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    let added = with_ops(&deck, vec![add(None, "blocks", None, json!({ "id": "pile", "kind": "stack" }))]);
    apply(&mut a, &added);
    exchange(&mut a, &mut b);
    let (_, pile) = node_of(&a.doc, "s-constat", "pile");
    let points = element(&deck, "b-points");
    apply(&mut b, &with_ops(&added, vec![remove("b-points"), add(Some("pile"), "children", None, points)]));
    exchange(&mut a, &mut b);
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        let read = r.deck().0;
        assert_that!(has(&read, "pile")).is_false();
        assert_that!(has(&read, "b-points")).is_false();
        assert_that!(ok(slide_tree(r, "s-constat").is_node_deleted(&pile))).is_true();
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
}

/// R1, « Nœud vide » : défaire un `mov` dont la cible a été supprimée par
/// Loro crée un nœud neuf à méta vide, ignoré sans `Issue`.
#[test]
fn undo_of_a_move_after_the_peer_undid_the_add_leaves_an_ignored_empty_node() {
    let deck = roadmap();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    // En tête de `blocks` : le passage dans `elements` déplace vraiment le
    // nœud (en fin de racine) ; sinon le `mov` ne change rien, et l'annuler non plus.
    let added = with_ops(&deck, vec![add(None, "blocks", Some(0), text("e", "Éphémère"))]);
    apply(&mut b, &added);
    exchange(&mut a, &mut b);
    let mut e = element(&added, "e");
    e["frame"] = json!({ "x": 0, "y": 0, "w": 400, "h": 200 });
    apply(&mut a, &with_ops(&added, vec![remove("e"), add(None, "elements", None, e)]));
    assert_that!(ok(b.undo())).is_true();
    exchange(&mut a, &mut b);
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    let tree = slide_tree(&a, "s-constat");
    let empty = tree
        .children(TreeParentId::Node(slide_node(&a, "s-constat")))
        .unwrap_or_default()
        .into_iter()
        .filter(|n| !ok(tree.is_node_deleted(n)))
        .any(|n| ok(tree.get_meta(n)).keys().next().is_none());
    assert_that!(empty).named("le scénario ne reproduit pas le nœud à méta vide").is_true();
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(has(&read, "e")).is_false();
    assert_that!(b.deck()).is_equal_to((read, Vec::new()));
    ok(DeckReplica::from_snapshot(&ok(a.snapshot()), 1 << 24, registry()));
    exchange(&mut a, &mut b);
    for r in [&mut a, &mut b] {
        let before = ops_of(r);
        let read = r.deck().0;
        apply(r, &read);
        assert_that!(ops_of(r)).is_equal_to(before);
    }
}

/// Garde t-188 de `@slot` (R3) : deux pairs qui créent `@slot` en concurrence
/// sur un même nœud gardent leurs deux entrées.
#[test]
fn concurrent_slot_entries_on_a_forged_node_are_both_kept() {
    for (pa, pb) in [(1, 2), (2, 1)] {
        let case = format!("pairs ({pa}, {pb})");
        let (mut a, mut b) = fixed_peers(&with_pile(), pa, pb);
        forge(&a, "s-constat", None, &text_meta("x"), None);
        exchange(&mut a, &mut b);
        let (repaired, issues) = a.deck();
        assert_that!(issue_ids(&issues)).named(&case).is_equal_to(vec!["issue-slot-repaired"]);
        apply(&mut a, &repaired);
        let (seen, _) = b.deck();
        let x = element(&seen, "x");
        apply(&mut b, &with_ops(&seen, vec![remove("x"), add(Some("pile"), "children", None, x)]));
        exchange(&mut a, &mut b);
        for r in [&a, &b] {
            let (read, issues) = r.deck();
            assert_that!(issues).named(&case).is_empty();
            assert_that!(ids_in(&read, "s-constat", Some("pile"), "children"))
                .named(&case)
                .is_equal_to(vec!["x".to_owned()]);
            let (tree, node) = node_of(&r.doc, "s-constat", "x");
            let mut want = serde_json::Map::new();
            want.insert(owner_key(r, "s-constat", None), json!("blocks"));
            want.insert(owner_key(r, "s-constat", Some("pile")), json!("children"));
            assert_that!(slots(&tree, node)).named(&case).is_equal_to(serde_json::Value::Object(want));
        }
    }
}

/// Les marques mal formées que l'entrée refuse.
const MALFORMED_MARKS: [(&str, bool); 5] =
    [("@deleted/9", false), ("@deleted/abc", true), ("@deleted/09", true), ("@deleted", true), ("@deletedx", true)];

/// Le genre d'erreur de `from_snapshot` sur le snapshot de `r`, s'il échoue.
fn import_error(r: &DeckReplica) -> Option<ReplicaErrorKind> {
    DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry()).err().map(|e| e.kind())
}

/// Applique `edit` à la méta de `b-points` de `r`, puis committe.
fn forge_on_points(r: &DeckReplica, edit: impl FnOnce(&LoroMap)) {
    let (tree, n) = node_of(&r.doc, "s-constat", "b-points");
    edit(&ok(tree.get_meta(n)));
    r.doc.commit();
}

/// Une écriture forgée dans une méta.
type Forge = fn(&LoroMap);

/// Les `@slot` non fusionnables que l'entrée refuse : carte ordinaire, valeur, texte.
fn non_mergeable_slots() -> [(&'static str, Forge); 3] {
    [
        ("carte ordinaire", |m| {
            ok(m.insert_container(SLOT, LoroMap::new()));
        }),
        ("valeur", |m| ok(m.insert(SLOT, "texte"))),
        ("texte", |m| {
            ok(m.insert_container(SLOT, LoroText::new()));
        }),
    ]
}

#[test]
fn from_snapshot_with_a_malformed_mark_is_refused() {
    for (key, value) in MALFORMED_MARKS {
        let r = new(&roadmap());
        forge_on_points(&r, |m| ok(m.insert(key, value)));
        assert_that!(import_error(&r)).named(key).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    }
}

#[test]
fn from_snapshot_with_a_non_mergeable_slot_is_refused() {
    for (case, edit) in non_mergeable_slots() {
        let r = new(&roadmap());
        forge_on_points(&r, edit);
        assert_that!(import_error(&r)).named(case).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    }
}

#[test]
fn merge_of_a_peer_with_a_malformed_mark_is_refused_and_keeps_self() {
    let (mut a, b) = fixed_peers(&roadmap(), 1, 2);
    forge_on_points(&b, |m| ok(m.insert("@deleted/9", false)));
    let before = a.deck();
    assert_that!(a.merge(&b).err().map(|e| e.kind())).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    assert_that!(a.deck()).is_equal_to(before);
}

#[test]
fn merge_of_a_peer_with_a_non_mergeable_slot_is_refused_and_keeps_self() {
    let (mut a, b) = fixed_peers(&roadmap(), 1, 2);
    forge_on_points(&b, |m| {
        ok(m.insert_container(SLOT, LoroMap::new()));
    });
    let before = a.deck();
    assert_that!(a.merge(&b).err().map(|e| e.kind())).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    assert_that!(a.deck()).is_equal_to(before);
}
