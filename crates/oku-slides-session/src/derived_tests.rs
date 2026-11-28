use std::sync::{Arc, mpsc};
use std::time::Duration;

use oku_slides_core::{Op, OpErrorKind};
use oku_slides_data::{DataError, DataErrorKind, DataId};
use oku_slides_i18n::{Localize, Localizer};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::QueryState;
use crate::test_support::{fresh_dir, opened_on, workspace_on};
use crate::{Session, Workspace};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
/// Keeps the rows of `ventes` whose `b` is positive.
pub(crate) const TOP: &str = "SELECT a, b FROM ventes WHERE b > 0";

pub(crate) fn id(s: &str) -> DataId {
    DataId::try_from(s.to_owned()).unwrap_or_else(|e| panic!("{e:?}"))
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op: {e}"))
}

pub(crate) fn apply(s: &mut Session, ops: Vec<Value>) {
    s.apply("edit", ops.into_iter().map(op).collect(), None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
}

/// `ventes`: a text column `a` (`c1`) and a number column `b` (`c2`), one row `r<i>` per value.
fn ventes(values: &[(&str, f64)]) -> Value {
    let rows: Vec<Value> =
        values.iter().enumerate().map(|(i, (a, b))| json!({ "id": format!("r{i}"), "cells": [a, b] })).collect();
    json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "a" }, { "id": "c2", "name": "b" }], "rows": rows })
}

/// Sets `b` of row `r<row>` of `ventes`.
pub(crate) fn set_b(row: usize, value: f64) -> Value {
    json!({ "op": "edit_data", "data": "m", "change":
        { "op": "set_cell", "row": format!("r{row}"), "column": "c2", "value": value } })
}

/// The roadmap deck with the manual data `m` (`ventes`), the query data `q` (`top`, `sql`) and
/// a chart `ch` on slide `s-libre` showing `q`; the session's data folder holds the bases.
pub(crate) fn with_query(sql: &str) -> (Session, tempfile::TempDir) {
    let dir = fresh_dir("oku-slides-derived-");
    let mut s = opened_on(dir.path());
    let chart = json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
        "element": { "id": "ch", "kind": "base/chart", "frame": { "x": 0, "y": 0, "w": 400, "h": 240 },
            "params": { "option": { "data": "q", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }
    });
    apply(
        &mut s,
        vec![
            json!({ "op": "add_data", "id": "m", "name": "ventes", "table": ventes(&[("x", 1.0), ("y", -1.0), ("z", -1.0)]) }),
            json!({ "op": "add_data", "id": "q", "name": "top", "sql": sql }),
            chart,
        ],
    );
    (s, dir)
}

/// The ready result of `data`, and its key.
pub(crate) fn ready(s: &Session, data: &str) -> (super::ResultKey, Arc<oku_slides_render_worker::QueryOut>) {
    match s.query_state(&id(data)) {
        Some(QueryState::Ready { key, out, .. }) => (*key, Arc::clone(out)),
        other => panic!("{data} not ready: {other:?}"),
    }
}

/// The rows of the ready result of `data`.
pub(crate) fn rows(s: &Session, data: &str) -> u32 {
    ready(s, data).1.base.rows
}

/// The hash of the page of slide `s-libre`, in the installed render.
pub(crate) fn page_hash(s: &Session) -> u128 {
    let page = s.page_of("s-libre").unwrap_or_else(|| panic!("no page s-libre"));
    let compiled = s.compiled.as_ref().unwrap_or_else(|| panic!("not rendered"));
    compiled.page_hash(page).unwrap_or_else(|| panic!("no page hash"))
}

fn bind(s: &mut Session, columns: &[&str]) -> crate::Result<()> {
    let bind = json!({ "op": "bind_data", "target": { "slide": "s-libre", "element": "ch" }, "param": "option",
        "data": "q", "columns": columns });
    s.apply("bind", vec![op(bind)], None)
}

#[test]
fn edit_input_recomputes_dependent_then_rerenders_view() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    assert_that!(rows(&s, "q")).is_equal_to(1);
    let drawn = page_hash(&s);

    apply(&mut s, vec![set_b(1, 5.0)]);
    let pending = page_hash(&s);
    assert_that!(s.query_results(s.deck()).get(&id("q"))).is_equal_to(Some(&None));
    s.settle_queries();

    assert_that!(rows(&s, "q")).is_equal_to(2);
    assert_that!(pending).is_not_equal_to(drawn);
    assert_that!(page_hash(&s)).is_not_equal_to(pending);
}

