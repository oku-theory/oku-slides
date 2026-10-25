//! Sonde de caractérisation de `LoroTree` (loro 1.16.2), pour t-694 : les
//! éléments d'une slide peuvent-ils passer d'une `LoroMovableList` par slot à
//! un `LoroTree` par slide ? Ces tests fixent le comportement observé de la
//! bibliothèque, pas celui du produit. Voir l'ADR-0005.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
#![expect(clippy::many_single_char_names, reason = "probe: peers, trees and nodes named as in the described scenario")]

use std::time::Instant;

use loro::{
    Container, ContainerTrait, ExportMode, LoroDoc, LoroMap, LoroText, LoroTree, LoroValue, TreeID, TreeParentId,
    UndoManager, ValueOrContainer,
};
use speculoos::prelude::*;

const TREE: &str = "elements";

/// Un pair déterministe, réglé comme `replica::configure` (sans horodatage).
fn peer(id: u64) -> LoroDoc {
    let doc = LoroDoc::new();
    doc.set_peer_id(id).unwrap_or_else(|e| panic!("loro : {e}"));
    doc.set_record_timestamp(false);
    doc.set_change_merge_interval(-1);
    doc
}

/// Un second pair qui part de l'état de `a`.
fn fork(a: &LoroDoc, id: u64) -> LoroDoc {
    let b = peer(id);
    b.import(&a.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("loro : {e}")))
        .unwrap_or_else(|e| panic!("loro : {e}"));
    b
}

/// Import croisé de tout ce que chaque pair a.
fn sync(a: &LoroDoc, b: &LoroDoc) {
    a.import(&b.export(ExportMode::all_updates()).unwrap_or_else(|e| panic!("loro : {e}")))
        .unwrap_or_else(|e| panic!("loro : {e}"));
    b.import(&a.export(ExportMode::all_updates()).unwrap_or_else(|e| panic!("loro : {e}")))
        .unwrap_or_else(|e| panic!("loro : {e}"));
}

fn tree(doc: &LoroDoc) -> LoroTree {
    let t = doc.get_tree(TREE);
    t.enable_fractional_index(0);
    t
}

fn meta_str(t: &LoroTree, id: TreeID, key: &str) -> Option<String> {
    match t.get_meta(id).unwrap_or_else(|e| panic!("loro : {e}")).get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

fn meta_text(meta: &LoroMap) -> LoroText {
    match meta.get("text") {
        Some(ValueOrContainer::Container(Container::Text(t))) => t,
        other => panic!("pas de LoroText sous `text` : {other:?}"),
    }
}

/// Élément `x` à méta `kind`, `@slot` et texte `text`.
fn element(t: &LoroTree, parent: TreeID, slot: &str, text: &str) -> TreeID {
    let x = t.create(parent).unwrap_or_else(|e| panic!("loro : {e}"));
    let meta = t.get_meta(x).unwrap_or_else(|e| panic!("loro : {e}"));
    meta.insert("kind", "text").unwrap_or_else(|e| panic!("loro : {e}"));
    meta.insert("@slot", slot).unwrap_or_else(|e| panic!("loro : {e}"));
    meta.insert_container("text", LoroText::new())
        .unwrap_or_else(|e| panic!("loro : {e}"))
        .insert(0, text)
        .unwrap_or_else(|e| panic!("loro : {e}"));
    x
}

fn kids(t: &LoroTree, p: TreeID) -> Vec<TreeID> {
    t.children(p).unwrap_or_default()
}

/// Chaîne des parents de `x` jusqu'à la racine ; panique sur un cycle.
fn ancestors(t: &LoroTree, x: TreeID) -> Vec<TreeParentId> {
    let mut out = Vec::new();
    let mut cur = TreeParentId::Node(x);
    while let TreeParentId::Node(n) = cur {
        assert!(out.len() <= t.nodes().len(), "cycle autour de {x:?}");
        cur = t.parent(n).unwrap_or_else(|| panic!("{n:?} absent"));
        out.push(cur);
    }
    out
}

fn same(a: &LoroDoc, b: &LoroDoc) {
    assert_that!(a.get_deep_value()).is_equal_to(b.get_deep_value());
}

/// Point 1a : Déplacement entre parents chez A, méta (clé + texte) de x chez B.
#[test]
fn p1_move_across_parents_keeps_concurrent_meta_edit() {
    let a = peer(1);
    let t = tree(&a);
    let slide = t.create(None).unwrap();
    let (p1, p2) = (t.create(slide).unwrap(), t.create(slide).unwrap());
    let x = element(&t, p1, "a", "hello");
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);

    t.mov(x, p2).unwrap();
    a.commit();
    let mb = tb.get_meta(x).unwrap();
    mb.insert("kind", "title").unwrap();
    meta_text(&mb).insert(5, " world").unwrap();
    b.commit();
    sync(&a, &b);

    same(&a, &b);
    for t in [&t, &tb] {
        assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(p2)));
        assert_that!(meta_str(t, x, "kind")).is_equal_to(Some("title".to_owned()));
        assert_that!(meta_text(&t.get_meta(x).unwrap()).to_string()).is_equal_to("hello world".to_owned());
        assert_that!(kids(t, p1)).is_empty();
    }
}

