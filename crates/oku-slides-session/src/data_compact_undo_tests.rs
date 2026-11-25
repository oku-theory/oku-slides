//! Undo and redo stay exact around compactions: a compaction never lands under a step that
//! writes into its data, and never disturbs the steps of another data.

use oku_slides_data::{CellRead, DataCell, Table};

use super::*;

fn undo(s: &mut Session) {
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
}

fn redo(s: &mut Session) {
    s.redo().unwrap_or_else(|e| panic!("redo: {}", e.one_line()));
}

#[test]
fn a_b_a_on_one_key_then_two_undos() {
    let dir = fresh_dir("oku-slides-compact-aba-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    assert_that!(compact_now(&mut s)).is_true();
    apply(&mut s, vec![set_cell("d9", "r1", 7)]);
    let a = cells(&s, "d9");
    apply(&mut s, vec![set_cell("d9", "r1", 9)]);
    let b = cells(&s, "d9");
    apply(&mut s, vec![set_cell("d9", "r1", 7)]);
    assert_that!(cells(&s, "d9")).is_equal_to(&a);
    undo(&mut s);
    assert_that!(cells(&s, "d9")).is_equal_to(&b);
    undo(&mut s);
    assert_that!(cells(&s, "d9")).is_equal_to(&a);
}

#[test]
fn undo_across_full_window_after_unrelated_compaction() {
    const WINDOW: usize = 5;
    let dir = fresh_dir("oku-slides-compact-window-");
    let mut settings = settings_on(dir.path());
    settings.history.limit = WINDOW;
    let mut s = with_d9(session_with(settings), COMPACT_AT / 2);
    apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d8", "name": "autre", "table": table(WINDOW + 1) }))]);
    let mut states = Vec::new();
    for (row, value) in (0..WINDOW).zip(40..) {
        states.push(cells(&s, "d8"));
        apply(&mut s, vec![set_cell("d8", &format!("r{row}"), value)]);
    }
    assert_that!(compact_now(&mut s)).is_true();
    for state in states.iter().rev() {
        undo(&mut s);
        assert_that!(cells(&s, "d8")).is_equal_to(state);
    }
    // The window held only the five edits: the creation of d8 fell out of it.
    assert_that!(s.can_undo()).is_false();
    assert_that!(s.deck().data[&id("d9")].table.base.is_some()).is_true();
}

#[test]
fn redo_after_compaction_of_other_data() {
    let dir = fresh_dir("oku-slides-compact-redo-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d8", "name": "autre", "table": table(2) }))]);
    apply(&mut s, vec![set_cell("d8", "r1", 40)]);
    let edited = cells(&s, "d8");
    undo(&mut s);
    assert_that!(compact_now(&mut s)).is_true();
    redo(&mut s);
    assert_that!(cells(&s, "d8")).is_equal_to(edited);
    assert_that!(s.deck().data[&id("d9")].table.base.is_some()).is_true();
}

#[test]
fn undo_of_replace_whose_base_is_kept() {
    let dir = fresh_dir("oku-slides-compact-replace-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    assert_that!(compact_now(&mut s)).is_true();
    let before = cells(&s, "d9");
    let base = s.deck().data[&id("d9")].table.base.clone().unwrap_or_else(|| panic!("no base"));
    let replace = json!({ "op": "edit_data", "data": "d9", "change": { "op": "replace", "table": table(3) } });
    apply(&mut s, vec![op(replace)]);
    assert_that!(s.deck().data[&id("d9")].table.base.is_none()).is_true();
    undo(&mut s);
    assert_that!(s.deck().data[&id("d9")].table.base.as_ref()).is_equal_to(Some(&base));
    assert_that!(s.doc.sources.bases.contains(&base.id)).is_true();
    assert_that!(cells(&s, "d9")).is_equal_to(before);
}

/// `rows` rows of three number columns `c1`…`c3`: `3 × rows` cells.
fn three(rows: usize) -> Value {
    let rows: Vec<Value> = (0..rows).map(|i| json!({ "id": format!("r{i}"), "cells": [i, i * 2, i * 3] })).collect();
    let columns: Vec<Value> = (1..=3).map(|c| json!({ "id": format!("c{c}"), "name": format!("v{c}") })).collect();
    json!({ "generation": "g1", "columns": columns, "rows": rows })
}

#[test]
fn single_150000_cell_add_data_then_100_edits_stays_exact() {
    let dir = fresh_dir("oku-slides-compact-150000-");
    let mut s = session_on(dir.path());
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    host_apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": three(50_000) }))]);
    let table = |s: &Session| -> Table { s.deck().data[&id("d9")].table.clone() };
    assert_that!(table(&s).overlay_len()).is_equal_to(150_000);
    let mut states = vec![table(&s)];
    for (row, value) in (0..100).zip((1..=100).map(|v: i64| -v)) {
        apply(&mut s, vec![set_cell("d9", &format!("r{row}"), value)]);
        states.push(table(&s));
        // The data is over COMPACT_AT, but every step writes into it: no compaction.
        assert_that!(s.compactions_due(Instant::now())).is_empty();
    }
    assert_that!(table(&s).cell("r99", "c2")).is_equal_to(CellRead::Overlay(&DataCell::Number(-100.0)));
    states.pop();
    while let Some(state) = states.pop() {
        undo(&mut s);
        match s.deck().data.get(&id("d9")) {
            Some(d) => assert_that!(d.table).is_equal_to(&state),
            None => panic!("d9 gone after {} undos", 100 - states.len()),
        }
    }
    assert_that!(table(&s).cell("r0", "c2")).is_equal_to(CellRead::Overlay(&DataCell::Number(0.0)));
    // The creation is still an undo step: no compaction; undone, the data goes.
    assert_that!(s.compactions_due(Instant::now())).is_empty();
    undo(&mut s);
    assert_that!(s.deck().data.contains_key(&id("d9"))).is_false();
}