#[test]
fn chain_recomputes_in_topological_order() {
    let (mut s, _dir) = with_query(TOP);
    // `a2` reads `top`, computed by `q`: a smaller id, computed after.
    apply(&mut s, vec![json!({ "op": "add_data", "id": "a2", "name": "big", "sql": "SELECT a FROM top WHERE b > 2" })]);
    s.settle_queries();
    assert_that!(rows(&s, "a2")).is_equal_to(0);

    apply(&mut s, vec![set_b(2, 5.0)]);
    let mut computed = Vec::new();
    loop {
        let jobs = s.query_jobs();
        if jobs.is_empty() {
            break;
        }
        for job in jobs {
            computed.push(job.data.clone());
            let reply = job.run();
            s.finish_query(job, reply);
        }
    }

    assert_that!(computed).is_equal_to(vec![id("q"), id("a2")]);
    assert_that!(rows(&s, "a2")).is_equal_to(1);
}

#[test]
fn recompute_after_burst_of_edits_matches_last_inputs() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    apply(&mut s, vec![set_b(1, 5.0)]);
    let jobs = s.query_jobs();
    assert_that!(jobs.len()).is_equal_to(1);

    apply(&mut s, vec![set_b(2, 5.0)]);
    // One job per data in flight: none until the first ends.
    assert_that!(s.query_jobs().is_empty()).is_true();
    for job in jobs {
        let reply = job.run();
        s.finish_query(job, reply);
    }
    // Its inputs changed meanwhile: the result is discarded, the query computes again.
    assert_that!(matches!(s.query_state(&id("q")), Some(QueryState::Pending))).is_true();
    s.settle_queries();

    assert_that!(rows(&s, "q")).is_equal_to(3);
}

#[test]
fn failed_query_view_shows_placeholder_and_issue_fr() {
    let (mut s, _dir) = with_query("SELECT zzz FROM ventes");
    s.settle_queries();

    let Some(QueryState::Failed { error, .. }) = s.query_state(&id("q")) else {
        panic!("q not failed: {:?}", s.query_state(&id("q")))
    };
    let fr = Localizer::new(Some("fr"));
    let reason = error.localize(&fr);
    let issues = s.query_issues();
    assert_that!(issues.iter().map(|i| i.message.id()).collect::<Vec<_>>())
        .is_equal_to(vec!["issue-data-query-failed"]);
    assert_that!(issues[0].localize(&fr))
        .is_equal_to(fr.tr_with("issue-data-query-failed", &[("data", "top".into()), ("reason", reason.into())]));
    assert_that!(s.query_results(s.deck()).get(&id("q"))).is_equal_to(Some(&None));
}

#[test]
fn ready_query_view_reading_a_column_its_result_lacks_raises_issue() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    bind(&mut s, &["a", "b"]).unwrap_or_else(|e| panic!("bind: {}", e.one_line()));
    assert_that!(s.query_issues().is_empty()).is_true();

    apply(&mut s, vec![json!({ "op": "set_query", "data": "q", "sql": "SELECT a FROM ventes WHERE b > 0" })]);
    s.settle_queries();

    let issues = s.query_issues();
    let fr = Localizer::new(Some("fr"));
    assert_that!(issues.iter().map(|i| i.message.id()).collect::<Vec<_>>())
        .is_equal_to(vec!["issue-view-column-missing"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("ch"));
    assert_that!(issues[0].localize(&fr)).contains("ch");
    assert_that!(issues[0].localize(&fr)).contains("b");
}