/// Point 1b : `mov_to` sous le même parent avec `@slot` changé chez A, méta chez B.
#[test]
fn p1_same_parent_slot_change_keeps_concurrent_meta_edit() {
    let a = peer(1);
    let t = tree(&a);
    let slide = t.create(None).unwrap();
    let x = element(&t, slide, "a", "hello");
    let y = element(&t, slide, "a", "y");
    let z = element(&t, slide, "b", "z");
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);

    t.mov_to(x, slide, 2).unwrap();
    t.get_meta(x).unwrap().insert("@slot", "b").unwrap();
    a.commit();
    let mb = tb.get_meta(x).unwrap();
    mb.insert("color", "red").unwrap();
    meta_text(&mb).insert(0, ">").unwrap();
    b.commit();
    sync(&a, &b);

    same(&a, &b);
    for t in [&t, &tb] {
        assert_that!(kids(t, slide)).is_equal_to(vec![y, z, x]);
        assert_that!(meta_str(t, x, "@slot")).is_equal_to(Some("b".to_owned()));
        assert_that!(meta_str(t, x, "color")).is_equal_to(Some("red".to_owned()));
        assert_that!(meta_text(&t.get_meta(x).unwrap()).to_string()).is_equal_to(">hello".to_owned());
    }
}

/// Point 2 : Cycle : A (pair 1) met x sous y, B (pair 2) met y sous x. Les `mov`
/// sont appliqués dans l'ordre (lamport, pair) ; celui qui créerait un cycle
/// est ignoré. Le PREMIER dans cet ordre gagne : ici A.
#[test]
fn p2_concurrent_cycle_first_move_in_order_wins() {
    let a = peer(1);
    let t = tree(&a);
    let r = t.create(None).unwrap();
    let x = t.create(r).unwrap();
    let y = t.create(r).unwrap();
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);

    t.mov(x, y).unwrap();
    a.commit();
    tb.mov(y, x).unwrap();
    b.commit();
    sync(&a, &b);

    same(&a, &b);
    for t in [&t, &tb] {
        for n in [r, x, y] {
            assert_that!(t.is_node_deleted(&n).unwrap()).is_false();
            ancestors(t, n);
        }
        assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(y)));
        assert_that!(t.parent(y)).is_equal_to(Some(TreeParentId::Node(r)));
    }
}

/// Point 2b : Même cycle, pairs inversés : c'est l'ordre (lamport, pair) qui tranche,
/// le plus petit pair gagne à lamport égal (B ici).
#[test]
fn p2_cycle_winner_is_the_lower_peer_at_equal_lamport() {
    let a = peer(2);
    let t = tree(&a);
    let r = t.create(None).unwrap();
    let x = t.create(r).unwrap();
    let y = t.create(r).unwrap();
    a.commit();
    let b = fork(&a, 1);
    let tb = tree(&b);

    t.mov(x, y).unwrap();
    a.commit();
    tb.mov(y, x).unwrap();
    b.commit();
    sync(&a, &b);

    same(&a, &b);
    assert_that!(t.parent(y)).is_equal_to(Some(TreeParentId::Node(x)));
    assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(r)));
}

