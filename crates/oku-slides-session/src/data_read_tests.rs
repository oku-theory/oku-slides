//! A data pane on a based data reads its rows through the worker with no session lock held.

use std::path::Path;
use std::time::Instant;

use oku_slides_core::Op;
use oku_slides_data::DataId;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::test_support::{fresh_dir, session_on, workspace_on};
use crate::{Request, Session, Workspace};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

/// The ops adding `d9` (two rows) and the chart `ch1` on `s-libre` that reads it.
fn data_and_chart() -> Vec<Op> {
    let table = json!({ "generation": "g1",
        "columns": [{ "id": "c1", "name": "Mois", "type": "text" }, { "id": "c2", "name": "ventes", "type": "number" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 7] }, { "id": "r2", "cells": ["Fév", 41] }] });
    vec![
        op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table })),
        op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
            "element": { "id": "ch1", "frame": { "x": 0, "y": 0, "w": 400, "h": 300 }, "kind": "base/chart",
                "params": { "option": { "data": "d9", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } })),
    ]
}

/// Opens the fixture with `d9` and its chart, then folds `d9` onto a base.
fn based(s: &mut Session) {
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    s.apply("données", data_and_chart(), None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    let d9 = DataId::try_from("d9".to_owned()).unwrap_or_else(|e| panic!("{e}"));
    let now = Instant::now();
    let job = s.compaction_job(&d9, now, true).unwrap_or_else(|| panic!("no job"));
    let reply = job.run();
    assert_that!(s.finish_compaction(job, reply, now)).is_true();
    assert_that!(s.deck().data[&d9].table.base.is_some()).is_true();
}

#[test]
fn pane_read_of_a_based_data_runs_without_the_session() {
    let dir = fresh_dir("oku-slides-pane-read-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let pane = s.data_pane("s-libre", "ch1").unwrap_or_else(|| panic!("no pane"));
    // The read needs nothing from the session: the worker call runs once its lock is gone.
    drop(s);
    let surface = pane.surface().unwrap_or_else(|| panic!("no surface"));
    let text = serde_json::to_string(&surface).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(text).contains("Fév");
    assert_that!(text).contains("41");
}

#[test]
fn workspace_data_panel_reads_a_based_data() {
    let dir = fresh_dir("oku-slides-pane-ws-");
    let ws = workspace_on(dir.path());
    let tab = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();
    Workspace::with_session(&ws, &tab, based).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    let req = Request::DataPanel { slide: "s-libre".to_owned(), element: "ch1".to_owned() };
    let (reply, _) = Workspace::request(&ws, &tab, req);
    let reply = reply.unwrap_or_else(|e| panic!("pane: {}", e.one_line()));
    assert_that!(reply).contains("Fév");
    assert_that!(reply).contains("41");
}

fn d9() -> DataId {
    DataId::try_from("d9".to_owned()).unwrap_or_else(|e| panic!("{e}"))
}

fn retype(column: &str, ty: &str) -> Op {
    op(json!({ "op": "set_column_type", "data": "d9", "column": column, "type": ty }))
}

/// Applies `ops` the way a request does: a check the batch asks runs between two calls.
/// `between` runs on the session right before the retry.
fn apply_checked(s: &mut Session, ops: &[Op], between: impl FnMut(&mut Session)) -> crate::Result<()> {
    folded(s, |s| s.apply("type", ops.to_vec(), None), between)
}

/// Runs `f` the way a folding request does; `between` runs on the session right before the retry.
fn folded(
    s: &mut Session,
    mut f: impl FnMut(&mut Session) -> crate::Result<()>,
    mut between: impl FnMut(&mut Session),
) -> crate::Result<()> {
    let mut calls = 0;
    let with = |g: &mut dyn FnMut(&mut Session)| {
        calls += 1;
        if calls == 3 {
            between(s);
        }
        g(s);
        Ok::<(), ()>(())
    };
    crate::data_compact::folding(with, &mut f)
        .unwrap_or_else(|()| panic!("with failed"))
        .unwrap_or_else(|| panic!("f not called"))
}

fn column_type(s: &Session, column: &str) -> oku_slides_data::ColumnType {
    s.deck().data[&d9()].table.columns.iter().find(|c| c.id == column).unwrap_or_else(|| panic!("no {column}")).ty
}

#[test]
fn set_column_type_on_base_refuses_lossy_base_cell() {
    let dir = fresh_dir("oku-slides-retype-lossy-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let err = apply_checked(&mut s, &[retype("c1", "number")], |_| {}).unwrap_err();
    // The same refusal as the cells give before their fold, naming both rows.
    let Some(Op::AddData { table, .. }) = data_and_chart().into_iter().next() else { panic!("no add_data") };
    let (mut cells, _) = table.into_table().unwrap_or_else(|e| panic!("{e}"));
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let expected = cells.set_column_type("c1", oku_slides_data::ColumnType::Number).unwrap_err();
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(oku_slides_i18n::Localize::localize(&expected, &fr));
    assert_that!(column_type(&s, "c1")).is_equal_to(oku_slides_data::ColumnType::Text);
}

#[test]
fn set_column_type_on_base_converts_when_base_cells_fit() {
    let dir = fresh_dir("oku-slides-retype-fits-");
    let mut s = session_on(dir.path());
    based(&mut s);
    apply_checked(&mut s, &[retype("c2", "integer")], |_| {}).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(column_type(&s, "c2")).is_equal_to(oku_slides_data::ColumnType::Integer);
}

#[test]
fn set_column_type_on_base_refused_when_data_changes_during_check() {
    let dir = fresh_dir("oku-slides-retype-stale-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let edit = op(json!({ "op": "edit_data", "data": "d9",
        "change": { "op": "set_cell", "row": "r1", "column": "c2", "value": 8 } }));
    let err = apply_checked(&mut s, &[retype("c2", "integer")], |s| {
        s.apply("peer", vec![edit.clone()], None).unwrap_or_else(|e| panic!("{}", e.one_line()));
    })
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::BaseStale);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(fr.tr_with("session-data-base-stale", &[("data", "ventes".into())]));
    assert_that!(column_type(&s, "c2")).is_equal_to(oku_slides_data::ColumnType::Number);
}

