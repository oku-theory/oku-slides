//! An edit that would pass `MAX_OVERLAY` folds the data, drops its undo steps with a notice,
//! and is retried once, whichever path applies it.

use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_data::limits::MAX_OVERLAY;
use oku_slides_data::{CellKey, CellRead, DataCell, Rows};

use super::*;
use crate::Request;

/// Rows of the overflow tests: four columns of them fill the overlay.
const FULL_ROWS: usize = MAX_OVERLAY / 4;

/// `rows` rows of four number columns `c1`…`c4`: `4 × rows` overlay cells.
fn wide(rows: usize) -> Value {
    let rows: Vec<Value> =
        (0..rows).map(|i| json!({ "id": format!("r{i}"), "cells": [i, i + 1, i + 2, i + 3] })).collect();
    let columns: Vec<Value> =
        (1..=4).map(|c| json!({ "id": format!("c{c}"), "name": format!("v{c}"), "type": "number" })).collect();
    json!({ "generation": "g1", "columns": columns, "rows": rows })
}

fn opened(data: &Path) -> Session {
    let mut s = session_on(data);
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    s
}

fn overlay(s: &Session) -> usize {
    s.deck().data[&id("d9")].table.overlay_len()
}

/// `d9` with its overlay full and one undo step on it; the id of its last row, none of
/// whose cells is in the overlay yet.
fn full_d9(s: &mut Session) -> String {
    let add = json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": wide(FULL_ROWS) });
    let last = format!("r{FULL_ROWS}");
    let row = json!({ "op": "edit_data", "data": "d9",
        "change": { "op": "insert_row", "id": last, "after": format!("r{}", FULL_ROWS - 1) } });
    host_apply(s, vec![op(add), op(row)]);
    assert_that!(overlay(s)).is_equal_to(MAX_OVERLAY);
    // An edit of a cell already in the overlay: an undo step on d9, the overlay still full.
    apply(s, vec![set_cell("d9", "r1", 7)]);
    s.doc.notice = None;
    last
}

#[rstest]
#[case::user(false)]
#[case::agent(true)]
fn overflow_folds_and_drops_undo_with_notice(#[case] agent: bool) {
    let dir = fresh_dir("oku-slides-overflow-");
    let mut s = opened(dir.path());
    let last = full_d9(&mut s);
    let ops = vec![set_cell("d9", &last, 5)];
    let s = Mutex::new(s);
    // The user's apply carries the notice in its reply; the agent's leaves it for the next one.
    let shown = with_folding(&s, |s| {
        if agent {
            s.apply_by_agent("agent", ops.clone(), None).map(|()| s.doc.notice.clone().unwrap_or_default())
        } else {
            s.handle(Request::Apply { label: "cellule".to_owned(), ops: ops.clone(), merge: None })
        }
    })
    .unwrap_or_else(|e| panic!("overflowing edit: {}", e.one_line()));
    let mut s = s.into_inner().unwrap_or_else(PoisonError::into_inner);
    let table = &s.deck().data[&id("d9")].table;
    assert_that!(table.base.is_some()).is_true();
    assert_that!(table.overlay_len()).is_equal_to(1);
    assert_that!(table.cell(&last, "c2")).is_equal_to(CellRead::Overlay(&DataCell::Number(5.0)));
    let cut = s.localizer.tr_with("session-data-undo-cut", &[("data", "ventes".into())]);
    assert_that!(shown).contains(cut.as_str());
    // Only the retried edit is left to undo: the steps before the fold were dropped.
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    assert_that!(s.deck().data[&id("d9")].table.cell(&last, "c2")).is_equal_to(CellRead::Base);
    assert_that!(s.can_undo()).is_false();
}

#[test]
fn overflowing_apply_asks_a_fold_and_changes_nothing() {
    let dir = fresh_dir("oku-slides-overflow-ask-");
    let mut s = opened(dir.path());
    let last = full_d9(&mut s);
    let e = s.apply_folding("cellule", vec![set_cell("d9", &last, 5)], None).err();
    let e = e.unwrap_or_else(|| panic!("folded under the lock"));
    assert_that!(e.transaction().is_some_and(|t| t.op_error().is_overlay_full())).is_true();
    assert_that!(s.deck().data[&id("d9")].table.base.is_none()).is_true();
    assert_that!(s.can_undo()).is_true();
    // The fold is the caller's, to run once the session's lock is released.
    assert_that!(s.take_fold(Instant::now()).is_some()).is_true();
}

#[test]
fn fold_of_a_data_edited_meanwhile_is_dropped_and_keeps_undo() {
    let dir = fresh_dir("oku-slides-overflow-stale-");
    let mut s = opened(dir.path());
    let last = full_d9(&mut s);
    assert_that!(s.apply_folding("cellule", vec![set_cell("d9", &last, 5)], None).is_err()).is_true();
    let job = s.take_fold(Instant::now()).unwrap_or_else(|| panic!("no fold asked"));
    let reply = job.run();
    // While the lock was released, the user edited d9 again.
    apply(&mut s, vec![set_cell("d9", "r2", 9)]);
    assert_that!(s.finish_fold(job, reply, Instant::now())).is_false();
    assert_that!(s.deck().data[&id("d9")].table.base.is_none()).is_true();
    assert_that!(s.can_undo()).is_true();
}

#[test]
fn null_marker_at_overlay_cap_folds() {
    let dir = fresh_dir("oku-slides-overflow-null-");
    let mut s = opened(dir.path());
    host_apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": wide(1) }))]);
    assert_that!(fold_now(&mut s, "d9")).is_true();
    // Every row but r0, the one the base holds, fills the overlay to its cap.
    let mut deck = s.deck().clone();
    let data = deck.data.get_mut(&id("d9")).unwrap_or_else(|| panic!("no d9"));
    let table = &mut Arc::make_mut(data).table;
    table.rows = Rows::repaired((0..=FULL_ROWS).map(|r| format!("r{r}")));
    for row in 1..=FULL_ROWS {
        for column in 1..=4 {
            let key = CellKey { row: format!("r{row}"), column: format!("c{column}") };
            table.cells.insert(key, DataCell::Number(1.0));
        }
    }
    let first = table.base.clone();
    s.doc.sources.apply_silent(deck).unwrap_or_else(|e| panic!("silent: {e:?}"));
    assert_that!(overlay(&s)).is_equal_to(MAX_OVERLAY);
    // Clearing a base cell stores a null marker: one more overlay entry.
    let clear = op(json!({ "op": "edit_data", "data": "d9",
        "change": { "op": "set_cell", "row": "r0", "column": "c1", "value": null } }));
    let s = Mutex::new(s);
    with_folding(&s, |s| s.apply_folding("vider", vec![clear.clone()], None))
        .unwrap_or_else(|e| panic!("clear: {}", e.one_line()));
    let s = s.into_inner().unwrap_or_else(PoisonError::into_inner);
    let table = &s.deck().data[&id("d9")].table;
    assert_that!(table.base.is_some() && table.base != first).is_true();
    assert_that!(table.overlay_len()).is_equal_to(1);
    assert_that!(table.cell("r0", "c1")).is_equal_to(CellRead::Cleared);
    assert_that!(table.cell("r1", "c1")).is_equal_to(CellRead::Base);
}