/// Point 3a : A supprime p, B déplace x sous p : x part avec p (supprimé, invisible),
/// sa méta reste lisible.
#[test]
fn p3_move_under_concurrently_deleted_parent_hides_node() {
    let a = peer(1);
    let t = tree(&a);
    let slide = t.create(None).unwrap();
    let p = t.create(slide).unwrap();
    let x = element(&t, slide, "a", "hello");
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);

    t.delete(p).unwrap();
    a.commit();
    tb.mov(x, p).unwrap();
    b.commit();
    sync(&a, &b);

    same(&a, &b);
    for t in [&t, &tb] {
        assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(p)));
        assert_that!(t.is_node_deleted(&p).unwrap()).is_true();
        assert_that!(t.is_node_deleted(&x).unwrap()).is_true();
        assert_that!(t.contains(x)).is_true();
        assert_that!(kids(t, slide)).is_empty();
        assert_that!(t.get_nodes(false).iter().any(|n| n.id == x)).is_false();
        assert_that!(meta_str(t, x, "kind")).is_equal_to(Some("text".to_owned()));
    }
}

/// Point 3b : A supprime p, B sort x de p en parallèle : x survit, hors de p.
#[test]
fn p3_move_out_of_concurrently_deleted_parent_saves_node() {
    let a = peer(1);
    let t = tree(&a);
    let slide = t.create(None).unwrap();
    let p = t.create(slide).unwrap();
    let x = element(&t, p, "a", "hello");
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);

    t.delete(p).unwrap();
    a.commit();
    tb.mov(x, slide).unwrap();
    b.commit();
    sync(&a, &b);

    same(&a, &b);
    for t in [&t, &tb] {
        assert_that!(t.is_node_deleted(&x).unwrap()).is_false();
        assert_that!(kids(t, slide)).is_equal_to(vec![x]);
    }
}

/// Point 4a : Le fractionnaire est actif par défaut (jitter 0) dans 1.16.2.
#[test]
fn p4_fractional_index_is_enabled_by_default() {
    let doc = peer(1);
    assert_that!(doc.get_tree(TREE).is_fractional_index_enabled()).is_true();
}

/// Point 4b: two concurrent insertions at the same index.
///
/// They get the same fractional index (jitter 0), tie-broken by `TreeID`; the order converges and `children` returns it.
#[test]
fn p4_concurrent_inserts_at_same_index_converge() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let c: Vec<TreeID> = (0..3).map(|_| t.create(s).unwrap()).collect();
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);

    let na = t.create_at(s, 1).unwrap();
    a.commit();
    let nb = tb.create_at(s, 1).unwrap();
    b.commit();
    sync(&a, &b);

    same(&a, &b);
    let order = kids(&t, s);
    assert_that!(kids(&tb, s)).is_equal_to(&order);
    assert_that!(t.fractional_index(na)).is_equal_to(t.fractional_index(nb));
    assert_that!(order).is_equal_to(vec![c[0], na, nb, c[1], c[2]]);
    // `children` suit l'ordre (fractionnaire, TreeID).
    let mut sorted = order.clone();
    sorted.sort_by_key(|n| (t.fractional_index(*n).unwrap(), *n));
    assert_that!(sorted).is_equal_to(&order);
}

/// Point 4c : `mov_to(x, p, k)` met x à l'index k de la liste finale, vers l'avant
/// comme vers l'arrière, et depuis un autre parent.
#[test]
fn p4_mov_to_index_k_lands_at_k() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let other = t.create(None).unwrap();
    for _ in 0..6 {
        t.create(s).unwrap();
    }
    for (from, k) in [(0, 3), (5, 1), (2, 5), (4, 0), (1, 1)] {
        let x = kids(&t, s)[from];
        t.mov_to(x, s, k).unwrap();
        assert_that!(kids(&t, s).iter().position(|n| *n == x)).is_equal_to(Some(k));
    }
    let y = t.create(other).unwrap();
    t.mov_to(y, s, 2).unwrap();
    assert_that!(kids(&t, s)[2]).is_equal_to(y);
    assert_that!(kids(&t, s).len()).is_equal_to(7);
}

/// Un `UndoManager` réglé comme `replica::undo`.
fn undo_manager(doc: &LoroDoc) -> UndoManager {
    let mut m = UndoManager::new(doc);
    m.set_merge_interval(0);
    m
}

