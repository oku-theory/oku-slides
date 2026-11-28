//! t-1690: an edit refused by the deck's overlay cap folded the largest touched data even
//! when no single fold could make room for the edit. The fold cut every undo step, then the
//! retry failed anyway. A refusal no fold can cure is now returned as is, the undo kept.

use oku_slides_core::{Op, OpErrorKind};
use oku_slides_session::{Request, Workspace};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{data_dir, fixture_path};
use crate::workspace_support::{on, op, workspace};

/// Rows of each of the five big data: two number columns, 59,990 overlay cells.
const BIG_ROWS: usize = 29_995;
/// Rows the overflowing edit inserts into `d9`: 62,000 cells, under the batch cap.
const NEW_ROWS: usize = 31_000;

fn table(rows: usize) -> Value {
    let rows: Vec<Value> = (0..rows).map(|i| json!({ "id": format!("r{i}"), "cells": [i, i] })).collect();
    json!({ "columns": [{ "id": "c1", "name": "a", "type": "number" }, { "id": "c2", "name": "b", "type": "number" }],
        "rows": rows })
}

fn add(id: &str, rows: usize) -> Op {
    op(json!({ "op": "add_data", "id": id, "name": id, "table": table(rows) }))
}

#[test]
fn issue_1690_deck_overlay_refusal_no_fold_can_cure_keeps_undo() {
    let data = data_dir();
    let ws = workspace(&data);
    let tab = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();
    on(&ws, &tab, |s| {
        s.open(&fixture_path())?;
        for i in 1..=5 {
            s.apply("données", vec![add(&format!("d{i}"), BIG_ROWS)], None)?;
        }
        s.apply("données", vec![add("d9", 2)], None)
    })
    .unwrap_or_else(|e| panic!("setup: {}", e.one_line()));
    let ops: Vec<Op> = (0..NEW_ROWS)
        .map(|i| {
            op(json!({ "op": "edit_data", "data": "d9",
                "change": { "op": "insert_row", "id": format!("n{i}"), "cells": { "c1": i, "c2": i } } }))
        })
        .collect();

    let (reply, _) = Workspace::request(&ws, &tab, Request::Apply { label: "lignes".to_owned(), ops, merge: None });

    let e = reply.err().unwrap_or_else(|| panic!("the edit passed the deck's overlay cap"));
    let kind = e.transaction().map(|t| t.op_error().kind());
    assert_that!(kind).is_equal_to(Some(OpErrorKind::DataDeckFull));
    let (based, can_undo) =
        on(&ws, &tab, |s| (s.deck().data.values().any(|d| d.table().is_some_and(|t| t.base.is_some())), s.can_undo()));
    assert_that!(based).is_false();
    assert_that!(can_undo).is_true();
}
