use std::time::{Duration, Instant};

use speculoos::prelude::*;

use super::{CloseDecision, CloseTarget};
use crate::McpEvent;
use crate::hub::tests::hub;

fn decision() -> CloseDecision {
    CloseDecision { discard: true, revisions: vec![("t-2".to_owned(), Some(4))] }
}

fn tab_target() -> CloseTarget {
    CloseTarget::Tab { tab: "t-2".to_owned() }
}

#[test]
fn close_decision_redeemed_once() {
    let hub = hub();
    let id = hub.store_close(tab_target(), "w-1", decision());
    assert_that!(hub.send_close(id)).is_true();
    let id = id.to_string();
    assert_that!(hub.take_close_decision("w-1", &id)).is_equal_to(Some(decision()));
    assert_that!(hub.take_close_decision("w-1", &id)).is_none();
}

#[test]
fn close_decision_from_other_window_refused() {
    let hub = hub();
    let id = hub.store_close(tab_target(), "w-1", decision());
    assert_that!(hub.send_close(id)).is_true();
    let id = id.to_string();
    assert_that!(hub.take_close_decision("w-2", &id)).is_none();
    // The other window's attempt leaves it to its own window.
    assert_that!(hub.take_close_decision("w-1", &id)).is_equal_to(Some(decision()));
}

#[test]
fn close_decision_expired_refused() {
    let hub = hub();
    let now = Instant::now();
    let id = hub.store_close(tab_target(), "w-1", decision());
    assert_that!(hub.send_close_at(id, now)).is_true();
    let late = now + super::CLOSE_DECISION_TTL + Duration::from_secs(1);
    assert_that!(hub.take_close_decision_at("w-1", &id.to_string(), late)).is_none();
    assert_that!(hub.take_close_decision_at("w-1", &id.to_string(), now)).is_none();
}

#[test]
fn close_decision_pending_until_sent_cannot_be_taken() {
    let hub = hub();
    let id = hub.store_close(tab_target(), "w-1", decision());
    assert_that!(hub.take_close_decision("w-1", &id.to_string())).is_none();
    assert_that!(hub.close_decision(id)).is_equal_to(Some((tab_target(), decision())));
}

#[test]
fn close_decision_of_a_forged_id_is_none() {
    let hub = hub();
    assert_that!(hub.take_close_decision("w-1", "not-a-uuid")).is_none();
    assert_that!(hub.take_close_decision("w-1", &uuid::Uuid::new_v4().to_string())).is_none();
}

#[tokio::test]
async fn close_event_is_not_replayed() {
    let hub = hub();
    let mut rx = hub.subscribe();
    let id = hub.store_close(tab_target(), "w-1", decision());
    assert_that!(hub.send_close(id)).is_true();
    let sent = rx.try_recv().unwrap_or_else(|e| panic!("close event: {e}"));
    assert_that!(matches!(sent, McpEvent::Close { ref target, ref decision } if *target == tab_target() && *decision == id.to_string()))
        .is_true();
    assert_that!(hub.surfaces()).is_empty();
    hub.resync();
    let replayed = rx.try_recv().unwrap_or_else(|e| panic!("resync event: {e}"));
    assert_that!(matches!(replayed, McpEvent::Surfaces { ref list } if list.is_empty())).is_true();
    assert_that!(rx.try_recv().is_err()).is_true();
}

#[test]
fn close_event_serializes_its_target_and_decision() {
    let event = McpEvent::Close { target: CloseTarget::Window { window: "w-3".into() }, decision: "d".into() };
    let json = serde_json::to_value(&event).unwrap_or_else(|e| panic!("json: {e}"));
    assert_that!(json).is_equal_to(serde_json::json!({
        "kind": "close", "target": { "kind": "window", "window": "w-3" }, "decision": "d"
    }));
}

#[test]
fn store_close_past_kept_evicts_a_sent_decision_before_a_pending_one() {
    let hub = hub();
    let pending = hub.store_close(tab_target(), "w-1", decision());
    for _ in 0..super::KEPT {
        let sent = hub.store_close(tab_target(), "w-1", decision());
        assert_that!(hub.send_close(sent)).is_true();
    }
    assert_that!(hub.close_decision(pending)).is_some();
}
