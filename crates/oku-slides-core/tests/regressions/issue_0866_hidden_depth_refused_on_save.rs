//! t-866 : un sous-arbre retiré (marqué) garde sa profondeur et suit son
//! ancêtre vivant quand il descend ; seule la profondeur des nœuds vivants
//! était bornée à l'écriture. Retraits et déplacements vers le bas
//! empilaient ainsi des niveaux cachés au-delà de 1024 : le deck
//! s'enregistrait, puis sa réouverture était refusée (`Corrupted`).
//! L'enregistrement est désormais refusé (`TooDeep`) ; défaire le
//! déplacement le rend de nouveau possible, et rien de vivant n'est perdu.

use loro::{Container, LoroDoc, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use oku_slides_core::{Deck, DeckReplica, ReplicaErrorKind};
use speculoos::prelude::*;

use crate::common;

/// Profondeur maximale relue (`replica::entry::MAX_TREE_DEPTH`).
const MAX_TREE_DEPTH: usize = 1024;
const BIG: usize = 64 << 20;

fn on_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(BIG)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique"))
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("{e}"))
}

fn node_with_id(tree: &LoroTree, id: &str) -> TreeID {
    tree.nodes()
        .into_iter()
        .find(|n| {
            tree.get_meta(*n)
                .is_ok_and(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if *s == id))
        })
        .unwrap_or_else(|| panic!("nœud {id} absent"))
}

/// `roadmap`, où l'élément `b-points` (profondeur 2) porte un élément retiré
/// au-dessus d'une chaîne cachée qui finit à la profondeur `bottom`.
fn hidden_chain_to(bottom: usize) -> Vec<u8> {
    let r = DeckReplica::new(&roadmap(), common::shared()).unwrap_or_else(|e| panic!("{e}"));
    let doc = LoroDoc::new();
    doc.import(&r.snapshot().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("arbre du deck attendu")
    };
    let mut at = TreeParentId::Node(node_with_id(&tree, "b-points"));
    for level in 3..=bottom {
        let node = tree.create(at).unwrap_or_else(|e| panic!("{e}"));
        if level == 3 {
            let meta = tree.get_meta(node).unwrap_or_else(|e| panic!("{e}"));
            meta.insert("id", "e-retire").unwrap_or_else(|e| panic!("{e}"));
            meta.insert("kind", "text").unwrap_or_else(|e| panic!("{e}"));
            meta.insert("@deleted/1", 1_i64).unwrap_or_else(|e| panic!("{e}"));
        }
        at = TreeParentId::Node(node);
    }
    doc.commit();
    let bytes = doc.export(loro::ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
    std::mem::forget(doc);
    bytes
}

/// `deck`, où `b-points` descend d'un niveau, dans une pile `k-boite` de `s-constat`.
fn moved_lower(deck: &Deck) -> Deck {
    let mut json = serde_json::to_value(deck).unwrap_or_else(|e| panic!("{e}"));
    let slides = json["slides"].as_array_mut().unwrap_or_else(|| panic!("slides"));
    let constat = slides.iter_mut().find(|s| s["id"] == "s-constat").unwrap_or_else(|| panic!("s-constat"));
    let points = constat["params"]["blocks"][0].take();
    constat["params"]["blocks"] =
        serde_json::json!([{ "id": "k-boite", "kind": "stack", "params": { "children": [points] } }]);
    Deck::from_json(&json.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

fn open(bytes: &[u8]) -> (DeckReplica, Deck) {
    let (r, (deck, _)) = DeckReplica::from_snapshot(bytes, 1 << 26, common::shared()).unwrap_or_else(|e| panic!("{e}"));
    (r, deck)
}

#[test]
fn snapshot_after_moving_a_hidden_subtree_past_the_bound_is_refused_then_saves_once_undone() {
    on_stack(|| {
        let saved = hidden_chain_to(MAX_TREE_DEPTH);
        let (mut r, deck) = open(&saved);
        r.apply(&moved_lower(&deck), "Déplacer", None).unwrap_or_else(|e| panic!("{e}"));
        assert_that!(r.snapshot().map_err(|e| e.kind())).is_err_containing(ReplicaErrorKind::TooDeep);
        // Le fichier déjà enregistré se rouvre, intact.
        assert_that!(open(&saved).1).is_equal_to(&deck);
        // Défaire le déplacement suffit à enregistrer de nouveau.
        assert_that!(r.undo().unwrap_or_else(|e| panic!("{e}"))).is_true();
        let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
        assert_that!(open(&bytes).1).is_equal_to(&deck);
    });
}

#[test]
fn snapshot_after_moving_a_hidden_subtree_down_to_the_bound_saves_and_reopens() {
    // Témoin : un niveau de moins, la même descente finit à la borne.
    on_stack(|| {
        let (mut r, deck) = open(&hidden_chain_to(MAX_TREE_DEPTH - 1));
        let moved = moved_lower(&deck);
        r.apply(&moved, "Déplacer", None).unwrap_or_else(|e| panic!("{e}"));
        let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
        assert_that!(open(&bytes).1).is_equal_to(&moved);
    });
}
