use rstest::rstest;
use speculoos::prelude::*;

use super::{QuitBarrier, QuitStart};

fn waiting_for(labels: &[&str]) -> QuitBarrier {
    let mut barrier = QuitBarrier::default();
    let start = barrier.start(labels.iter().map(|l| (*l).to_owned()));
    assert_that!(start).is_equal_to(QuitStart::Wait);
    barrier
}

#[test]
fn quit_barrier_without_windows_is_ready_at_once() {
    let mut barrier = QuitBarrier::default();
    assert_that!(barrier.start(Vec::new())).is_equal_to(QuitStart::Ready);
}

#[test]
fn quit_barrier_second_start_joins_the_first() {
    let mut barrier = waiting_for(&["w-1"]);
    assert_that!(barrier.start(vec!["w-1".to_owned(), "w-2".to_owned()])).is_equal_to(QuitStart::Joined);
}

#[test]
fn quit_barrier_joining_start_keeps_the_windows_waited_for() {
    let mut barrier = waiting_for(&["w-1"]);
    barrier.start(vec!["w-1".to_owned(), "w-2".to_owned()]);
    assert_that!(barrier.answer("w-1")).is_true();
}

#[rstest]
#[case::completed(true)]
#[case::without_windows(false)]
fn quit_barrier_start_after_the_wait_ended_joins(#[case] with_windows: bool) {
    let mut barrier = QuitBarrier::default();
    if with_windows {
        barrier.start(vec!["w-1".to_owned()]);
        barrier.answer("w-1");
    } else {
        barrier.start(Vec::new());
    }
    assert_that!(barrier.start(vec!["w-1".to_owned()])).is_equal_to(QuitStart::Joined);
}

#[test]
fn quit_barrier_pending_lists_the_windows_not_answered() {
    let mut barrier = waiting_for(&["w-1", "agent", "w-2"]);
    barrier.answer("w-1");
    assert_that!(barrier.pending()).is_equal_to(vec!["agent".to_owned(), "w-2".to_owned()]);
}

#[test]
fn quit_barrier_pending_with_no_quit_waiting_is_empty() {
    assert_that!(QuitBarrier::default().pending()).is_empty();
}

#[test]
fn quit_barrier_last_answer_completes_it() {
    let mut barrier = waiting_for(&["w-1", "agent"]);
    assert_that!(barrier.answer("w-1")).is_false();
    assert_that!(barrier.answer("agent")).is_true();
}

#[rstest]
#[case::unknown_window("w-9")]
#[case::duplicate("w-1")]
fn quit_barrier_answer_from_a_window_not_waited_for_is_ignored(#[case] label: &str) {
    let mut barrier = waiting_for(&["w-1", "w-2"]);
    barrier.answer("w-1");
    assert_that!(barrier.answer(label)).is_false();
    assert_that!(barrier.answer("w-2")).is_true();
}

#[test]
fn quit_barrier_answer_with_no_quit_underway_is_ignored() {
    let mut barrier = QuitBarrier::default();
    assert_that!(barrier.answer("w-1")).is_false();
}

#[test]
fn quit_barrier_completes_only_once() {
    let mut barrier = waiting_for(&["w-1"]);
    assert_that!(barrier.answer("w-1")).is_true();
    assert_that!(barrier.answer("w-1")).is_false();
}

#[test]
fn quit_barrier_after_release_ignores_late_answers() {
    let mut barrier = waiting_for(&["w-1"]);
    barrier.release();
    assert_that!(barrier.answer("w-1")).is_false();
}

#[test]
fn quit_barrier_after_release_joins_every_later_start() {
    let mut barrier = waiting_for(&["w-1"]);
    barrier.release();
    assert_that!(barrier.start(Vec::new())).is_equal_to(QuitStart::Joined);
}
