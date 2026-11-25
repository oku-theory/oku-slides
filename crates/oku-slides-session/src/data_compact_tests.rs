use std::path::Path;
use std::time::{Duration, Instant};

use oku_slides_core::{Op, Origin};
use oku_slides_data::DataId;
use oku_slides_data::limits::COMPACT_AT;
use oku_slides_i18n::{FluentValue, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{Compaction, FIRST_BACKOFF, Growth, JobKey, MAX_BACKOFF, Refusal, Reply, is_due};
use crate::Session;
use crate::data_read::RowsRead;
use crate::test_support::{fresh_dir, session_on, session_with, settings_on};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

fn id(s: &str) -> DataId {
    DataId::try_from(s.to_owned()).unwrap_or_else(|e| panic!("{e}"))
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

/// A table of `rows` rows of two numbers, `generation` `g1`.
fn table(rows: usize) -> Value {
    let rows: Vec<Value> = (0..rows).map(|i| json!({ "id": format!("r{i}"), "cells": [i, i * 2] })).collect();
    json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "a" }, { "id": "c2", "name": "b" }], "rows": rows })
}

fn apply(s: &mut Session, ops: Vec<Op>) {
    s.apply("données", ops, None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
}

fn set_cell(data: &str, row: &str, value: i64) -> Op {
    op(json!({ "op": "edit_data", "data": data,
        "change": { "op": "set_cell", "row": row, "column": "c2", "value": value } }))
}

/// The fixture with `d9` of `rows` rows, its creation no longer an undo step.
fn opened_with(data: &Path, rows: usize) -> Session {
    with_d9(session_on(data), rows)
}

/// `s` on the fixture with `d9` of `rows` rows, its creation no longer an undo step.
fn with_d9(mut s: Session, rows: usize) -> Session {
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table(rows) }))]);
    s.doc.sources.drop_data_undo(&id("d9"));
    s
}

/// Applies `ops` as the host does (no external batch bound), in one undo step.
fn host_apply(s: &mut Session, ops: Vec<Op>) {
    let deck = s.doc.sources.deck().clone();
    let prepared = crate::data_ops::prepare(&deck, &s.registry, &s.chart_labels, ops, Origin::Host)
        .unwrap_or_else(|e| panic!("prepare: {e}"));
    let mut next = deck;
    oku_slides_core::apply_all(&mut next, prepared.ops, &s.registry).unwrap_or_else(|e| panic!("apply: {e:?}"));
    s.doc.sources.apply(next, "hôte", None).unwrap_or_else(|e| panic!("sources: {e:?}"));
}

/// Runs `d9`'s due compaction to its end; whether it committed.
fn compact_now(s: &mut Session) -> bool {
    let now = Instant::now();
    let job = s.compaction_job(&id("d9"), now, false).unwrap_or_else(|| panic!("no job"));
    let reply = job.run();
    s.finish_compaction(job, reply, now)
}

/// `f` on the session behind `s`, a fold it asks run between two calls as a host runs it.
fn with_folding<R>(s: &std::sync::Mutex<Session>, f: impl FnMut(&mut Session) -> R) -> R {
    let with = |g: &mut dyn FnMut(&mut Session)| {
        g(&mut s.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
        Ok::<(), std::convert::Infallible>(())
    };
    match super::folding(with, f) {
        Ok(out) => out.unwrap_or_else(|| panic!("f not called")),
        Err(e) => match e {},
    }
}

/// Folds `data` now, as an overflowing apply does once the lock is released; whether it did.
fn fold_now(s: &mut Session, data: &str) -> bool {
    let now = Instant::now();
    let job = s.compaction_job(&id(data), now, true).unwrap_or_else(|| panic!("no job"));
    let reply = job.run();
    s.finish_fold(job, reply, now)
}

/// The cells of `data`, read through the worker when it has a base.
fn cells(s: &Session, data: &str) -> Value {
    let d = &s.deck().data[&id(data)];
    let rows = d.table.rows.len();
    let set = s
        .rows_read(d, None, 0..rows, usize::MAX)
        .and_then(RowsRead::run)
        .unwrap_or_else(|e| panic!("read: {}", e.one_line()));
    serde_json::to_value(set).unwrap_or_else(|e| panic!("{e}"))
}

#[rstest]
#[case::at_threshold(COMPACT_AT, false, false, false, true)]
#[case::below(COMPACT_AT - 1, false, false, false, false)]
#[case::undo_touches(COMPACT_AT, true, false, false, false)]
#[case::in_flight(COMPACT_AT, false, true, false, false)]
#[case::backing_off(COMPACT_AT, false, false, true, false)]
fn compaction_due_follows_the_rule(
    #[case] overlay: usize,
    #[case] undo: bool,
    #[case] in_flight: bool,
    #[case] backing_off: bool,
    #[case] due: bool,
) {
    assert_that!(is_due(overlay, undo, in_flight, backing_off)).is_equal_to(due);
}

#[test]
fn compaction_keeps_every_cell_value() {
    let dir = fresh_dir("oku-slides-compact-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    let before = cells(&s, "d9");
    assert_that!(s.compactions_due(Instant::now())).contains(id("d9"));
    assert_that!(compact_now(&mut s)).is_true();
    let table = &s.deck().data[&id("d9")].table;
    assert_that!(table.base.is_some()).is_true();
    assert_that!(table.overlay_len()).is_equal_to(0);
    assert_that!(cells(&s, "d9")).is_equal_to(before);
    assert_that!(s.compactions_due(Instant::now())).is_empty();
}

#[test]
fn compaction_is_not_an_undo_step() {
    let dir = fresh_dir("oku-slides-compact-undo-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d8", "name": "autre", "table": table(2) }))]);
    let before = cells(&s, "d8");
    apply(&mut s, vec![set_cell("d8", "r1", 40)]);
    let dirty = s.doc.dirty;
    assert_that!(compact_now(&mut s)).is_true();
    assert_that!(s.doc.dirty).is_equal_to(dirty);
    // The one undo undoes the edit of d8, not the compaction of d9.
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    assert_that!(cells(&s, "d8")).is_equal_to(before);
    assert_that!(s.deck().data[&id("d9")].table.base.is_some()).is_true();
}

