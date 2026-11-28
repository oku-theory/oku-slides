//! A deck at every data bound stays within its snapshot and frame bounds through 100 000
//! single edits and 10 compactions (slow group).

use oku_slides_data::limits::{MAX_DATA, MAX_DECK_OVERLAY, MAX_DECK_ROW_IDS, MAX_OVERLAY, MAX_ROWS};

use super::*;

const COLUMNS: usize = 4;

/// Data `name` of `rows` rows of four columns, its first `full` rows holding numbers.
fn bounded(name: &str, rows: usize, full: usize) -> Op {
    let rows: Vec<Value> = (0..rows)
        .map(|i| {
            if i < full {
                json!({ "id": format!("r{i}"), "cells": [i, i, i, i] })
            } else {
                json!({ "id": format!("r{i}") })
            }
        })
        .collect();
    let columns: Vec<Value> =
        (1..=COLUMNS).map(|c| json!({ "id": format!("c{c}"), "name": format!("v{c}") })).collect();
    op(json!({ "op": "add_data", "id": name, "name": name,
        "table": { "generation": "g1", "columns": columns, "rows": rows } }))
}

fn edit(data: &str, row: usize, column: usize, value: usize) -> Op {
    op(json!({ "op": "edit_data", "data": data,
        "change": { "op": "set_cell", "row": format!("r{row}"), "column": format!("c{column}"), "value": value } }))
}

/// The fixture with `MAX_DATA` data holding `MAX_DECK_ROW_IDS` row ids and `MAX_DECK_OVERLAY`
/// cells: `d0` at `MAX_OVERLAY`, `d1` with the rest, sixteen data at `MAX_ROWS` rows in all.
fn worst_case_session(data: &Path) -> Session {
    let mut settings = settings_on(data);
    settings.history.limit = 3;
    let mut s = session_with(settings);
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    let small = MAX_DATA - 16;
    let mut ops = vec![
        bounded("d0", MAX_ROWS, MAX_OVERLAY / COLUMNS),
        bounded("d1", MAX_ROWS, (MAX_DECK_OVERLAY - MAX_OVERLAY) / COLUMNS),
    ];
    ops.extend((2..15).map(|i| bounded(&format!("d{i}"), MAX_ROWS, 0)));
    ops.push(bounded("d15", MAX_ROWS - small, 0));
    ops.extend((16..MAX_DATA).map(|i| bounded(&format!("d{i}"), 1, 0)));
    host_apply(&mut s, ops);
    s
}

/// Runs `data`'s due compaction to its end; whether it committed.
fn compact(s: &mut Session, data: &str) -> bool {
    let now = Instant::now();
    let undo_touches = s.doc.sources.replica().undo_touches_data(&id(data));
    let overlay = s.deck().data[&id(data)].table().unwrap().overlay_len();
    assert_that!(s.compactions_due(now))
        .named(&format!("due (undo touches {undo_touches}, overlay {overlay})"))
        .contains(id(data));
    let job =
        s.compaction_job(&id(data), now, false).unwrap_or_else(|| panic!("{data}: no job (data worker unavailable?)"));
    let reply = job.run();
    s.finish_compaction(job, reply, now)
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release only (just test-bounds): a debug build rereads the whole deck after each edit"
)]
fn worst_case_deck_snapshot_and_frame_fit() {
    let dir = fresh_dir("oku-slides-compact-worst-");
    let mut s = worst_case_session(dir.path());
    let deck = s.deck();
    let rows: usize = deck.data.values().map(|d| d.table().unwrap().rows.len()).sum();
    let overlay: usize = deck.data.values().map(|d| d.table().unwrap().overlay_len()).sum();
    assert_that!((deck.data.len(), rows, overlay)).is_equal_to((MAX_DATA, MAX_DECK_ROW_IDS, MAX_DECK_OVERLAY));
    let mut compacted = 0;
    for round in 0..10 {
        // Each round writes COMPACT_AT cells of d0, then pushes them out of the undo window.
        for n in 0..COMPACT_AT {
            apply(&mut s, vec![edit("d0", n / COLUMNS, n % COLUMNS + 1, round * COMPACT_AT + n)]);
        }
        // Values no cell of d1 holds: an edit to the value already there makes no undo step.
        for n in 0..3 {
            apply(&mut s, vec![edit("d1", n, 1, MAX_ROWS + round * 3 + n)]);
        }
        compacted += usize::from(compact(&mut s, "d0"));
    }
    assert_that!(compacted).is_equal_to(10);
    let snapshot = s.doc.sources.replica().snapshot().unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot.len()).is_less_than_or_equal_to(64 << 20);
    // Every frame of a render: each data whole (`PutData`), then the deck without them.
    let max = crate::Settings::defaults().unwrap_or_else(|e| panic!("{e}")).render.max_frame_bytes();
    let mut deck = s.deck().clone();
    for d in std::mem::take(&mut deck.data).values() {
        let json = serde_json::to_vec(&**d).unwrap_or_else(|e| panic!("{e}"));
        assert_that!(json.len()).is_less_than(max);
    }
    let json = serde_json::to_vec(&deck).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(json.len()).is_less_than(max);
}