/// Point 5a : Défaire un déplacement entre parents : place (parent et index) et méta
/// retrouvées ; rétablir le refait.
#[test]
fn p5_undo_redo_move_across_parents() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let (p1, p2) = (t.create(s).unwrap(), t.create(s).unwrap());
    let before = t.create(p1).unwrap();
    let x = element(&t, p1, "a", "hello");
    let after = t.create(p1).unwrap();
    t.create(p2).unwrap();
    a.commit();
    let mut m = undo_manager(&a);

    t.mov_to(x, p2, 0).unwrap();
    a.commit();
    assert_that!(m.undo().unwrap()).is_true();
    assert_that!(kids(&t, p1)).is_equal_to(vec![before, x, after]);
    assert_that!(meta_text(&t.get_meta(x).unwrap()).to_string()).is_equal_to("hello".to_owned());
    assert_that!(meta_str(&t, x, "@slot")).is_equal_to(Some("a".to_owned()));
    assert_that!(m.redo().unwrap()).is_true();
    assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(p2)));
    assert_that!(kids(&t, p2)[0]).is_equal_to(x);
}

/// Point 5b: undoing a deletion does NOT revive the node.
///
/// x stays deleted, and a NEW node (another `TreeID`) is created in its place, with a copy of its meta
/// (text included, in a new `LoroText`) and of its subtree, also
/// recreated. Each redo/undo recreates a node again. Same mechanism as
/// lists (t-652).
#[test]
fn p5_undo_delete_recreates_a_new_node_with_copied_meta() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let first = t.create(s).unwrap();
    let x = element(&t, s, "a", "hello");
    let child = t.create(x).unwrap();
    t.get_meta(child).unwrap().insert("kind", "child").unwrap();
    let last = t.create(s).unwrap();
    a.commit();
    let mut m = undo_manager(&a);

    t.delete(x).unwrap();
    a.commit();
    assert_that!(m.undo().unwrap()).is_true();
    assert_that!(t.is_node_deleted(&x).unwrap()).is_true();
    let back = kids(&t, s);
    assert_that!(back.len()).is_equal_to(3);
    assert_that!(back[0]).is_equal_to(first);
    assert_that!(back[2]).is_equal_to(last);
    let n = back[1];
    assert_that!(n).is_not_equal_to(x);
    let meta = t.get_meta(n).unwrap();
    assert_that!(meta.get_deep_value()).is_equal_to(t.get_meta(x).unwrap().get_deep_value());
    let (old_text, new_text) = (meta_text(&t.get_meta(x).unwrap()), meta_text(&meta));
    assert_that!(new_text.to_string()).is_equal_to("hello".to_owned());
    assert_that!(new_text.id()).is_not_equal_to(old_text.id());
    let sub = kids(&t, n);
    assert_that!(sub.len()).is_equal_to(1);
    assert_that!(sub[0]).is_not_equal_to(child);
    assert_that!(meta_str(&t, sub[0], "kind")).is_equal_to(Some("child".to_owned()));

    assert_that!(m.redo().unwrap()).is_true();
    assert_that!(t.is_node_deleted(&n).unwrap()).is_true();
    assert_that!(m.undo().unwrap()).is_true();
    let again = kids(&t, s)[1];
    assert_that!(again).is_not_equal_to(n);
    assert_that!(again).is_not_equal_to(x);
}

/// Point 5c: B's concurrent `mov` (peer 2) arrives BEFORE the undo.
///
/// It comes after the deletion in the order, so x is alive (under p2). The undo
/// recreates nothing but PUTS x BACK under p1 (its place before the deletion): B's
/// move is undone. A single node, same `TreeID`.
#[test]
fn p5_undo_delete_after_receiving_concurrent_move_moves_x_back() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let (p1, p2) = (t.create(s).unwrap(), t.create(s).unwrap());
    let x = element(&t, p1, "a", "hello");
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);
    let mut m = undo_manager(&a);

    t.delete(x).unwrap();
    a.commit();
    tb.mov(x, p2).unwrap();
    b.commit();
    sync(&a, &b);
    assert_that!(t.is_node_deleted(&x).unwrap()).is_false();
    assert_that!(m.undo().unwrap()).is_true();
    a.commit();
    sync(&a, &b);

    same(&a, &b);
    assert_that!(t.nodes().len()).is_equal_to(4);
    assert_that!(kids(&t, p1)).is_equal_to(vec![x]);
    assert_that!(kids(&t, p2)).is_empty();
}

