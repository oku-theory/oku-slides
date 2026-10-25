//! Bornes d'entrée : rejeu, arbre unique, profondeur, supprimés (t-809).

use loro::{ExportMode, LoroDoc, LoroTree, TreeParentId};
use speculoos::prelude::*;

use super::*;
use crate::error::ReplicaInner;

/// Un document dont `deck.slides` est une chaîne de `n` niveaux.
fn chain_doc(n: usize) -> (LoroDoc, LoroTree) {
    let doc = LoroDoc::new();
    let tree = doc.get_map("deck").insert_container("slides", LoroTree::new()).unwrap_or_else(|e| panic!("{e}"));
    let mut parent = TreeParentId::Root;
    for _ in 0..n {
        parent = TreeParentId::Node(tree.create(parent).unwrap_or_else(|e| panic!("{e}")));
    }
    doc.commit();
    (doc, tree)
}

fn refused<T>(r: &Result<T, ReplicaInner>, what: &str) -> bool {
    matches!(r, Err(ReplicaInner::MalformedDeckTree { what: w, .. }) if *w == what)
}

#[test]
fn check_entry_at_the_depth_bound_is_accepted() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH);
    assert_that!(check_entry(&doc, Some(&tree))).is_ok();
}

#[test]
fn check_entry_one_level_past_the_bound_is_refused() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH + 1);
    assert_that!(refused(&check_entry(&doc, Some(&tree)), TOO_DEEP)).is_true();
}

#[test]
fn check_entry_counts_nodes_deleted_by_loro() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH + 1);
    tree.delete(tree.roots()[0]).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    // Sous la racine des supprimés, la chaîne garde MAX + 1 niveaux.
    assert_that!(refused(&check_entry(&doc, Some(&tree)), TOO_DEEP)).is_true();
}

#[test]
fn check_entry_deleted_chain_at_the_bound_is_accepted_and_reported_deleted() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH);
    tree.delete(tree.roots()[0]).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let deleted = check_entry(&doc, Some(&tree)).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(deleted.len()).is_equal_to(MAX_TREE_DEPTH);
}

#[test]
fn deleted_nodes_matches_is_node_deleted_on_a_mixed_tree() {
    let (doc, tree) = chain_doc(10);
    let root = tree.roots()[0];
    let kept = tree.create(TreeParentId::Node(root)).unwrap_or_else(|e| panic!("{e}"));
    let gone = tree.create(TreeParentId::Node(root)).unwrap_or_else(|e| panic!("{e}"));
    tree.create(TreeParentId::Node(gone)).unwrap_or_else(|e| panic!("{e}"));
    tree.delete(gone).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let deleted = deleted_nodes(&tree).unwrap_or_else(|e| panic!("{e}"));
    for n in tree.nodes() {
        let loro_says = tree.is_node_deleted(&n).unwrap_or_else(|e| panic!("{e}"));
        assert_that!(deleted.contains(&n)).named(&format!("{n:?}")).is_equal_to(loro_says);
    }
    assert_that!(deleted.contains(&kept)).is_false();
    assert_that!(deleted.contains(&gone)).is_true();
}

#[test]
fn ancestry_gone_matches_is_node_deleted_on_a_mixed_tree() {
    let (doc, tree) = chain_doc(10);
    let root = tree.roots()[0];
    let gone = tree.create(TreeParentId::Node(root)).unwrap_or_else(|e| panic!("{e}"));
    tree.create(TreeParentId::Node(gone)).unwrap_or_else(|e| panic!("{e}"));
    tree.delete(gone).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let ancestry = Ancestry::new(&tree);
    for n in tree.nodes() {
        let loro_says = tree.is_node_deleted(&n).unwrap_or_else(|e| panic!("{e}"));
        assert_that!(ancestry.gone(n)).named(&format!("{n:?}")).is_equal_to(loro_says);
    }
}

#[test]
fn ancestry_gone_of_a_node_absent_from_the_tree_is_true() {
    let (_, tree) = chain_doc(1);
    let absent = loro::TreeID::new(7, 7);
    assert_that!(Ancestry::new(&tree).gone(absent)).is_true();
}

#[test]
fn ancestry_climbs_each_node_once_however_many_queries() {
    // t-865 : 100 requêtes par nœud d'une chaîne de 1000 niveaux.
    let (_, tree) = chain_doc(1000);
    let ancestry = Ancestry::new(&tree);
    let before = climbed();
    for _ in 0..100 {
        for n in tree.nodes() {
            assert_that!(ancestry.gone(n)).is_false();
        }
    }
    assert_that!(climbed() - before).is_equal_to(1000);
}

#[test]
fn check_entry_branch_past_the_bound_is_refused_whatever_the_visit_order() {
    // Un tronc de MAX - 1, deux feuilles à la borne, puis un enfant sous l'une :
    // l'une des deux remontées passe forcément par le mémo de l'autre.
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH - 1);
    let tip = tree.nodes().into_iter().max_by_key(|n| n.counter).unwrap_or_else(|| panic!("nœud"));
    let a = tree.create(TreeParentId::Node(tip)).unwrap_or_else(|e| panic!("{e}"));
    tree.create(TreeParentId::Node(tip)).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(check_entry(&doc, Some(&tree))).is_ok();
    tree.create(TreeParentId::Node(a)).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(refused(&check_entry(&doc, Some(&tree)), TOO_DEEP)).is_true();
}

#[test]
fn check_entry_with_a_second_tree_is_refused() {
    let (doc, tree) = chain_doc(1);
    doc.get_tree("autre").create(TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(refused(&check_entry(&doc, Some(&tree)), FOREIGN_TREE)).is_true();
}

#[test]
fn check_entry_without_a_deck_tree_refuses_any_tree() {
    let doc = LoroDoc::new();
    doc.get_tree("autre").create(TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(refused(&check_entry(&doc, None), FOREIGN_TREE)).is_true();
}

#[test]
fn replayable_updates_of_a_shallow_snapshot_is_refused() {
    let doc = LoroDoc::new();
    doc.get_map("deck").insert("k", 1).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let bytes = doc.export(ExportMode::shallow_snapshot(&doc.oplog_frontiers())).unwrap_or_else(|e| panic!("{e}"));
    let r = replayable_updates(&bytes);
    assert_that!(matches!(r, Err(ReplicaInner::NotADeck { what: SHALLOW, .. }))).is_true();
}

#[test]
fn replayable_updates_rebuild_the_same_state() {
    let (doc, _) = chain_doc(3);
    doc.get_map("deck").insert("titre", "x").unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
    let rebuilt = LoroDoc::new();
    rebuilt.import(&replayable_updates(&bytes).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    // Document de test, peu profond et construit ici : `get_deep_value` y est sûr.
    assert_that!(rebuilt.get_deep_value()).is_equal_to(doc.get_deep_value());
}

#[test]
fn check_depth_at_the_bound_is_accepted() {
    let (_, tree) = chain_doc(MAX_TREE_DEPTH);
    assert_that!(check_depth(&tree)).is_ok();
}

#[test]
fn check_depth_one_level_past_the_bound_is_too_deep_not_corrupted() {
    let (_, tree) = chain_doc(MAX_TREE_DEPTH + 1);
    let r = check_depth(&tree);
    assert_that!(matches!(r, Err(ReplicaInner::TreeTooDeep { max: MAX_TREE_DEPTH, .. }))).is_true();
}

#[test]
fn check_depth_counts_nodes_deleted_by_loro() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH + 1);
    tree.delete(tree.roots()[0]).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(matches!(check_depth(&tree), Err(ReplicaInner::TreeTooDeep { .. }))).is_true();
}
