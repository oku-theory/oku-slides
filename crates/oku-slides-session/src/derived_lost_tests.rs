//! A lost query worker (t-1794): its query computes again, a bounded number of times.

use oku_slides_data::{DataError, DataErrorKind};
use oku_slides_render::RenderError;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::derived_tests::{TOP, apply, id, rows, with_query};
use super::{QUERY_ATTEMPTS, QueryReply, QueryState};
use crate::Session;

/// Lands each job taken now as lost to its worker; returns, per job, whether its state changed.
fn lose_jobs(s: &mut Session) -> Vec<bool> {
    let jobs = s.query_jobs();
    jobs.into_iter().map(|job| s.finish_query(job, QueryReply::Lost(DataError::query_run("pipe closed")))).collect()
}

#[test]
fn lost_worker_query_computes_again_then_lands_ready() {
    let (mut s, _dir) = with_query(TOP);

    // Still pending: nothing for the host to push.
    assert_that!(lose_jobs(&mut s)).is_equal_to(vec![false]);
    assert_that!(matches!(s.query_state(&id("q")), Some(QueryState::Pending))).is_true();
    s.settle_queries();

    assert_that!(rows(&s, "q")).is_equal_to(1);
    assert_that!(s.query_issues()).is_empty();
}

#[test]
fn lost_worker_query_fails_after_bounded_attempts() {
    let (mut s, _dir) = with_query(TOP);

    for _ in 1..QUERY_ATTEMPTS {
        assert_that!(lose_jobs(&mut s)).is_equal_to(vec![false]);
    }
    // The last run lost fails the query: its views change.
    assert_that!(lose_jobs(&mut s)).is_equal_to(vec![true]);

    match s.query_state(&id("q")) {
        Some(QueryState::Failed { error, .. }) => assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryRun),
        other => panic!("q not failed: {other:?}"),
    }
    assert_that!(s.query_jobs()).is_empty();
    assert_that!(s.query_issues()).has_length(1);
}

#[test]
fn lost_worker_attempts_restart_for_new_inputs() {
    let (mut s, _dir) = with_query(TOP);
    for _ in 1..QUERY_ATTEMPTS {
        lose_jobs(&mut s);
    }
    apply(&mut s, vec![super::derived_tests::set_b(1, 5.0)]);

    // New inputs, a new key: the runs lost for the old one no longer count.
    lose_jobs(&mut s);
    assert_that!(matches!(s.query_state(&id("q")), Some(QueryState::Pending))).is_true();
    s.settle_queries();
    assert_that!(rows(&s, "q")).is_equal_to(2);
}

#[test]
fn lost_worker_input_keeps_its_reader_pending_then_computes_it() {
    let (mut s, _dir) = with_query(TOP);
    apply(&mut s, vec![json!({ "op": "add_data", "id": "a2", "name": "big", "sql": "SELECT a FROM top" })]);

    lose_jobs(&mut s);

    assert_that!(matches!(s.query_state(&id("a2")), Some(QueryState::Pending))).is_true();
    s.settle_queries();
    assert_that!(rows(&s, "a2")).is_equal_to(1);
}

#[rstest]
#[case::worker_lost(Err(RenderError::worker_lost("pipe closed")), true)]
#[case::stale(Err(RenderError::stale()), true)]
#[case::hello_timeout(Err(RenderError::timeout(10)), true)]
#[case::query_timeout(Ok(Err(DataError::query_timeout(10))), true)]
#[case::out_of_memory(Err(RenderError::out_of_memory(64)), false)]
#[case::refused(Ok(Err(DataError::query_run("no such column"))), false)]
fn query_reply_of_tells_a_lost_worker_from_a_refusal(
    #[case] run: Result<Result<oku_slides_render_worker::QueryOut, DataError>, RenderError>,
    #[case] lost: bool,
) {
    let reply = QueryReply::of(run);

    assert_that!(matches!(reply, QueryReply::Lost(_))).is_equal_to(lost);
    assert_that!(matches!(reply, QueryReply::Refused(_))).is_equal_to(!lost);
}