#[test]
fn ready_query_table_past_its_rows_raises_truncated_issue() {
    let (mut s, _dir) = with_query(TOP);
    let names: Vec<String> = (0..120).map(|i| format!("n{i}")).collect();
    let many: Vec<(&str, f64)> = names.iter().map(|n| (n.as_str(), 1.0)).collect();
    let table = json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
        "element": { "id": "tb", "kind": "base/table", "frame": { "x": 0, "y": 300, "w": 400, "h": 200 },
            "params": { "data": { "data": "q3" } } }
    });
    apply(
        &mut s,
        vec![
            json!({ "op": "add_data", "id": "big", "name": "grand", "table": ventes(&many) }),
            json!({ "op": "add_data", "id": "q3", "name": "tout", "sql": "SELECT a, b FROM grand" }),
            table,
        ],
    );
    s.settle_queries();
    assert_that!(rows(&s, "q3")).is_equal_to(120);

    let issues = s.query_issues();

    assert_that!(issues.iter().map(|i| i.message.id()).collect::<Vec<_>>()).is_equal_to(vec!["issue-view-truncated"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("tb"));
    let fr = Localizer::new(Some("fr"));
    assert_that!(issues[0].localize(&fr)).contains("120");
}

#[test]
fn pending_query_view_has_no_issue() {
    let (mut s, _dir) = with_query(TOP);
    let jobs = s.query_jobs();
    assert_that!(jobs.len()).is_equal_to(1);

    assert_that!(matches!(s.query_state(&id("q")), Some(QueryState::Pending))).is_true();
    assert_that!(s.query_issues().is_empty()).is_true();
    assert_that!(s.query_results(s.deck()).get(&id("q"))).is_equal_to(Some(&None));
}

#[test]
fn unchanged_inputs_do_not_recompute() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    let (key, out) = ready(&s, "q");

    apply(&mut s, vec![json!({ "op": "add_data", "id": "o", "name": "autre", "table": ventes(&[("w", 2.0)]) })]);

    assert_that!(s.query_jobs().is_empty()).is_true();
    let (after, same) = ready(&s, "q");
    assert_that!(after).is_equal_to(key);
    assert_that!(Arc::ptr_eq(&out, &same)).is_true();
}

#[test]
fn bind_to_query_without_result_is_refused() {
    let (mut s, _dir) = with_query(TOP);
    let before = s.deck().clone();

    let err = bind(&mut s, &["a"]).unwrap_err();

    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr_with("session-data-query-no-result", &[("data", "top".into())]));
    assert_that!(*s.deck()).is_equal_to(&before);
}

#[test]
fn bind_to_query_with_missing_column_is_refused() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    let before = s.deck().clone();

    let err = bind(&mut s, &["a", "zzz"]).unwrap_err();

    let kind = err.transaction().map(|t| t.op_error().kind());
    assert_that!(kind).is_equal_to(Some(OpErrorKind::Data(DataErrorKind::NoColumn)));
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).contains(DataError::no_column("zzz").localize(&fr).as_str());
    assert_that!(*s.deck()).is_equal_to(&before);
    bind(&mut s, &["a", "b"]).unwrap_or_else(|e| panic!("bind: {}", e.one_line()));
}

#[test]
fn undo_of_input_edit_recomputes_back() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    let (key, _) = ready(&s, "q");
    apply(&mut s, vec![set_b(1, 5.0)]);
    s.settle_queries();
    assert_that!(rows(&s, "q")).is_equal_to(2);

    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    let jobs = s.query_jobs();
    assert_that!(jobs.len()).is_equal_to(1);
    for job in jobs {
        let reply = job.run();
        s.finish_query(job, reply);
    }

    assert_that!(rows(&s, "q")).is_equal_to(1);
    assert_that!(ready(&s, "q").0).is_equal_to(key);
}

