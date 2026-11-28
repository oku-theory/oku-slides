use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{OpError, OpErrorKind};
use oku_slides_data::{DataCell, DataError, DataErrorKind};
use oku_slides_i18n::{Localize, Localizer};
use serde_json::json;
use speculoos::prelude::*;

use super::QueryStatus;
use crate::derived::derived_tests::{TOP, apply, id, with_query};

/// The kind of the op error a `Prepare` session error wraps.
fn op_kind(err: &crate::SessionError) -> Option<OpErrorKind> {
    std::error::Error::source(err).and_then(|e| e.downcast_ref::<OpError>()).map(OpError::kind)
}

/// The cells of the rows a read returned, its header row left out.
fn cells(dataset: &oku_slides_data::Dataset) -> Vec<Vec<DataCell>> {
    dataset.source.iter().skip(1).cloned().collect()
}

#[test]
fn query_preview_reads_rows_without_storing_a_result() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    let deck = s.deck().clone();
    let states = s.derived.states.len();
    let bases: Vec<_> = s.derived.bases().collect();

    let preview = s.query_preview("SELECT a FROM ventes ORDER BY a", 10, 0..10, 100).unwrap_or_else(|e| panic!("{e}"));
    let (data, dataset) = preview.run().unwrap_or_else(|e| panic!("{e}"));

    assert_that!(cells(&dataset)).is_equal_to(vec![
        vec![DataCell::Text("x".into())],
        vec![DataCell::Text("y".into())],
        vec![DataCell::Text("z".into())],
    ]);
    assert_that!(data.table().map(|t| t.rows.len())).is_equal_to(Some(3));
    assert_that!(*s.deck()).is_equal_to(&deck);
    assert_that!(s.derived.states.len()).is_equal_to(states);
    assert_that!(s.derived.bases().collect::<Vec<_>>()).is_equal_to(bases);
}

#[test]
fn query_preview_reads_a_ready_query_input() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();

    let preview = s.query_preview("SELECT a FROM top", 10, 0..10, 100).unwrap_or_else(|e| panic!("{e}"));
    let (_, dataset) = preview.run().unwrap_or_else(|e| panic!("{e}"));

    assert_that!(cells(&dataset)).is_equal_to(vec![vec![DataCell::Text("x".into())]]);
}

#[test]
fn query_preview_pending_input_is_refused_fr() {
    let (s, _dir) = with_query(TOP);

    let err = s.query_preview("SELECT a FROM top", 10, 0..10, 100).map(|_| ()).unwrap_err();

    let fr = Localizer::new(Some("fr"));
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::QueryInputPending);
    assert_that!(err.localize(&fr))
        .is_equal_to(fr.tr_with("session-data-query-input-pending", &[("name", "top".into())]));
    assert_that!(err.localize(&fr)).contains("« top »");
}

#[test]
fn query_preview_unknown_input_is_refused() {
    let (s, _dir) = with_query(TOP);

    let err = s.query_preview("SELECT a FROM absente", 10, 0..10, 100).map(|_| ()).unwrap_err();

    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(OpError::data_unknown_input("absente").localize(&fr));
}

#[test]
fn query_preview_run_failure_is_a_data_error() {
    let (s, _dir) = with_query(TOP);

    let preview = s.query_preview("SELECT zzz FROM ventes", 10, 0..10, 100).unwrap_or_else(|e| panic!("{e}"));
    let err = preview.run().map(|_| ()).unwrap_err();

    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::Op);
    assert_that!(op_kind(&err)).is_equal_to(Some(OpErrorKind::Data(DataErrorKind::QueryRun)));
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.is_empty()).is_false();
    assert_that!(fr).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
}

#[test]
fn query_preview_input_missing_from_the_statuses_is_failed_not_pending_fr() {
    let (s, _dir) = with_query(TOP);
    let data = Arc::clone(&s.deck().data[&id("q")]);

    let err = super::input(&id("q"), &data, &BTreeMap::new()).map(|_| ()).unwrap_err();

    assert_that!(op_kind(&err)).is_equal_to(Some(OpErrorKind::Data(DataErrorKind::QueryInputFailed)));
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(OpError::from(DataError::query_input_failed("top")).localize(&fr));
}

#[test]
fn data_rows_of_a_ready_query_reads_its_result() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();

    let (data, read) =
        s.data_rows("q", 10, 0..10, 100).unwrap_or_else(|| panic!("q missing")).unwrap_or_else(|e| panic!("{e}"));
    let dataset = read.run().unwrap_or_else(|e| panic!("{e}"));

    assert_that!(data.table().map(|t| t.rows.len())).is_equal_to(Some(1));
    assert_that!(cells(&dataset)).is_equal_to(vec![vec![DataCell::Text("x".into()), DataCell::Number(1.0)]]);
}

#[test]
fn data_rows_of_a_failed_query_is_refused_fr() {
    let (mut s, _dir) = with_query("SELECT zzz FROM ventes");
    s.settle_queries();

    let err = s.data_rows("q", 10, 0..10, 100).unwrap_or_else(|| panic!("q missing")).map(|_| ()).unwrap_err();

    let fr = Localizer::new(Some("fr"));
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::QueryReadNoResult);
    assert_that!(err.localize(&fr))
        .is_equal_to(fr.tr_with("session-data-query-read-no-result", &[("data", "top".into())]));
    assert_that!(err.localize(&fr)).contains("« top »");
}

#[test]
fn query_statuses_follow_the_current_inputs() {
    let (mut s, _dir) = with_query(TOP);
    apply(&mut s, vec![json!({ "op": "add_data", "id": "f", "name": "rate", "sql": "SELECT zzz FROM ventes" })]);
    assert_that!(matches!(s.query_statuses().get(&id("q")), Some(QueryStatus::Pending))).is_true();

    s.settle_queries();
    let statuses = s.query_statuses();

    let Some(QueryStatus::Ready(table)) = statuses.get(&id("q")) else { panic!("q not ready: {statuses:?}") };
    assert_that!(table.rows.len()).is_equal_to(1);
    let Some(QueryStatus::Failed(message)) = statuses.get(&id("f")) else { panic!("f not failed: {statuses:?}") };
    assert_that!(message.id()).is_equal_to("data-query-run");
    assert_that!(statuses.contains_key(&id("m"))).is_false();
}
