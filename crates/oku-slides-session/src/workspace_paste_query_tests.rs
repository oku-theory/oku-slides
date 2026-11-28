//! Pasting a view over query data (t-1777): the query itself in its own deck, its current result
//! as manual data in another.

use oku_slides_core::Op;
use oku_slides_data::DataKind;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::workspace_paste_tests::{Two, add, chart, fr, fr_text, op, two_with};
use crate::Session;
use crate::derived::derived_tests::{TOP, with_query};
use crate::test_support::{fresh_dir, opened_on};

const PASTED: &str = "session-data-query-pasted-as-values";
const NO_RESULT: &str = "session-data-query-paste-no-result";

/// Two tabs; `a` holds `ventes` (`m`), the query `top` (`q`, labelled "Top") over it, ready, and
/// the chart `ch1` showing `q`.
fn query_two() -> Two {
    let t = two_with(None);
    let table = json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "a" }, { "id": "c2", "name": "b" }],
        "rows": [{ "id": "r0", "cells": ["x", 1.0] }, { "id": "r1", "cells": ["y", -1.0] }] });
    let ops: Vec<Op> = [
        json!({ "op": "add_data", "id": "m", "name": "ventes", "table": table }),
        json!({ "op": "add_data", "id": "q", "name": "top", "label": "Top", "sql": TOP }),
        add(chart("ch1", "q")),
    ]
    .into_iter()
    .map(op)
    .collect();
    // Settled under the tab's lock, so no background job is left to race the paste.
    t.on(&t.a.clone(), |s| {
        s.apply("requête", ops, None).unwrap_or_else(|e| panic!("{}", e.one_line()));
        s.settle_queries();
    });
    t
}

fn ids(columns: &[oku_slides_data::Column]) -> Vec<&str> {
    columns.iter().map(|c| c.id.as_str()).collect()
}

#[test]
fn paste_query_view_same_deck_copies_query_as_name_2() {
    let t = query_two();
    t.paste(&t.a, &t.a, &[add(chart("ch2", "q"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.names(&t.a)).is_equal_to(vec!["top".to_owned(), "top_2".to_owned(), "ventes".to_owned()]);
    let (copy, data) = t.named(&t.a, "top_2");
    assert_that!(data.kind()).is_equal_to(DataKind::Query);
    assert_that!(data.sql()).is_equal_to(Some(TOP));
    assert_that!(data.label.as_deref()).is_equal_to(Some("Top"));
    assert_that!(t.view_ref(&t.a, "ch2", "option")).is_equal_to(copy);
    assert_that!(t.view_ref(&t.a, "ch1", "option")).is_equal_to("q".to_owned());
}

#[test]
fn paste_query_view_into_other_deck_copies_result_as_manual_data() {
    let t = query_two();
    let reply = t.paste(&t.a, &t.b, &[add(chart("ch1", "q"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let snapshot: Value = serde_json::from_str(&reply.into_json()).unwrap_or_else(|e| panic!("{e}"));
    // The workspace speaks French here: the notice names the data in the session's language.
    let fr_notice = fr_text(PASTED, &[("data", "top".into())]);
    assert_that!(fr_notice).does_not_contain(PASTED);
    assert_that!(snapshot["notice"].as_str().unwrap_or_default()).contains(fr_notice.as_str());

    assert_that!(t.names(&t.b)).is_equal_to(vec!["top".to_owned()]);
    let (copy, data) = t.named(&t.b, "top");
    assert_that!(data.kind()).is_equal_to(DataKind::Manual);
    assert_that!(data.label.as_deref()).is_equal_to(Some("Top"));
    let table = data.table().unwrap_or_else(|| panic!("manual"));
    assert_that!(ids(&table.columns)).is_equal_to(vec!["a", "b"]);
    assert_that!(table.rows.len()).is_equal_to(1);
    assert_that!(table.base.is_some()).is_true();
    assert_that!(t.view_ref(&t.b, "ch1", "option")).is_equal_to(copy);
    // The copy owns its base: the target knows and leases it.
    let known = t.on(&t.b.clone(), |s| s.doc.sources.bases.clone());
    assert_that!(table.base.as_ref().is_some_and(|b| known.contains(&b.id))).is_true();
}

/// A session with `q` over `m` that no job computed: its result is not ready.
fn pending() -> (Session, tempfile::TempDir) {
    with_query(TOP)
}

#[test]
fn paste_pending_query_view_into_other_deck_is_refused() {
    let (source, _keep) = pending();
    let dir = fresh_dir("oku-slides-paste-query-");
    let mut target = opened_on(dir.path());
    let ops = vec![op(add(chart("ch2", "q")))];
    let before = target.deck().clone();
    let paste = source.paste_source(&ops);
    let e = target.paste("Coller", ops, paste).map(|_| ()).unwrap_err();
    assert_that!(fr(&e)).is_equal_to(fr_text(NO_RESULT, &[("data", "top".into())]));
    assert_that!(fr(&e)).does_not_contain(NO_RESULT);
    assert_that!(target.deck()).is_equal_to(&before);
}