#[test]
fn set_column_type_on_base_outside_a_checking_caller_is_refused() {
    let dir = fresh_dir("oku-slides-retype-direct-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let err = s.apply("type", vec![retype("c2", "integer")], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::BaseStale);
    assert_that!(column_type(&s, "c2")).is_equal_to(oku_slides_data::ColumnType::Number);
}

/// The agent's read of a based data takes the worker from the data link under the lock,
/// then reads with no session held.
#[test]
fn data_read_from_base_goes_through_data_link() {
    let dir = fresh_dir("oku-slides-agent-read-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let before = s.data_link.handed();
    let read = s.data_rows("d9", 100, 0..20, 200).unwrap_or_else(|| panic!("d9 unknown"));
    let (_, read) = read.unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(s.data_link.handed()).is_equal_to(before + 1);
    drop(s);
    let dataset = read.run().unwrap_or_else(|e| panic!("{}", e.one_line()));
    let text = serde_json::to_string(&dataset).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(text).contains("Fév");
    assert_that!(text).contains("41");
}

#[test]
fn data_rows_of_an_unknown_data_is_none() {
    let dir = fresh_dir("oku-slides-agent-read-none-");
    let mut s = session_on(dir.path());
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    assert_that!(s.data_rows("absente", 100, 0..20, 200).is_none()).is_true();
    assert_that!(s.data_rows("not an id!", 100, 0..20, 200).is_none()).is_true();
}

/// A view conversion of `ch1`, the one a `chart_kind` or `show_as` request runs.
#[derive(Clone, Copy, Debug)]
enum Conversion {
    ChartKind,
    ShowAs,
}

impl Conversion {
    fn run(self, s: &mut Session) -> crate::Result<()> {
        match self {
            Self::ChartKind => {
                s.set_chart_kind("kind", "s-libre".to_owned(), "ch1", oku_slides_chart::SeriesType::Line)
            }
            Self::ShowAs => s.show_as("table", "s-libre".to_owned(), "ch1", crate::View::Table),
        }
    }
}

/// The element `ch1` as JSON.
fn ch1(s: &Session) -> Value {
    let slide = s.deck().slide("s-libre").unwrap_or_else(|| panic!("no s-libre"));
    let located = slide.locate("ch1").unwrap_or_else(|| panic!("no ch1"));
    serde_json::to_value(located.element).unwrap_or_else(|e| panic!("{e}"))
}

/// Under the session lock, a conversion of a based view only plans its read: it hands out the
/// worker, reads nothing and waits for the read run off the lock.
#[rstest]
#[case::chart_kind(Conversion::ChartKind)]
#[case::show_as(Conversion::ShowAs)]
fn view_conversion_on_base_under_the_lock_is_refused_until_read(#[case] conversion: Conversion) {
    let dir = fresh_dir("oku-slides-view-lock-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let before = ch1(&s);
    let handed = s.data_link.handed();
    let err = conversion.run(&mut s).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::BaseStale);
    assert_that!(s.data_link.handed()).is_equal_to(handed + 1);
    assert_that!(ch1(&s)).is_equal_to(before);
}

#[rstest]
#[case::chart_kind(Conversion::ChartKind)]
#[case::show_as(Conversion::ShowAs)]
fn view_conversion_on_base_applies_the_read_done_off_lock(#[case] conversion: Conversion) {
    let dir = fresh_dir("oku-slides-view-off-lock-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let before = ch1(&s);
    let handed = s.data_link.handed();
    let mut at_retry = None;
    folded(&mut s, |s| conversion.run(s), |s| at_retry = Some(s.data_link.handed()))
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    // The retry uses the read done off the lock: it hands out no worker again.
    assert_that!(at_retry).is_equal_to(Some(handed + 1));
    assert_that!(s.data_link.handed()).is_equal_to(handed + 1);
    assert_that!(ch1(&s)).is_not_equal_to(before);
}

#[rstest]
#[case::chart_kind(Conversion::ChartKind)]
#[case::show_as(Conversion::ShowAs)]
fn view_conversion_on_base_refused_when_data_changes_during_read(#[case] conversion: Conversion) {
    let dir = fresh_dir("oku-slides-view-stale-");
    let mut s = session_on(dir.path());
    based(&mut s);
    let edit = op(json!({ "op": "edit_data", "data": "d9",
        "change": { "op": "set_cell", "row": "r1", "column": "c2", "value": 8 } }));
    let err = folded(
        &mut s,
        |s| conversion.run(s),
        |s| {
            s.apply("peer", vec![edit.clone()], None).unwrap_or_else(|e| panic!("{}", e.one_line()));
        },
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::BaseStale);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(fr.tr_with("session-data-base-stale", &[("data", "ventes".into())]));
    assert_that!(ch1(&s)["kind"]).is_equal_to(json!("base/chart"));
}
