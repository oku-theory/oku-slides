//! Un snapshot vient d'un fichier : jamais fiable. `from_snapshot` doit
//! refuser proprement le tronqué, l'aléatoire, le trop gros et le document
//! Loro d'un autre projet — jamais paniquer.

mod common;

use loro::{Container, ExportMode, LoroDoc, LoroMap, LoroTree, LoroValue, TreeParentId, ValueOrContainer};
use oku_slides_core::{Deck, DeckReplica, ReplicaErrorKind};
use speculoos::prelude::*;

fn snapshot() -> Vec<u8> {
    let deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json"))
        .unwrap_or_else(|e| panic!("fixture invalide : {e}"));
    let replica = DeckReplica::new(&deck, common::shared()).unwrap_or_else(|e| panic!("création du replica : {e}"));
    replica.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"))
}

#[test]
fn from_snapshot_truncated_returns_corrupted() {
    let s = snapshot();
    let err = DeckReplica::from_snapshot(&s[..s.len() / 2], 1 << 20, common::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

#[test]
fn from_snapshot_random_bytes_returns_corrupted() {
    let bytes: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
    let err = DeckReplica::from_snapshot(&bytes, 1 << 20, common::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

#[test]
fn from_snapshot_over_limit_returns_too_large() {
    let s = snapshot();
    let err = DeckReplica::from_snapshot(&s, s.len() - 1, common::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::TooLarge);
}

/// Découpe un snapshot Loro (enveloppe de 22 octets, puis `u32` + oplog,
/// `u32` + état, `u32` + racine superficielle).
fn split(s: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let len = |at: usize| u32::from_le_bytes([s[at], s[at + 1], s[at + 2], s[at + 3]]) as usize;
    let oplog_at = 22 + 4;
    let state_at = oplog_at + len(22) + 4;
    let shallow_at = state_at + len(state_at - 4) + 4;
    (&s[oplog_at..state_at - 4], &s[state_at..shallow_at - 4], &s[shallow_at..])
}

/// Réassemble un snapshot et recalcule sa somme de contrôle : Loro l'accepte
/// à l'import, quel que soit l'état qu'il porte.
fn assemble(header: &[u8], oplog: &[u8], state: &[u8], shallow: &[u8]) -> Vec<u8> {
    let mut b = header[..22].to_vec();
    for part in [oplog, state, shallow] {
        let len = u32::try_from(part.len()).unwrap_or_else(|e| panic!("partie trop longue : {e}"));
        b.extend(len.to_le_bytes());
        b.extend(part);
    }
    let sum = xxhash_rust::xxh32::xxh32(&b[20..], u32::from_le_bytes(*b"LORO"));
    b[16..20].copy_from_slice(&sum.to_le_bytes());
    b
}

/// Loro ne décode l'état d'un conteneur qu'au premier accès, et panique
/// alors sur un état malformé (`decode_value(..).unwrap()`). Ces snapshots
/// passent l'import (sommes de contrôle recalculées, par bloc et globale) ;
/// l'état de chaque conteneur, tronqué, ne se décode plus. Depuis t-809,
/// `from_snapshot` ne lit jamais l'état reçu : il rejoue les opérations, et
/// chaque snapshot forgé relit le deck d'origine, sans panique.
#[test]
fn from_snapshot_with_malformed_container_state_replays_the_operations() {
    use loro_kv_store::{MemKvStore, mem_store::MemKvConfig};
    let s = snapshot();
    let expected = DeckReplica::from_snapshot(&s, 1 << 20, common::shared()).unwrap_or_else(|e| panic!("{e}")).1;
    let (oplog, state, shallow) = split(&s);
    let store = || {
        let mut kv = MemKvStore::new(MemKvConfig::default().should_encode_none(false));
        kv.import_all(bytes::Bytes::copy_from_slice(state)).unwrap();
        kv
    };
    let entries: Vec<_> = store().scan(std::ops::Bound::Unbounded, std::ops::Bound::Unbounded).collect();
    assert_that!(assemble(&s, oplog, state, shallow)).is_equal_to(&s);
    assert_that!(entries.len()).is_greater_than(1);
    for (key, value) in &entries {
        let mut kv = store();
        kv.set(key, value.slice(..value.len() / 2));
        let forged = assemble(&s, oplog, &kv.export_all(), shallow);
        assert_that!(forged).is_not_equal_to(&s);
        let read =
            std::panic::catch_unwind(|| DeckReplica::from_snapshot(&forged, 1 << 20, common::shared()).map(|r| r.1));
        match read {
            Ok(Ok(read)) => assert_that!(read).is_equal_to(&expected),
            Ok(Err(e)) => panic!("état forgé lu : {e}"),
            Err(_) => panic!("from_snapshot a paniqué"),
        }
    }
}

#[test]
fn from_snapshot_of_foreign_loro_doc_returns_corrupted() {
    let doc = loro::LoroDoc::new();
    doc.get_map("autre").insert("k", 1).unwrap();
    doc.commit();
    let bytes = doc.export(loro::ExportMode::Snapshot).unwrap();
    let err = DeckReplica::from_snapshot(&bytes, 1 << 20, common::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

/// Vrai si la méta porte l'`id` `id`.
fn has_id(m: &LoroMap, id: &str) -> bool {
    matches!(m.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if *s == id)
}

/// Le snapshot de `roadmap` après `edit` sur l'arbre du deck, sous le nœud de `slide`.
fn forged(slide: &str, edit: impl FnOnce(&LoroTree, TreeParentId)) -> Vec<u8> {
    let doc = LoroDoc::new();
    doc.import(&snapshot()).unwrap_or_else(|e| panic!("import : {e}"));
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("arbre du deck attendu")
    };
    let node = tree
        .children(TreeParentId::Root)
        .unwrap_or_default()
        .into_iter()
        .find(|n| tree.get_meta(*n).is_ok_and(|m| has_id(&m, slide)))
        .unwrap_or_else(|| panic!("slide {slide} absente"));
    edit(&tree, TreeParentId::Node(node));
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("export : {e}"))
}

/// Relit `bytes` dans un fil à pile de 256 Kio : le deck et les ids des `Issue`.
fn read_on_a_small_stack(bytes: Vec<u8>) -> (Deck, Vec<&'static str>) {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let (_, (deck, issues)) = DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared())
                .unwrap_or_else(|e| panic!("from_snapshot : {e}"));
            (deck, issues.iter().map(|i| i.message.id()).collect())
        })
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("la lecture a paniqué ou débordé la pile"))
}

/// Nombre de clés forgées dans une méta.
const HOSTILE_KEYS: u64 = 100_000;

/// Garde (sans mesure de temps) : un nœud porte 100 000 marques bien
/// formées. L'import les contrôle toutes, la lecture cache le nœud.
#[test]
fn from_snapshot_with_many_marks_on_one_node_hides_it() {
    let bytes = forged("s-constat", |tree, slide| {
        let points = tree
            .children(slide)
            .unwrap_or_default()
            .into_iter()
            .find(|n| tree.get_meta(*n).is_ok_and(|m| has_id(&m, "b-points")))
            .unwrap_or_else(|| panic!("b-points absent"));
        let meta = tree.get_meta(points).unwrap_or_else(|e| panic!("méta : {e}"));
        for peer in 0..HOSTILE_KEYS {
            meta.insert(&format!("@deleted/{peer}"), 1_700_000_000_i64).unwrap_or_else(|e| panic!("marque : {e}"));
        }
    });
    let (deck, issues) = read_on_a_small_stack(bytes);
    assert_that!(issues).is_empty();
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    assert_that!(slide.locate("b-points").is_none()).is_true();
}

/// Garde de coût (sans mesure de temps) : un `@slot` porte 100 000 entrées,
/// aucune pour le parent réel, toutes pour des nœuds absents et un même slot.
/// La règle 3 s'applique une fois (3a, sans `Issue`).
#[test]
fn from_snapshot_with_a_huge_slot_map_reads() {
    let bytes = forged("s-title", |tree, slide| {
        let node = tree.create(slide).unwrap_or_else(|e| panic!("nœud : {e}"));
        let meta = tree.get_meta(node).unwrap_or_else(|e| panic!("méta : {e}"));
        meta.insert("id", "x").unwrap_or_else(|e| panic!("id : {e}"));
        meta.insert("kind", "text").unwrap_or_else(|e| panic!("kind : {e}"));
        let slot = meta.ensure_mergeable_map("@slot").unwrap_or_else(|e| panic!("@slot : {e}"));
        for i in 0..HOSTILE_KEYS {
            slot.insert(&format!("{i}@1"), "blocks").unwrap_or_else(|e| panic!("entrée : {e}"));
        }
    });
    let (deck, issues) = read_on_a_small_stack(bytes);
    assert_that!(issues).is_empty();
    let slide = deck.slide("s-title").unwrap_or_else(|| panic!("s-title absente"));
    let ids: Vec<&str> = slide.params.slot("blocks").unwrap_or_default().iter().map(|e| e.id.as_str()).collect();
    assert_that!(ids).is_equal_to(vec!["x"]);
}

/// t-770 : une marque qui n'est pas une date est refusée à l'entrée.
#[test]
fn from_snapshot_with_a_non_integer_mark_returns_corrupted() {
    for value in [LoroValue::Bool(true), LoroValue::from("hier"), LoroValue::Double(1.5)] {
        let forged_value = value.clone();
        let bytes = forged("s-constat", move |tree, slide| {
            let points = tree
                .children(slide)
                .unwrap_or_default()
                .into_iter()
                .find(|n| tree.get_meta(*n).is_ok_and(|m| has_id(&m, "b-points")))
                .unwrap_or_else(|| panic!("b-points absent"));
            let meta = tree.get_meta(points).unwrap_or_else(|e| panic!("méta : {e}"));
            meta.insert("@deleted/7", forged_value).unwrap_or_else(|e| panic!("marque : {e}"));
        });
        let kind = DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared()).err().map(|e| e.kind());
        assert_that!(kind).named(&format!("{value:?}")).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    }
}