#[test]
fn edit_during_compaction_drops_the_job() {
    let dir = fresh_dir("oku-slides-compact-race-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    let now = Instant::now();
    let job = s.compaction_job(&id("d9"), now, false).unwrap_or_else(|| panic!("no job"));
    assert_that!(s.compaction_job(&id("d9"), now, false).is_none()).is_true();
    let reply = job.run();
    apply(&mut s, vec![set_cell("d9", "r3", 99)]);
    assert_that!(s.finish_compaction(job, reply, now)).is_false();
    assert_that!(s.deck().data[&id("d9")].table.base.is_none()).is_true();
}

#[test]
fn compaction_dropped_when_generation_changed() {
    let dir = fresh_dir("oku-slides-compact-gen-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    let now = Instant::now();
    let job = s.compaction_job(&id("d9"), now, false).unwrap_or_else(|| panic!("no job"));
    let reply = job.run();
    let replace = json!({ "op": "edit_data", "data": "d9", "change": { "op": "replace", "table": table(3) } });
    apply(&mut s, vec![op(replace)]);
    assert_that!(s.finish_compaction(job, reply, now)).is_false();
    assert_that!(s.deck().data[&id("d9")].table.rows.len()).is_equal_to(3);
}

fn key(seq: u64) -> JobKey {
    JobKey { generation: "g1".into(), epoch: "e1".into(), touch_seq: seq }
}

#[test]
fn failure_retries_with_backoff_and_memo() {
    let (mut c, d9, t0) = (Compaction::default(), id("d9"), Instant::now());
    c.fail(&d9, key(0), t0);
    assert_that!(c.backing_off(&d9, &key(0), t0 + Duration::from_secs(59))).is_true();
    assert_that!(c.backing_off(&d9, &key(0), t0 + FIRST_BACKOFF)).is_false();
    // A changed data is a new key: tried again at once.
    assert_that!(c.backing_off(&d9, &key(1), t0)).is_false();
    c.fail(&d9, key(0), t0);
    assert_that!(c.failed[&d9].delay).is_equal_to(FIRST_BACKOFF * 2);
    for _ in 0..10 {
        c.fail(&d9, key(0), t0);
    }
    assert_that!(c.failed[&d9].delay).is_equal_to(MAX_BACKOFF);
    c.fail(&d9, key(1), t0);
    assert_that!(c.failed[&d9].delay).is_equal_to(FIRST_BACKOFF);
}

#[test]
fn failed_reply_backs_off_the_data() {
    let dir = fresh_dir("oku-slides-compact-fail-");
    let mut s = opened_with(dir.path(), COMPACT_AT / 2);
    let now = Instant::now();
    let job = s.compaction_job(&id("d9"), now, false).unwrap_or_else(|| panic!("no job"));
    assert_that!(s.finish_compaction(job, Reply::Failed, now)).is_false();
    assert_that!(s.compactions_due(now)).is_empty();
    assert_that!(s.compactions_due(now + FIRST_BACKOFF)).contains(id("d9"));
}

#[test]
fn store_full_skips_and_reports_fr() {
    let dir = fresh_dir("oku-slides-compact-full-");
    let mut settings = settings_on(dir.path());
    settings.sources.max_store_mib = 0;
    let mut s = session_with(settings);
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table(COMPACT_AT / 2) }))]);
    s.doc.sources.drop_data_undo(&id("d9"));
    crate::bases::bases_tests::put_base(s.bases.store(), b"une base");
    s.doc.notice = None;
    assert_that!(compact_now(&mut s)).is_false();
    let full = s.localizer.tr_with("session-data-store-full", &[("data", "ventes".into())]);
    assert_that!(s.doc.notice.clone().unwrap_or_default()).contains(full.as_str());
    assert_that!(s.deck().data[&id("d9")].table.base.is_none()).is_true();
}

