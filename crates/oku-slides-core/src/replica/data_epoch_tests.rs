//! Layers per epoch: a compaction writes a new `layer` container, and merges report edits
//! made to a replaced one.

use super::*;

/// `deck` with data `id` compacted into `epoch`: a base recorded, its overlay folded away.
fn compacted(deck: &Deck, id: &str, epoch: &str) -> Deck {
    edited(deck, id, |d| {
        d.table_mut().unwrap().epoch = epoch.to_owned();
        d.table_mut().unwrap().base = Some(base());
        d.table_mut().unwrap().cells.clear();
    })
}

fn layer_map(r: &DeckReplica, id: &str) -> LoroMap {
    child_map(&table_map(r, id), LAYER)
}

#[test]
fn new_epoch_writes_a_new_layer_container_and_reads_back() {
    let deck = sample();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let before = layer_map(&r, "d1").id();
    let table = table_map(&r, "d1").id();
    let after = compacted(&deck, "d1", "e2");
    r.apply_silent(&after, crate::KnowledgeSync::Changed).unwrap();
    assert_that!(layer_map(&r, "d1").id()).is_not_equal_to(before);
    assert_that!(table_map(&r, "d1").id()).is_equal_to(table);
    assert_that!(table_of(&r.deck().0, "d1").epoch.as_str()).is_equal_to("e2");
    // Bases are resolved by the session: the replica reads one as missing (read-only).
    assert_that!(reopened(&r).0.data).is_equal_to(after.data);
}

#[test]
fn edit_within_one_epoch_keeps_the_layer_container() {
    let deck = compacted(&sample(), "d1", "e2");
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let layer = layer_map(&r, "d1").id();
    r.apply(
        &edited(&deck, "d1", |d| set_cell(d.table_mut().unwrap(), "r1", "c1", DataCell::Text("A".to_owned()))),
        "A",
        None,
    )
    .unwrap();
    assert_that!(layer_map(&r, "d1").id()).is_equal_to(layer);
    assert_that!(cells_map(&r, "d1").len()).is_equal_to(1);
}

#[test]
fn edit_on_replaced_epoch_is_lost_reported_on_both_peers_and_docs_equal() {
    let deck = sample();
    let db = edited(&deck, "d1", |d| set_cell(d.table_mut().unwrap(), "r2", "c2", DataCell::Number(5.0)));
    let (mut a, mut b) = peers(&deck);
    a.apply_silent(&compacted(&deck, "d1", "e2"), crate::KnowledgeSync::Changed).unwrap();
    b.apply(&db, "B", None).unwrap();
    let ra = a.merge(&b).unwrap();
    let rb = b.merge(&a).unwrap();
    assert_that!(ra.lost).is_equal_to(vec![did("d1")]);
    assert_that!(rb.lost).is_equal_to(vec![did("d1")]);
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
    assert_that!(table_of(&a.deck().0, "d1").epoch.as_str()).is_equal_to("e2");
}

#[test]
fn compaction_without_a_concurrent_edit_loses_nothing() {
    let deck = sample();
    let (mut a, mut b) = peers(&deck);
    a.apply_silent(&compacted(&deck, "d1", "e2"), crate::KnowledgeSync::Changed).unwrap();
    assert_that!(b.merge(&a).unwrap().lost).is_empty();
    assert_that!(a.merge(&b).unwrap().lost).is_empty();
}

/// A copy of `r` from its snapshot: its state before a merge, for the other peer.
fn copy(r: &DeckReplica) -> DeckReplica {
    DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 26, registry()).unwrap().0
}

#[test]
fn two_copies_compacting_converge_and_report() {
    let deck = sample();
    let (mut a, mut b) = peers(&deck);
    a.apply_silent(&compacted(&deck, "d1", "e2"), crate::KnowledgeSync::Changed).unwrap();
    b.apply_silent(&compacted(&deck, "d1", "e3"), crate::KnowledgeSync::Changed).unwrap();
    let (a0, b0) = (copy(&a), copy(&b));
    let ra = a.merge(&b0).unwrap();
    let rb = b.merge(&a0).unwrap();
    // One layer wins; the other's creation is reported lost on both peers, never merged.
    assert_that!(ra.lost).is_equal_to(vec![did("d1")]);
    assert_that!(rb.lost).is_equal_to(vec![did("d1")]);
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
    let table = table_of(&a.deck().0, "d1").clone();
    assert_that!(["e2", "e3"].contains(&table.epoch.as_str())).is_true();
    assert_that!(table.base).is_equal_to(Some(base()));
    assert_that!(table.overlay_len()).is_equal_to(0);
}

#[test]
fn compaction_of_truncated_data_runs() {
    let (deck, cut, _) = over_bound("cell");
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let (read, _) = r.deck();
    assert_that!(read.truncated.contains(&did(cut))).is_true();
    let from = table_of(&read, cut).clone();
    assert_that!(r.compact_data(&did(cut), &from, "e2".to_owned(), base()).unwrap()).is_true();
    let (got, issues) = reopened(&r);
    assert_that!(got.truncated.contains(&did(cut))).is_false();
    assert_that!(keys(&issues).contains(&"issue-data-truncated")).is_false();
    let table = table_of(&got, cut);
    assert_that!(table.epoch.as_str()).is_equal_to("e2");
    assert_that!(table.base).is_equal_to(Some(base()));
    assert_that!(table.overlay_len()).is_equal_to(0);
}
