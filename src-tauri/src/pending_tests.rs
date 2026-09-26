use std::path::PathBuf;

use speculoos::prelude::*;

use super::*;

fn pb(name: &str) -> PathBuf {
    PathBuf::from(name)
}

#[test]
fn pending_receive_before_ready_queues_the_files() {
    let pending = Pending::default();
    let handed = pending.receive(vec![pb("a.pdf")]);
    assert_that!(handed).is_none();
}

#[test]
fn pending_ready_returns_queued_files_in_order() {
    let pending = Pending::default();
    pending.receive(vec![pb("a.pdf")]);
    pending.receive(vec![pb("b.pdf")]);
    assert_that!(pending.ready()).is_equal_to(vec![pb("a.pdf"), pb("b.pdf")]);
}

#[test]
fn pending_receive_after_ready_hands_the_files_back() {
    let pending = Pending::default();
    pending.ready();
    let handed = pending.receive(vec![pb("a.pdf")]);
    assert_that!(handed).is_equal_to(Some(vec![pb("a.pdf")]));
}

#[test]
fn pending_ready_called_twice_returns_an_empty_list_the_second_time() {
    let pending = Pending::default();
    pending.receive(vec![pb("a.pdf")]);
    pending.ready();
    assert_that!(pending.ready()).is_empty();
}

#[test]
fn pending_receive_caps_at_max_received_and_drops_the_rest() {
    let pending = Pending::default();
    let files: Vec<PathBuf> = (0..(MAX_RECEIVED + 6)).map(|i| pb(&format!("{i}.pdf"))).collect();
    pending.receive(files);
    assert_that!(pending.ready()).has_length(MAX_RECEIVED);
}
