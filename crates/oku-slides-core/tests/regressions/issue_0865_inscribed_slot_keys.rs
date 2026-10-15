//! t-865 : un `@slot` forgé de 100 000 clés visant des nœuds au bas d'une
//! chaîne de 1 000 niveaux ralentissait la lecture du deck en proportion de
//! la profondeur : chaque clé appelait `is_node_deleted`, qui la parcourt.
//! Les suppressions se lisent désormais une fois par lecture, en temps
//! linéaire : viser le bas de la chaîne ne coûte pas plus que viser sa tête.

use std::time::{Duration, Instant};

use loro::{Container, LoroDoc, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use oku_slides_core::{Deck, DeckReplica};
use speculoos::prelude::*;

use crate::common;

const LEVELS: usize = 1_000;
const KEYS: usize = 100_000;
/// Avant le correctif, ~30 (debug) ; après, ~1. Large contre la charge de la machine.
const MAX_RATIO: f64 = 4.0;

fn slide_node(tree: &LoroTree, id: &str) -> TreeID {
    tree.roots()
        .into_iter()
        .find(|n| {
            tree.get_meta(*n)
                .is_ok_and(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if *s == id))
        })
        .unwrap_or_else(|| panic!("slide {id} absente"))
}

/// `roadmap`, plus une chaîne de [`LEVELS`] niveaux sans méta, [`KEYS`] feuilles
/// au niveau `depth` de la chaîne, et sous `s-title` un élément dont le
/// `@slot` vise chaque feuille (aucun frère : le slot inscrit est lu).
fn forged(depth: usize) -> Vec<u8> {
    let deck = Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("{e}"));
    let r = DeckReplica::new(&deck, common::shared()).unwrap_or_else(|e| panic!("{e}"));
    let doc = LoroDoc::new();
    doc.import(&r.snapshot().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("arbre du deck attendu")
    };
    let mut at = TreeParentId::Node(slide_node(&tree, "s-constat"));
    let mut under = at;
    for level in 0..LEVELS {
        at = TreeParentId::Node(tree.create(at).unwrap_or_else(|e| panic!("{e}")));
        if level < depth {
            under = at;
        }
    }
    let leaves: Vec<TreeID> = (0..KEYS).map(|_| tree.create(under).unwrap_or_else(|e| panic!("{e}"))).collect();
    let element = tree.create(TreeParentId::Node(slide_node(&tree, "s-title"))).unwrap_or_else(|e| panic!("{e}"));
    let meta = tree.get_meta(element).unwrap_or_else(|e| panic!("{e}"));
    meta.insert("id", "e-forge").unwrap_or_else(|e| panic!("{e}"));
    meta.insert("kind", "text").unwrap_or_else(|e| panic!("{e}"));
    let slots = meta.ensure_mergeable_map("@slot").unwrap_or_else(|e| panic!("{e}"));
    for leaf in leaves {
        slots.insert(&leaf.to_string(), "body").unwrap_or_else(|e| panic!("{e}"));
    }
    doc.commit();
    doc.export(loro::ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"))
}

/// Le temps d'une lecture complète du deck ouvert de `bytes`, et l'élément forgé y est lu.
fn read_time(bytes: &[u8]) -> Duration {
    let (r, _) = DeckReplica::from_snapshot(bytes, 1 << 28, common::shared()).unwrap_or_else(|e| panic!("{e}"));
    let start = Instant::now();
    let deck = r.deck().0;
    let elapsed = start.elapsed();
    assert_that!(serde_json::to_string(&deck).unwrap_or_default()).contains("e-forge");
    elapsed
}

#[test]
fn deck_with_a_forged_slot_aiming_at_deep_nodes_reads_as_fast_as_aiming_at_shallow_ones() {
    let shallow = read_time(&forged(1));
    let deep = read_time(&forged(LEVELS));
    let ratio = deep.as_secs_f64() / shallow.as_secs_f64().max(1e-3);
    assert_that!(ratio).named(&format!("profond {deep:?}, en tête {shallow:?}")).is_less_than(MAX_RATIO);
}