#[test]
fn fold_cuts_undo_with_notice_fr() {
    let dir = fresh_dir("oku-slides-compact-fold-");
    let mut s = opened_with(dir.path(), 4);
    apply(&mut s, vec![set_cell("d9", "r1", 7)]);
    s.doc.notice = None;
    assert_that!(fold_now(&mut s, "d9")).is_true();
    assert_that!(s.can_undo()).is_false();
    assert_that!(s.deck().data[&id("d9")].table.base.is_some()).is_true();
    let cut = s.localizer.tr_with("session-data-undo-cut", &[("data", "ventes".into())]);
    assert_that!(s.doc.notice.clone().unwrap_or_default()).contains(cut.as_str());
}

#[test]
fn apply_folding_passes_other_errors_through() {
    let dir = fresh_dir("oku-slides-compact-other-");
    let mut s = opened_with(dir.path(), 4);
    let e = s.apply_folding("x", vec![set_cell("d7", "r1", 1)], None).err().unwrap_or_else(|| panic!("applied"));
    assert_that!(e.transaction().is_some_and(|t| t.op_error().is_overlay_full())).is_false();
    assert_that!(s.deck().data[&id("d9")].table.base.is_none()).is_true();
}

/// `d9` folded onto a base (empty overlay) and `d8` of four rows in its overlay.
fn d9_based_d8_overlaid(data: &Path) -> Session {
    let mut s = opened_with(data, 4);
    assert_that!(fold_now(&mut s, "d9")).is_true();
    apply(&mut s, vec![op(json!({ "op": "add_data", "id": "d8", "name": "autres", "table": table(4) }))]);
    s
}

#[rstest]
// A data's own cap: only the data it names may fold, and only if it holds an overlay.
#[case::data_cap_names_a_based_data(Refusal::Data, "d9", None)]
// The deck's cap: the largest touched data first, else the deck's largest.
#[case::deck_cap_touched(Refusal::Deck, "d8", Some("d8"))]
#[case::deck_cap_falls_back_to_the_largest(Refusal::Deck, "d9", Some("d8"))]
fn fold_target_folds_what_the_refusal_names(
    #[case] refusal: Refusal,
    #[case] edited: &str,
    #[case] expected: Option<&str>,
) {
    let dir = fresh_dir("oku-slides-compact-target-");
    let s = d9_based_d8_overlaid(dir.path());
    let ops = [op(json!({ "op": "edit_data", "data": edited,
        "change": { "op": "insert_row", "id": "n1", "cells": { "c1": 1, "c2": 2 } } }))];
    assert_that!(Growth::of(&ops).fold_target(s.deck(), refusal)).is_equal_to(expected.map(id));
}

#[test]
fn fold_target_refuses_a_fold_that_cannot_make_room() {
    let dir = fresh_dir("oku-slides-compact-room-");
    let s = d9_based_d8_overlaid(dir.path());
    // More new cells than the cap: folding d8's overlay away cannot make room for them.
    let cells: serde_json::Map<String, Value> =
        (0..oku_slides_data::limits::MAX_ROWS).map(|i| (format!("r{i}"), json!(i))).collect();
    let column = op(json!({ "op": "edit_data", "data": "d8",
        "change": { "op": "insert_column", "id": "c9", "name": "z", "cells": cells } }));
    let rows: Vec<Op> = (0..oku_slides_data::limits::MAX_OVERLAY / 2)
        .map(|i| {
            op(json!({ "op": "edit_data", "data": "d8",
            "change": { "op": "insert_row", "id": format!("n{i}"), "cells": { "c1": 1, "c2": 2 } } }))
        })
        .collect();
    let ops: Vec<Op> = rows.into_iter().chain([column]).collect();
    assert_that!(Growth::of(&ops).fold_target(s.deck(), Refusal::Data)).is_none();
    assert_that!(Growth::of(&ops).fold_target(s.deck(), Refusal::Deck)).is_none();
}

#[test]
fn every_task4f_key_formats_in_all_locales() {
    let keys: [(&str, &str, FluentValue<'static>); 2] =
        [("session-data-store-full", "data", "ventes".into()), ("op-replica-full", "max", 256.into())];
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, arg, value) in &keys {
            assert_that!(l.has(key)).named(&format!("{locale}: {key}")).is_true();
            let text = l.tr_with(key, &[(*arg, value.clone())]);
            assert_that!(text.contains('{')).named(&format!("{locale}: {text}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    let args = [("data", "ventes".into())];
    assert_that!(fr.tr_with("session-data-store-full", &args))
        .is_not_equal_to(en.tr_with("session-data-store-full", &args));
}

#[path = "data_compact_undo_tests.rs"]
mod data_compact_undo_tests;

#[path = "data_compact_overflow_tests.rs"]
mod data_compact_overflow_tests;

#[path = "data_compact_peer_tests.rs"]
mod data_compact_peer_tests;

#[path = "data_compact_worst_tests.rs"]
mod data_compact_worst_tests;
