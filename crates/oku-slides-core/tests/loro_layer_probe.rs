//! Loro 1.17.1 characterization probe for t-1690 (Task 4f, Step 0): a data
//! table's `layer` becomes one child container per epoch, replaced by a
//! silent compaction commit. These tests pin the library behavior the
//! replica relies on, not the product behavior.
//!
//! - (a) Undo of a local step stays exact after a silent commit that
//!   `insert_container`s a new layer under the same key.
//! - (b) `UndoManager` has no API to drop selected items: only `clear`, which
//!   empties both stacks (the replica's `drop_data` therefore cuts).
//! - (c) A concurrent edit into a replaced layer is in the merged version
//!   vector but not under the current key.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use loro::{ExportMode, LoroDoc, LoroMap, LoroValue, UndoManager, ValueOrContainer};
use speculoos::prelude::*;

const SILENT: &str = "oku-silent";

/// `unwrap` outside test functions, refused by clippy.
trait Must<T> {
    fn must(self) -> T;
}

impl<T, E: std::fmt::Display> Must<T> for Result<T, E> {
    fn must(self) -> T {
        self.unwrap_or_else(|e| panic!("loro: {e}"))
    }
}

/// A deterministic peer, set up like `replica::configure`.
fn peer(id: u64) -> LoroDoc {
    let doc = LoroDoc::new();
    doc.set_peer_id(id).must();
    doc.set_record_timestamp(false);
    doc.set_change_merge_interval(-1);
    doc
}

fn fork(a: &LoroDoc, id: u64) -> LoroDoc {
    let b = peer(id);
    b.import(&a.export(ExportMode::Snapshot).must()).must();
    b
}

fn undo_of(doc: &LoroDoc) -> UndoManager {
    let mut m = UndoManager::new(doc);
    m.set_merge_interval(0);
    m.add_exclude_origin_prefix(SILENT);
    m
}

fn table(doc: &LoroDoc) -> LoroMap {
    match doc.get_map("data").get("d1") {
        Some(ValueOrContainer::Container(loro::Container::Map(m))) => m,
        _ => doc.get_map("data").insert_container("d1", LoroMap::new()).must(),
    }
}

/// The current layer of the table.
fn layer(doc: &LoroDoc) -> LoroMap {
    match table(doc).get("layer") {
        Some(ValueOrContainer::Container(loro::Container::Map(m))) => m,
        other => panic!("no layer: {other:?}"),
    }
}

fn cells(doc: &LoroDoc) -> LoroMap {
    match layer(doc).get("cells") {
        Some(ValueOrContainer::Container(loro::Container::Map(m))) => m,
        other => panic!("no cells: {other:?}"),
    }
}

/// A new layer under `layer`, epoch `epoch`, `base` set, empty cells.
fn new_layer(doc: &LoroDoc, epoch: i64, base: &str) {
    let l = table(doc).insert_container("layer", LoroMap::new()).must();
    l.insert("epoch", epoch).must();
    l.insert("base", base).must();
    l.insert_container("cells", LoroMap::new()).must();
}

fn cell(doc: &LoroDoc, key: &str) -> Option<LoroValue> {
    match cells(doc).get(key) {
        Some(ValueOrContainer::Value(v)) => Some(v),
        _ => None,
    }
}

fn seeded(id: u64) -> LoroDoc {
    let doc = peer(id);
    new_layer(&doc, 1, "");
    doc.set_next_commit_origin(SILENT);
    doc.commit();
    doc
}

#[test]
fn undo_of_local_step_stays_exact_after_silent_layer_replacement() {
    let doc = seeded(1);
    let mut undo = undo_of(&doc);
    cells(&doc).insert("r1:c1", "a").must();
    doc.commit();
    cells(&doc).insert("r2:c1", "b").must();
    doc.commit();
    // Silent compaction: the cells move into a "base", the layer is replaced.
    new_layer(&doc, 2, "sha256:x");
    doc.set_next_commit_origin(SILENT);
    doc.commit();
    assert_that!(undo.undo_count()).is_equal_to(2);
    assert_that!(cells(&doc).len()).is_equal_to(0);

    undo.undo().must();
    // Undo of an edit into the replaced layer does not touch the current one.
    assert_that!(cells(&doc).len()).is_equal_to(0);
    assert_that!(layer(&doc).get("epoch").and_then(|v| v.into_value().ok())).is_equal_to(Some(LoroValue::I64(2)));
}

#[test]
fn undo_after_silent_layer_replacement_undoes_new_layer_edits_exactly() {
    let doc = seeded(1);
    let mut undo = undo_of(&doc);
    new_layer(&doc, 2, "sha256:x");
    doc.set_next_commit_origin(SILENT);
    doc.commit();
    cells(&doc).insert("r1:c1", "a").must();
    doc.commit();
    cells(&doc).insert("r1:c1", "b").must();
    doc.commit();
    undo.undo().must();
    assert_that!(cell(&doc, "r1:c1")).is_equal_to(Some(LoroValue::from("a")));
    undo.undo().must();
    assert_that!(cell(&doc, "r1:c1")).is_none();
    undo.redo().must();
    assert_that!(cell(&doc, "r1:c1")).is_equal_to(Some(LoroValue::from("a")));
}

#[test]
fn undo_manager_has_only_clear_to_drop_items() {
    let doc = seeded(1);
    let mut undo = undo_of(&doc);
    cells(&doc).insert("r1:c1", "a").must();
    doc.commit();
    doc.get_map("other").insert("k", 1).must();
    doc.commit();
    undo.undo().must();
    assert_that!(undo.undo_count()).is_equal_to(1);
    assert_that!(undo.redo_count()).is_equal_to(1);
    // No method removes one item: `clear` is the only way, and it empties both stacks.
    undo.clear();
    assert_that!(undo.undo_count()).is_equal_to(0);
    assert_that!(undo.redo_count()).is_equal_to(0);
}

#[test]
fn concurrent_edit_into_replaced_layer_is_in_version_but_not_current() {
    let a = seeded(1);
    let b = fork(&a, 2);
    cells(&b).insert("r1:c1", "lost").must();
    b.commit();
    new_layer(&a, 2, "sha256:x");
    a.set_next_commit_origin(SILENT);
    a.commit();

    a.import(&b.export(ExportMode::all_updates()).must()).must();
    b.import(&a.export(ExportMode::all_updates()).must()).must();

    assert_that!(a.oplog_vv().get(&2).copied()).is_some();
    assert_that!(cell(&a, "r1:c1")).is_none();
    assert_that!(cell(&b, "r1:c1")).is_none();
    assert_that!(layer(&b).get("epoch").and_then(|v| v.into_value().ok())).is_equal_to(Some(LoroValue::I64(2)));
    assert_that!(a.get_deep_value()).is_equal_to(b.get_deep_value());
}