/// Point 5d : Annulation de la suppression AVANT de recevoir le `mov` + édition de
/// méta concurrents de B. Si B passe après la suppression (pair 2) : DOUBLON,
/// x chez B avec son édition, plus le nœud recréé par A (t-652 à l'identique).
/// Si B passe avant (pair 0) : x reste supprimé, l'édition de B est PERDUE,
/// seul le nœud recréé par A est visible.
#[test]
fn p5_undo_delete_before_concurrent_move_duplicates_or_loses_edit() {
    for (bid, duplicate) in [(2u64, true), (0, false)] {
        let a = peer(1);
        let t = tree(&a);
        let s = t.create(None).unwrap();
        let (p1, p2) = (t.create(s).unwrap(), t.create(s).unwrap());
        let x = element(&t, p1, "a", "hello");
        a.commit();
        let b = fork(&a, bid);
        let tb = tree(&b);
        let mut m = undo_manager(&a);

        t.delete(x).unwrap();
        a.commit();
        assert_that!(m.undo().unwrap()).is_true();
        a.commit();
        tb.mov(x, p2).unwrap();
        tb.get_meta(x).unwrap().insert("kind", "title").unwrap();
        b.commit();
        sync(&a, &b);

        same(&a, &b);
        let n = kids(&t, p1);
        assert_that!(n.len()).is_equal_to(1);
        assert_that!(n[0]).is_not_equal_to(x);
        assert_that!(meta_str(&t, n[0], "kind")).is_equal_to(Some("text".to_owned()));
        assert_that!(t.is_node_deleted(&x).unwrap()).is_equal_to(!duplicate);
        if duplicate {
            assert_that!(kids(&t, p2)).is_equal_to(vec![x]);
            assert_that!(meta_str(&t, x, "kind")).is_equal_to(Some("title".to_owned()));
        } else {
            assert_that!(kids(&t, p2)).is_empty();
        }
    }
}

/// Point 6 : Plusieurs `mov` dans un seul commit (échange non voisin, et passage
/// d'un parent à l'autre), défaits d'un coup : l'arbre revient exactement,
/// sans perte ; le bug t-263 ne se reproduit pas.
#[test]
fn p6_many_moves_in_one_commit_undo_cleanly() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let (p1, p2) = (t.create(s).unwrap(), t.create(s).unwrap());
    let c: Vec<TreeID> = (0..5).map(|i| element(&t, p1, "a", &format!("e{i}"))).collect();
    a.commit();
    let initial = a.get_deep_value();
    let mut m = undo_manager(&a);

    // Échange de c0 et c3, inversion de la suite, puis c4 vers p2.
    t.mov_to(c[0], p1, 3).unwrap();
    t.mov_to(c[3], p1, 0).unwrap();
    t.mov_before(c[2], c[1]).unwrap();
    t.mov_after(c[1], c[0]).unwrap();
    t.mov_to(c[4], p2, 0).unwrap();
    a.commit();
    let done = a.get_deep_value();
    let moved = kids(&t, p1);

    assert_that!(m.undo().unwrap()).is_true();
    assert_that!(kids(&t, p1)).is_equal_to(&c);
    assert_that!(kids(&t, p2)).is_empty();
    assert_that!(a.get_deep_value()).is_equal_to(&initial);
    assert_that!(m.can_undo()).is_false();
    assert_that!(m.redo().unwrap()).is_true();
    assert_that!(kids(&t, p1)).is_equal_to(&moved);
    assert_that!(a.get_deep_value()).is_equal_to(done);
}

/// Les `mov` (from, to) qu'emploie `Writer::reorder` pour passer de `0..n` à `wanted`.
fn reorder_moves(wanted: &[usize]) -> Vec<(usize, usize)> {
    let mut keys: Vec<usize> = (0..wanted.len()).collect();
    let mut out = Vec::new();
    for (target, key) in wanted.iter().enumerate() {
        if keys[target] != *key {
            let from = keys.iter().position(|k| k == key).unwrap_or_else(|| panic!("{key} absent"));
            out.push((from, target));
            let k = keys.remove(from);
            keys.insert(target, k);
        }
    }
    out
}

/// Tous les échanges (i, j) de 8 éléments, i != j : 56 cas.
fn swaps() -> Vec<Vec<usize>> {
    let n = 8;
    let mut out = Vec::new();
    for i in 0..n {
        for j in (0..n).filter(|j| *j != i) {
            let mut w: Vec<usize> = (0..n).collect();
            w.swap(i, j);
            out.push(w);
        }
    }
    out
}