#[test]
fn query_reading_failed_input_fails_naming_it_then_recomputes_on_recovery() {
    let (mut s, _dir) = with_query("SELECT zzz FROM ventes");
    apply(&mut s, vec![json!({ "op": "add_data", "id": "a2", "name": "big", "sql": "SELECT a FROM top" })]);
    s.settle_queries();

    let Some(QueryState::Failed { error, .. }) = s.query_state(&id("a2")) else {
        panic!("a2 not failed: {:?}", s.query_state(&id("a2")))
    };
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryInputFailed);
    let fr = Localizer::new(Some("fr"));
    let reason = error.localize(&fr);
    assert_that!(reason.as_str()).is_equal_to(DataError::query_input_failed("top").localize(&fr).as_str());
    assert_that!(reason.as_str()).contains("top");
    let issues: Vec<String> = s.query_issues().iter().map(|i| i.localize(&fr)).collect();
    let named = fr.tr_with("issue-data-query-failed", &[("data", "big".into()), ("reason", reason.clone().into())]);
    assert_that!(issues).contains(named);

    apply(&mut s, vec![json!({ "op": "set_query", "data": "q", "sql": TOP })]);
    let jobs = s.query_jobs();
    assert_that!(jobs.iter().map(|j| j.data.clone()).collect::<Vec<_>>()).is_equal_to(vec![id("q")]);
    assert_that!(matches!(s.query_state(&id("a2")), Some(QueryState::Pending))).is_true();
    for job in jobs {
        let reply = job.run();
        s.finish_query(job, reply);
    }
    s.settle_queries();

    assert_that!(rows(&s, "a2")).is_equal_to(1);
    assert_that!(s.query_issues().is_empty()).is_true();
}

#[test]
fn dropped_job_is_recomputed_on_next_request() {
    let (mut s, _dir) = with_query(TOP);
    let jobs = s.query_jobs();
    assert_that!(jobs.len()).is_equal_to(1);
    assert_that!(s.query_jobs().is_empty()).is_true();

    drop(jobs);

    let again = s.query_jobs();
    assert_that!(again.iter().map(|j| j.data.clone()).collect::<Vec<_>>()).is_equal_to(vec![id("q")]);
}

#[test]
fn bind_in_batch_changing_query_inputs_is_refused() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    let before = s.deck().clone();
    let bind = json!({ "op": "bind_data", "target": { "slide": "s-libre", "element": "ch" }, "param": "option",
        "data": "q", "columns": ["a"] });

    let err = s.apply("bind", vec![op(set_b(1, 5.0)), op(bind)], None).unwrap_err();

    let fr = Localizer::new(Some("fr"));
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::QueryNoResult);
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr_with("session-data-query-no-result", &[("data", "top".into())]));
    assert_that!(*s.deck()).is_equal_to(&before);
}

#[test]
fn bind_in_batch_setting_another_ready_query_is_accepted() {
    let (mut s, _dir) = with_query(TOP);
    apply(&mut s, vec![json!({ "op": "add_data", "id": "q2", "name": "tous", "sql": "SELECT a FROM ventes" })]);
    s.settle_queries();
    let set = json!({ "op": "set_query", "data": "q2", "sql": "SELECT b FROM ventes" });
    let bind = |columns: &[&str]| {
        op(json!({ "op": "bind_data", "target": { "slide": "s-libre", "element": "ch" }, "param": "option",
            "data": "q", "columns": columns }))
    };
    let missing = s.apply("bind", vec![op(set.clone()), bind(&["a", "zzz"])], None).unwrap_err();
    let kind = missing.transaction().map(|t| t.op_error().kind());
    assert_that!(kind).is_equal_to(Some(OpErrorKind::Data(DataErrorKind::NoColumn)));

    let applied = s.apply("bind", vec![op(set), bind(&["a"])], None);

    assert_that!(applied.map_err(|e| e.one_line())).is_ok();
    assert_that!(s.deck().data.get(&id("q2")).and_then(|d| d.sql())).is_equal_to(Some("SELECT b FROM ventes"));
}

/// A workspace whose tab `t-1` shows the roadmap, its query results told on the returned channel.
fn workspace_with_sink(data: &std::path::Path) -> (std::sync::Mutex<Workspace>, String, mpsc::Receiver<String>) {
    let ws = workspace_on(data);
    let tab = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();
    let (tx, rx) = mpsc::channel();
    let tx = std::sync::Mutex::new(tx);
    Workspace::on_results(&ws, move |t| {
        let _sent = tx.lock().unwrap_or_else(std::sync::PoisonError::into_inner).send(t.to_owned());
    });
    let opened = Workspace::with_session(&ws, &tab, |s| s.open(std::path::Path::new(FIXTURE))).0;
    opened.and_then(|r| r).unwrap_or_else(|e| panic!("{}", e.one_line()));
    (ws, tab, rx)
}

/// Waits for the sink to be told `tab`.
fn landed(rx: &mpsc::Receiver<String>, tab: &str) {
    let told = rx.recv_timeout(Duration::from_secs(60)).unwrap_or_else(|e| panic!("no result landed: {e}"));
    assert_that!(told.as_str()).is_equal_to(tab);
}

