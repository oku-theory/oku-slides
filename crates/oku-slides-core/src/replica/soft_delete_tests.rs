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
    assert_that!(matches!(value, Some(ValueOrContainer::Value(LoroValue::I64(_))))).is_true();
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
    ok(ok(tree.get_meta(pile)).insert("@deleted/9", 1_700_000_000_i64));
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
    marked.push(("@deleted/9", LoroValue::I64(1_700_000_000)));
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

/// t-776 : défaire un ajout marque le nœud ; l'enfant que le pair y a déplacé
/// est caché avec lui, et rétablir rend le même nœud avec l'enfant.
#[test]
fn undo_of_an_add_marks_the_node_and_hides_a_child_moved_in_by_the_peer() {
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
        let tree = slide_tree(r, "s-constat");
        assert_that!(ok(tree.is_node_deleted(&pile))).is_false();
        assert_that!(marks(&tree, pile)).is_equal_to(vec![mark_key(1)]);
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
    assert_that!(ok(a.redo())).is_true();
    exchange(&mut a, &mut b);
    assert_that!(node_of(&a.doc, "s-constat", "pile").1).is_equal_to(pile);
    assert_that!(ids_in(&a.deck().0, "s-constat", Some("pile"), "children")).is_equal_to(vec!["b-points".to_owned()]);
}