/// Point 6b : Les 56 échanges de 8 nœuds, `mov` de `reorder` dans UN commit, défaits
/// puis rétablis : l'arbre revient exactement à chaque fois.
#[test]
fn p6_every_swap_in_one_commit_undoes_and_redoes_exactly() {
    for wanted in swaps() {
        let a = peer(1);
        let t = tree(&a);
        let s = t.create(None).unwrap();
        let c: Vec<TreeID> = (0..wanted.len()).map(|k| element(&t, s, "a", &format!("e{k}"))).collect();
        a.commit();
        let initial = a.get_deep_value();
        let mut m = undo_manager(&a);
        for (from, to) in reorder_moves(&wanted) {
            let x = kids(&t, s)[from];
            t.mov_to(x, s, to).unwrap();
        }
        a.commit();
        let want: Vec<TreeID> = wanted.iter().map(|k| c[*k]).collect();
        assert_that!(kids(&t, s)).is_equal_to(&want);

        assert_that!(m.undo().unwrap()).is_true();
        assert_that!(kids(&t, s)).is_equal_to(&c);
        assert_that!(a.get_deep_value()).is_equal_to(&initial);
        assert_that!(m.redo().unwrap()).is_true();
        assert_that!(kids(&t, s)).is_equal_to(&want);
    }
}

/// Point 6c : Témoin t-263 : les mêmes 56 échanges sur une `LoroMovableList` de
/// cartes, dans UN commit. Défaire perd un élément dans 32 cas, et rétablir
/// panique dans 10 autres : 42 sur 56, le chiffre de t-263.
#[test]
fn p6_control_movable_list_reproduces_t263() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let (mut lost, mut panicked) = (0, 0);
    for wanted in swaps() {
        let a = peer(1);
        let slides = a.get_movable_list("slides");
        for k in 0..wanted.len() {
            let m = slides.insert_container(k, LoroMap::new()).unwrap();
            m.insert("id", format!("s{k}")).unwrap();
        }
        a.commit();
        let initial = a.get_deep_value();
        let mut m = undo_manager(&a);
        for (from, to) in reorder_moves(&wanted) {
            slides.mov(from, to).unwrap();
        }
        a.commit();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            m.undo().unwrap();
            let ok = a.get_deep_value() == initial;
            m.redo().unwrap();
            ok
        }));
        match r {
            Ok(true) => {}
            Ok(false) => lost += 1,
            Err(_) => panicked += 1,
        }
    }
    std::panic::set_hook(hook);
    assert_that!((lost, panicked)).is_equal_to((32, 10));
}

/// Point 7a : A défait un déplacement après avoir reçu l'édition de méta de B : la
/// méta de B survit, x revient à sa place.
#[test]
fn p7_undo_move_keeps_remote_meta_edit_received_before() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let (p1, p2) = (t.create(s).unwrap(), t.create(s).unwrap());
    let x = element(&t, p1, "a", "hello");
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);
    let mut m = undo_manager(&a);

    t.mov(x, p2).unwrap();
    a.commit();
    sync(&a, &b);
    let mb = tb.get_meta(x).unwrap();
    mb.insert("kind", "title").unwrap();
    meta_text(&mb).insert(5, "!").unwrap();
    b.commit();
    sync(&a, &b);
    assert_that!(m.undo().unwrap()).is_true();
    a.commit();
    sync(&a, &b);

    same(&a, &b);
    assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(p1)));
    assert_that!(meta_str(&t, x, "kind")).is_equal_to(Some("title".to_owned()));
    assert_that!(meta_text(&t.get_meta(x).unwrap()).to_string()).is_equal_to("hello!".to_owned());
}

/// Point 7b : Même chose, l'édition de B étant concurrente de l'annulation.
#[test]
fn p7_undo_move_keeps_concurrent_remote_meta_edit() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let (p1, p2) = (t.create(s).unwrap(), t.create(s).unwrap());
    let x = element(&t, p1, "a", "hello");
    a.commit();
    let b = fork(&a, 2);
    let tb = tree(&b);
    let mut m = undo_manager(&a);

    t.mov(x, p2).unwrap();
    a.commit();
    sync(&a, &b);
    let mb = tb.get_meta(x).unwrap();
    mb.insert("kind", "title").unwrap();
    meta_text(&mb).insert(5, "!").unwrap();
    b.commit();
    assert_that!(m.undo().unwrap()).is_true();
    a.commit();
    sync(&a, &b);

    same(&a, &b);
    assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(p1)));
    assert_that!(meta_str(&t, x, "kind")).is_equal_to(Some("title".to_owned()));
    assert_that!(meta_text(&t.get_meta(x).unwrap()).to_string()).is_equal_to("hello!".to_owned());
}