#[test]
fn request_recomputes_off_the_lock_and_notifies_results_sink() {
    let data = fresh_dir("oku-slides-ws-derived-");
    let (ws, tab, rx) = workspace_with_sink(data.path());
    let ops = vec![
        op(json!({ "op": "add_data", "id": "m", "name": "ventes", "table": ventes(&[("x", 1.0), ("y", 2.0)]) })),
        op(json!({ "op": "add_data", "id": "q", "name": "top", "sql": TOP })),
    ];

    let (reply, _) = Workspace::request(&ws, &tab, crate::Request::Apply { label: "données".into(), ops, merge: None });
    reply.unwrap_or_else(|e| panic!("{}", e.one_line()));
    landed(&rx, &tab);

    let rows = Workspace::with_session(&ws, &tab, |s| rows(s, "q")).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(rows).is_equal_to(2);
    assert_that!(Workspace::schedule_recomputes(&ws, &tab).is_none()).is_true();
}

#[test]
fn with_session_edit_recomputes_dependent_notifies_sink_and_redraws_view() {
    let data = fresh_dir("oku-slides-ws-derived-");
    let (ws, tab, rx) = workspace_with_sink(data.path());
    let chart = json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
        "element": { "id": "ch", "kind": "base/chart", "frame": { "x": 0, "y": 0, "w": 400, "h": 240 },
            "params": { "option": { "data": "q", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }
    });
    let ops = vec![
        op(json!({ "op": "add_data", "id": "m", "name": "ventes", "table": ventes(&[("x", 1.0), ("y", -2.0)]) })),
        op(json!({ "op": "add_data", "id": "q", "name": "top", "sql": TOP })),
        op(chart),
    ];
    let applied = Workspace::with_session(&ws, &tab, |s| s.apply("données", ops, None)).0;
    applied.and_then(|r| r).unwrap_or_else(|e| panic!("{}", e.one_line()));
    landed(&rx, &tab);
    let drawn = Workspace::with_session(&ws, &tab, |s| page_hash(s)).0.unwrap_or_else(|e| panic!("{}", e.one_line()));

    // The agent's path: an input edit through `with_session`, no UI request after it.
    let edited = Workspace::with_session(&ws, &tab, |s| s.apply("cellule", vec![op(set_b(1, 5.0))], None)).0;
    edited.and_then(|r| r).unwrap_or_else(|e| panic!("{}", e.one_line()));
    landed(&rx, &tab);

    let (rows, redrawn) = Workspace::with_session(&ws, &tab, |s| (rows(s, "q"), page_hash(s)))
        .0
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(rows).is_equal_to(2);
    assert_that!(redrawn).is_not_equal_to(drawn);
}

#[test]
fn a_ready_state_carries_the_table_its_status_shows() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    let Some(QueryState::Ready { out, table, .. }) = s.query_state(&id("q")) else {
        panic!("q not ready: {:?}", s.query_state(&id("q")))
    };
    assert_that!(*table).is_equal_to(out.table());
    let statuses = s.query_statuses();
    let Some(super::QueryStatus::Ready(shown)) = statuses.get(&id("q")) else { panic!("q not ready: {statuses:?}") };
    assert_that!(std::ptr::eq(*shown, table)).is_true();
}

#[test]
fn a_query_missing_from_the_statuses_is_failed_not_pending_fr() {
    let (s, _dir) = with_query(TOP);
    let none = std::collections::BTreeMap::new();
    let status = super::query_status(&none, &id("q"), &s.deck().data[&id("q")]);
    let Some(super::QueryStatus::Failed(message)) = status else { panic!("q not failed: {status:?}") };
    assert_that!(message.id()).is_equal_to("session-data-query-no-state");
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with("session-data-query-no-state", &[("data", "top".into())]);
    assert_that!(message.localize(&fr)).is_equal_to(expected);
    assert_that!(message.localize(&fr)).is_not_equal_to(message.localize(&Localizer::new(Some("en"))));
    let manual = super::query_status(&none, &id("m"), &s.deck().data[&id("m")]);
    assert_that!(manual.is_none()).is_true();
}
