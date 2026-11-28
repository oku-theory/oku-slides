//! t-1779: a question the agent asked just after the user stopped its turn (the tool call was
//! still on its way when `cancel_asks` ran) waited for an answer forever. The `session/prompt`
//! never returned, the turn never ended, and its undo group swallowed the user's next edits.

use std::time::Duration;

use serde_json::json;
use speculoos::prelude::*;

use crate::common::{call, client, json_of, next_surface, start};

#[tokio::test(flavor = "multi_thread")]
async fn ask_after_cut_turn_is_cancelled_at_once() {
    let h = start().await;
    h.hub.begin_turn();
    h.hub.cut_turn();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let asked =
        tokio::time::timeout(Duration::from_secs(5), call(&c, "ask_confirm", json!({ "message": "Sûr ?" }))).await;
    let reply = asked.unwrap_or_else(|_| panic!("the question of a stopped turn waits forever"));
    assert_that!(json_of(&reply)).is_equal_to(json!({ "cancelled": true }));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_in_the_next_turn_after_a_cut_waits_for_its_answer() {
    let h = start().await;
    h.hub.begin_turn();
    h.hub.cut_turn();
    h.hub.begin_turn();
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_confirm", json!({ "message": "Sûr ?" })).await }
    });
    next_surface(&mut rx).await;
    assert_that!(h.hub.has_pending_ask()).is_true();
    h.hub.cancel_asks();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "cancelled": true }));
}
