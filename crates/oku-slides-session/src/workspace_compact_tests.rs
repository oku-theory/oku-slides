use std::path::Path;

use oku_slides_core::Op;
use oku_slides_data::DataId;
use oku_slides_data::limits::COMPACT_AT;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::Workspace;
use crate::test_support::{fresh_dir, workspace_on};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

/// `rows` rows of two number columns: `2 × rows` overlay cells.
fn table(rows: usize) -> Value {
    let rows: Vec<Value> = (0..rows).map(|i| json!({ "id": format!("r{i}"), "cells": [i, i * 2] })).collect();
    json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "a" }, { "id": "c2", "name": "b" }], "rows": rows })
}

#[test]
fn scheduler_thread_runs_a_due_job_and_commits_it() {
    let data = fresh_dir("oku-slides-ws-compact-");
    let ws = workspace_on(data.path());
    let tab = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();
    let d9 = DataId::try_from("d9".to_owned()).unwrap_or_else(|e| panic!("{e:?}"));
    let add: Op = serde_json::from_value(
        json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table(COMPACT_AT / 2) }),
    )
    .unwrap_or_else(|e| panic!("op: {e}"));
    Workspace::with_session(&ws, &tab, |s| {
        s.open(Path::new(FIXTURE))?;
        s.apply("données", vec![add], None)?;
        // Its creation no longer an undo step, d9 is idle and over COMPACT_AT: due.
        s.doc.sources.drop_data_undo(&d9);
        Ok(())
    })
    .0
    .and_then(|r: crate::Result<()>| r)
    .unwrap_or_else(|e| panic!("{}", e.one_line()));

    let handles = Workspace::schedule_compactions(&ws, &tab);
    assert_that!(handles.len()).is_equal_to(1);
    for handle in handles {
        handle.join().unwrap_or_else(|_| panic!("compaction thread panicked"));
    }

    let (base, overlay) = Workspace::with_session(&ws, &tab, |s| {
        let table = &s.deck().data[&d9].table;
        (table.base.clone(), table.overlay_len())
    })
    .0
    .unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(base.is_some()).is_true();
    assert_that!(overlay).is_equal_to(0);
}
