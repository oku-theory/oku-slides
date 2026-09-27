//! Un snapshot vient d'un fichier : jamais fiable. `from_snapshot` doit
//! refuser proprement le tronqué, l'aléatoire, le trop gros et le document
//! Loro d'un autre projet — jamais paniquer.

use oku_slides_core::{Deck, DeckReplica, ReplicaErrorKind};
use speculoos::prelude::*;

fn snapshot() -> Vec<u8> {
    let deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json"))
        .unwrap_or_else(|e| panic!("fixture invalide : {e}"));
    let replica = DeckReplica::new(&deck).unwrap_or_else(|e| panic!("création du replica : {e}"));
    replica.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"))
}

#[test]
fn from_snapshot_truncated_returns_corrupted() {
    let s = snapshot();
    let err = DeckReplica::from_snapshot(&s[..s.len() / 2], 1 << 20).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

#[test]
fn from_snapshot_random_bytes_returns_corrupted() {
    let bytes: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
    let err = DeckReplica::from_snapshot(&bytes, 1 << 20).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

#[test]
fn from_snapshot_over_limit_returns_too_large() {
    let s = snapshot();
    let err = DeckReplica::from_snapshot(&s, s.len() - 1).unwrap_err();
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
/// l'état de chaque conteneur, tronqué, ne se décode plus. `from_snapshot`
/// fait la première lecture sous garde : `Corrupted`, jamais de panique ni
/// d'arrêt du processus (le doc empoisonné n'est pas libéré).
#[test]
fn from_snapshot_with_malformed_container_state_returns_corrupted() {
    use loro_kv_store::{MemKvStore, mem_store::MemKvConfig};
    let s = snapshot();
    let (oplog, state, shallow) = split(&s);
    let store = || {
        let mut kv = MemKvStore::new(MemKvConfig::default().should_encode_none(false));
        kv.import_all(bytes::Bytes::copy_from_slice(state)).unwrap();
        kv
    };
    let entries: Vec<_> = store().scan(std::ops::Bound::Unbounded, std::ops::Bound::Unbounded).collect();
    assert_that!(assemble(&s, oplog, state, shallow)).is_equal_to(&s);
    let mut corrupted = 0;
    for (key, value) in &entries {
        let mut kv = store();
        kv.set(key, value.slice(..value.len() / 2));
        let forged = assemble(&s, oplog, &kv.export_all(), shallow);
        let read = std::panic::catch_unwind(|| DeckReplica::from_snapshot(&forged, 1 << 20).map(|_| ()));
        match read {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                assert_that!(e.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
                corrupted += 1;
            }
            Err(_) => panic!("from_snapshot a paniqué"),
        }
    }
    // Les conteneurs du deck sont tous relus : chaque état tronqué est vu.
    assert_that!(corrupted).is_greater_than(entries.len() / 2);
}

#[test]
fn from_snapshot_of_foreign_loro_doc_returns_corrupted() {
    let doc = loro::LoroDoc::new();
    doc.get_map("autre").insert("k", 1).unwrap();
    doc.commit();
    let bytes = doc.export(loro::ExportMode::Snapshot).unwrap();
    let err = DeckReplica::from_snapshot(&bytes, 1 << 20).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}