/// t-776 : défaire un `mov` après l'annulation de l'ajout par le pair ne crée
/// plus de nœud vide : le nœud, marqué par le pair, reçoit le `mov` défait.
#[test]
fn undo_of_a_move_after_the_peer_undid_the_add_leaves_no_empty_node() {
    let deck = roadmap();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    let added = with_ops(&deck, vec![add(None, "blocks", Some(0), text("e", "Éphémère"))]);
    apply(&mut b, &added);
    exchange(&mut a, &mut b);
    let (_, node) = node_of(&a.doc, "s-constat", "e");
    let mut e = element(&added, "e");
    e["frame"] = json!({ "x": 0, "y": 0, "w": 400, "h": 200 });
    apply(&mut a, &with_ops(&added, vec![remove("e"), add(None, "elements", None, e)]));
    assert_that!(ok(b.undo())).is_true();
    exchange(&mut a, &mut b);
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    let tree = slide_tree(&a, "s-constat");
    assert_that!(ok(tree.is_node_deleted(&node))).named("nœud de e").is_false();
    assert_that!(marks(&tree, node)).is_equal_to(vec![mark_key(2)]);
    let empty =
        tree.nodes().into_iter().filter(|n| !ok(tree.is_node_deleted(n))).any(|n| ok(tree.get_meta(n)).is_empty());
    assert_that!(empty).is_false();
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

/// t-776 : un collage se défait d'un bloc (nœuds marqués, pas supprimés), et
/// rétablir rend les mêmes nœuds.
#[test]
fn undo_of_a_paste_marks_every_pasted_node_and_redo_restores_the_same_nodes() {
    let deck = roadmap();
    let mut r = new(&deck);
    let ids: Vec<String> = (0..5).map(|i| format!("p{i}")).collect();
    let pasted = with_ops(&deck, ids.iter().map(|id| add(None, "blocks", None, text(id, "Collé"))).collect());
    apply(&mut r, &pasted);
    let before: Vec<TreeID> = ids.iter().map(|id| node_of(&r.doc, "s-constat", id).1).collect();
    assert_that!(ok(r.undo())).is_true();
    let tree = deck_tree(&r);
    for n in &before {
        assert_that!(ok(tree.is_node_deleted(n))).is_false();
        assert_that!(marks(&tree, *n)).is_equal_to(vec![mark_key(r.peer_id())]);
    }
    assert_that!(r.can_undo()).is_false();
    assert_that!(ok(r.redo())).is_true();
    let after: Vec<TreeID> = ids.iter().map(|id| node_of(&r.doc, "s-constat", id).1).collect();
    assert_that!(after).is_equal_to(before);
}

/// t-776 : `apply_silent` crée sans commit de naissance ; `apply`, avec un seul pour toutes les créations.
#[test]
fn apply_silent_adds_one_change_and_apply_a_bounded_number() {
    let deck = roadmap();
    let three = |prefix: &str| {
        with_ops(&deck, (0..3).map(|i| add(None, "blocks", None, text(&format!("{prefix}{i}"), "x"))).collect())
    };
    let mut r = new(&deck);
    let before = r.len_changes();
    ok(r.apply_silent(&three("s")));
    assert_that!(r.len_changes() - before).is_equal_to(1);
    let mut r = new(&deck);
    let before = r.len_changes();
    apply(&mut r, &three("a"));
    // Commit de naissance, commit final (et la partie qui précède, si non vide).
    assert_that!(r.len_changes() - before).is_less_than_or_equal_to(3);
    assert_that!(r.len_changes() - before).is_greater_than_or_equal_to(2);
}

/// t-776 : toutes les créations d'une application naissent en un seul commit.
#[test]
fn apply_with_nested_creations_adds_a_bounded_number_of_changes() {
    let deck = roadmap();
    let mut r = new(&deck);
    let nested = |i: usize| {
        json!({ "id": format!("q{i}"), "kind": "stack",
        "params": { "children": [text(&format!("q{i}a"), "a"), text(&format!("q{i}b"), "b")] } })
    };
    let before = r.len_changes();
    apply(&mut r, &with_ops(&deck, (0..20).map(|i| add(None, "blocks", None, nested(i))).collect()));
    // Partie qui précède, commit de naissance, commit final.
    assert_that!(r.len_changes() - before).is_less_than_or_equal_to(3);
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
const MALFORMED_MARKS: [(&str, LoroValue); 7] = [
    ("@deleted/9", LoroValue::Bool(false)),
    ("@deleted/9", LoroValue::Bool(true)),
    ("@deleted/abc", LoroValue::I64(1_700_000_000)),
    ("@deleted/09", LoroValue::I64(1_700_000_000)),
    ("@deleted", LoroValue::I64(1_700_000_000)),
    ("@deletedx", LoroValue::I64(1_700_000_000)),
    ("@deleted/9", LoroValue::Double(1.5)),
];

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
        forge_on_points(&r, |m| ok(m.insert(key, value.clone())));
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

/// `pile` et, dans `children`, le seul texte `k1`.
fn with_single_child() -> Deck {
    with_ops(&with_pile(), vec![add(Some("pile"), "children", None, text("k1", "Un"))])
}

/// Le dernier déplacement de chaque nœud racine ou enfant de `r`, par nœud.
fn last_moves(r: &DeckReplica) -> Vec<(TreeID, Option<loro::ID>)> {
    let tree = deck_tree(r);
    tree.nodes().into_iter().map(|n| (n, tree.get_last_move_id(&n))).collect()
}

/// t-776 : coller avant un frère existant ne le déplace pas (seul le nœud neuf est rangé).
#[test]
fn pasting_before_an_existing_element_does_not_move_it() {
    let base = with_single_child();
    let mut r = new(&base);
    let before = last_moves(&r);
    apply(&mut r, &with_ops(&base, vec![add(Some("pile"), "children", Some(0), text("k0", "Zéro"))]));
    let after = last_moves(&r);
    for (n, mv) in &before {
        assert_that!(after.iter().find(|(m, _)| m == n).map(|(_, v)| *v))
            .named("dernier mov d'un nœud existant")
            .is_equal_to(Some(*mv));
    }
    assert_that!(ids_in(&r.deck().0, "s-constat", Some("pile"), "children"))
        .is_equal_to(vec!["k0".to_owned(), "k1".into()]);
}

/// t-776 : une slide collée en tête ne déplace aucune slide existante.
#[test]
fn pasting_a_slide_at_the_front_does_not_move_existing_slides() {
    let mut base = roadmap();
    base.slides.truncate(1);
    let mut r = new(&base);
    let before = last_moves(&r);
    let mut pasted = base.clone();
    pasted.slides.insert(0, crate::Slide::new("s-neuve-2", "content"));
    pasted.slides.insert(0, crate::Slide::new("s-neuve", "content"));
    apply(&mut r, &pasted);
    let after = last_moves(&r);
    for (n, mv) in &before {
        assert_that!(after.iter().find(|(m, _)| m == n).map(|(_, v)| *v))
            .named("dernier mov d'une slide existante")
            .is_equal_to(Some(*mv));
    }
    assert_that!(r.deck().0.slides.first().map(|s| s.id.clone())).is_equal_to(Some("s-neuve".to_owned()));
}

/// t-776 : le déplacement d'un pair survit quand un collage précède l'élément déplacé.
#[test]
fn a_peer_move_survives_a_paste_before_the_moved_element() {
    let base = with_single_child();
    let (mut a, mut b) = fixed_peers(&base, 1, 2);
    let mut k1 = element(&base, "k1");
    k1["frame"] = json!({ "x": 0, "y": 0, "w": 400, "h": 200 });
    apply(&mut b, &with_ops(&base, vec![remove("k1"), add(None, "elements", None, k1)]));
    apply(&mut a, &with_ops(&base, vec![add(Some("pile"), "children", Some(0), text("k0", "Zéro"))]));
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        assert_that!(ids_in(&r.deck().0, "s-constat", None, "elements")).contains("k1".to_owned());
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
}

/// `apply` sans naissance marquée (`born = None`) : la base d'avant t-776, pour la mesure.
fn apply_without_birth(r: &mut DeckReplica, deck: &Deck) {
    let doc = &r.doc;
    let split = || {
        doc.set_next_commit_message(&crate::replica::part_message("Coller"));
        doc.commit();
    };
    ok(DeckReplica::reconcile_doc(doc, deck, &r.registry, &split, None, &[]));
    doc.set_next_commit_message("Coller");
    doc.commit();
    r.undo.close_step("Coller", None);
}

fn median(mut v: Vec<std::time::Duration>) -> std::time::Duration {
    v.sort();
    v[v.len() / 2]
}

/// Mesure t-776 : coût d'un collage, à plat et imbriqué, avec et sans naissance marquée.
///
/// Alterne les deux variantes, 5 répétitions chacune, et affiche les médianes.
#[test]
#[ignore = "mesure"]
fn measure_paste_cost_flat_and_nested() {
    for n in [50_usize, 500] {
        for nested in [false, true] {
            let deck = with_pile();
            let element = |i: usize| {
                if nested {
                    json!({ "id": format!("q{i}"), "kind": "stack",
                        "params": { "children": [text(&format!("q{i}a"), "a"), text(&format!("q{i}b"), "b")] } })
                } else {
                    text(&format!("q{i}"), "Collé")
                }
            };
            let pasted = with_ops(&deck, (0..n).map(|i| add(None, "blocks", None, element(i))).collect());
            let (mut before, mut after) = ((Vec::new(), Vec::new()), (Vec::new(), Vec::new()));
            let mut items = (0, 0);
            for _ in 0..5 {
                for born in [false, true] {
                    let mut r = new(&deck);
                    let start = r.undo.items();
                    let t = std::time::Instant::now();
                    if born {
                        apply(&mut r, &pasted);
                    } else {
                        apply_without_birth(&mut r, &pasted);
                    }
                    let apply_time = t.elapsed();
                    let step = r.undo.items() - start;
                    let t = std::time::Instant::now();
                    assert_that!(ok(r.undo())).is_true();
                    let undo_time = t.elapsed();
                    let (side, it) = if born { (&mut after, &mut items.1) } else { (&mut before, &mut items.0) };
                    side.0.push(apply_time);
                    side.1.push(undo_time);
                    *it = step;
                }
            }
            let (ba, bu, aa, au) = (median(before.0), median(before.1), median(after.0), median(after.1));
            println!(
                "collage n={n} imbriqué={nested} : avant apply {ba:?} ⌘Z {bu:?} items {} | après apply {aa:?} ⌘Z {au:?} items {} | rapport ⌘Z {:.1}x apply {:.1}x",
                items.0,
                items.1,
                au.as_secs_f64() / bu.as_secs_f64(),
                aa.as_secs_f64() / ba.as_secs_f64()
            );
        }
    }
}

/// t-770 : une marque de retrait porte l'heure Unix de sa pose.
#[test]
fn mark_of_a_removal_carries_its_unix_time() {
    let deck = roadmap();
    let mut r = new(&deck);
    let (_, points) = node_of(&r.doc, "s-constat", "b-points");
    apply(&mut r, &without(&deck, "b-points"));
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    match ok(deck_tree(&r).get_meta(points)).get(&mark_key(r.peer_id())) {
        Some(ValueOrContainer::Value(LoroValue::I64(t))) => assert_that!((now - t).abs()).is_less_than(60),
        other => panic!("marque datée attendue : {other:?}"),
    }
}

/// t-770 : défaire un ajout remet la marque de naissance, datée de la création.
#[test]
fn undo_of_an_add_restores_a_birth_mark_dated_at_creation() {
    let deck = roadmap();
    let mut r = new(&deck);
    apply(&mut r, &with_ops(&deck, vec![add(None, "blocks", None, text("n1", "Neuf"))]));
    let (tree, n) = node_of(&r.doc, "s-constat", "n1");
    assert_that!(ok(r.undo())).is_true();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    match ok(tree.get_meta(n)).get(&mark_key(r.peer_id())) {
        Some(ValueOrContainer::Value(LoroValue::I64(t))) => assert_that!((now - t).abs()).is_less_than(60),
        other => panic!("marque de naissance datée attendue : {other:?}"),
    }
}