/// Point 8: a snapshot read back keeps the tree.
///
/// `TreeID`, order, fractional indexes (of live nodes), meta and deleted nodes are
/// preserved; the fractional index stays active on the reloaded doc.
#[test]
fn p8_snapshot_round_trip_preserves_tree() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    let (p1, p2) = (t.create(s).unwrap(), t.create(s).unwrap());
    let c: Vec<TreeID> = (0..4).map(|i| element(&t, p1, "a", &format!("e{i}"))).collect();
    t.mov_to(c[3], p1, 0).unwrap();
    t.mov(c[1], p2).unwrap();
    t.delete(c[2]).unwrap();
    a.commit();

    let bytes = a.export(ExportMode::Snapshot).unwrap();
    let b = LoroDoc::from_snapshot(&bytes).unwrap();
    let tb = b.get_tree(TREE);

    same(&a, &b);
    assert_that!(tb.get_value_with_meta()).is_equal_to(t.get_value_with_meta());
    assert_that!(tb.nodes().len()).is_equal_to(t.nodes().len());
    for p in [s, p1, p2] {
        assert_that!(kids(&tb, p)).is_equal_to(kids(&t, p));
    }
    for n in t.nodes() {
        assert_that!(tb.is_node_deleted(&n).unwrap()).is_equal_to(t.is_node_deleted(&n).unwrap());
        if !t.is_node_deleted(&n).unwrap() {
            assert_that!(tb.fractional_index(n)).is_equal_to(t.fractional_index(n));
        }
    }
    // Seule différence : un nœud supprimé n'a pas de fractionnaire avant, en a un après.
    assert_that!(t.fractional_index(c[2])).is_none();
    assert_that!(tb.fractional_index(c[2])).is_some();
    assert_that!(meta_text(&tb.get_meta(c[2]).unwrap()).to_string()).is_equal_to("e2".to_owned());
    assert_that!(tb.is_fractional_index_enabled()).is_true();
}

/// Point 9 : Coût de lecture, pour le rapport seulement (rien n'est asserté sur le temps).
#[test]
fn p9_read_cost() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).unwrap();
    for i in 0..50 {
        element(&t, s, "a", &format!("élément {i}"));
    }
    a.commit();

    let n = 1_000;
    let start = Instant::now();
    for _ in 0..n {
        assert_that!(kids(&t, s).len()).is_equal_to(50);
    }
    let children_ms = start.elapsed().as_secs_f64() * 1e3 / f64::from(n);

    let start = Instant::now();
    for _ in 0..100 {
        let mut read = 0usize;
        for x in kids(&t, s) {
            let meta = t.get_meta(x).unwrap();
            read += meta_str(&t, x, "kind").map_or(0, |k| k.len());
            read += meta_text(&meta).len_unicode();
            read += kids(&t, x).len();
        }
        assert_that!(read).is_greater_than(0);
    }
    let walk_ms = start.elapsed().as_secs_f64() * 1e3 / 100.0;

    let start = Instant::now();
    for _ in 0..100 {
        assert_that!(t.get_value_with_meta()).is_not_equal_to(LoroValue::Null);
    }
    let deep_ms = start.elapsed().as_secs_f64() * 1e3 / 100.0;

    // Un deck de 40 slides de 20 éléments, un arbre par slide, parcouru en entier.
    let d = peer(2);
    let trees: Vec<(LoroTree, TreeID)> = (0..40)
        .map(|i| {
            let t = d.get_tree(format!("slide-{i}").as_str());
            let s = t.create(None).unwrap();
            for j in 0..20 {
                element(&t, s, "a", &format!("e{j}"));
            }
            (t, s)
        })
        .collect();
    d.commit();
    let start = Instant::now();
    let mut seen = 0usize;
    for (t, s) in &trees {
        for x in kids(t, *s) {
            seen += meta_text(&t.get_meta(x).unwrap()).len_unicode();
        }
    }
    let deck_ms = start.elapsed().as_secs_f64() * 1e3;
    assert_that!(seen).is_greater_than(0);

    println!(
        "coût : children(50) = {children_ms:.4} ms ; parcours méta de 50 = {walk_ms:.3} ms ; \
         get_value_with_meta(50) = {deep_ms:.3} ms ; deck 40x20 parcouru = {deck_ms:.3} ms"
    );
}
