//! t-1779: a `session/cancel` that reached the fake agent during a turn that does not wait for one
//! stayed stored, and the next `wait_cancel` turn of that session ended "cancelled" at once. The
//! hands-free "stop" then found no turn left to cancel (e2e 340), whenever an earlier spec's turn
//! was still running when the next command arrived.

use std::time::Duration;

use oku_slides_agent::AgentEvent;
use oku_slides_session::agent::AgentUi;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{logged, start, until};

fn ended(e: &AgentEvent) -> bool {
    matches!(e, AgentEvent::TurnEnded { .. })
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_of_an_ended_turn_never_cancels_the_next_one() {
    let h = start(json!({ "turns": [{ "delay_ms": 800 }, { "wait_cancel": true }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("lent".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.cancel();
    until(&mut rx, ended).await;
    assert_that!(logged(&h.log, "session/cancel")).has_length(1);
    t.agent.prompt("attends".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    let early = tokio::time::timeout(Duration::from_millis(1500), until(&mut rx, ended)).await;
    asserting("the waiting turn must still run").that(&early.is_err()).is_true();
    t.agent.cancel();
    let e = until(&mut rx, ended).await;
    assert_that!(e).is_equal_to(AgentEvent::TurnEnded { turn: 2, stop: "cancelled".into() });
}
