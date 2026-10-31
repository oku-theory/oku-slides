//! Each turn's usage lands in the tab's deck: tokens, model, preset and cost share.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::PoisonError;
use std::time::Duration;

use common::*;
use oku_slides_agent::AgentEvent;
use oku_slides_core::{UsageCost, UsageRecord, UsageTab, UsageTokens};
use oku_slides_session::agent::AgentUi;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// The deck's records once it holds `n` (recording follows the turn's end; 5 s at most).
async fn records(h: &Harness, n: usize) -> Vec<UsageRecord> {
    for _ in 0..250 {
        let got: Vec<UsageRecord> = h
            .session("t-1")
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .usage_ledger()
            .map(|l| l.records().cloned().collect())
            .unwrap_or_default();
        if got.len() >= n {
            return got;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("{n} usage records expected");
}

/// Plays the scenario's turns on `t-1`, one prompt each, then returns the deck's records.
async fn played(script: Value, prompts: usize) -> Vec<UsageRecord> {
    let h = start(script).await;
    play_turns(&h, prompts).await;
    records(&h, prompts).await
}

async fn play_turns(h: &Harness, prompts: usize) {
    let t = h.tab("t-1");
    for i in 0..prompts {
        t.agent.prompt(format!("consigne {i}"), AgentUi::default());
        wait_turn_ended(t).await;
    }
}

fn usd(amount: f64) -> UsageCost {
    UsageCost { amount, currency: "USD".into() }
}

#[tokio::test(flavor = "multi_thread")]
async fn turn_with_usage_and_cost_records_them_with_preset_and_model() {
    let script = json!({
        "models": [{ "id": "opus-test", "name": "Opus" }],
        "turns": [{ "usage": [30, 20, 10], "cost": { "amount": 0.5, "currency": "USD" } }],
    });
    let r = played(script, 1).await;
    assert_that!(r).has_length(1);
    assert_that!(r[0].tab).is_equal_to(UsageTab::Deck);
    assert_that!(r[0].agent.as_str()).is_equal_to("fake");
    assert_that!(r[0].model.as_deref()).is_equal_to(Some("opus-test"));
    let tokens = UsageTokens { total: 30, input: 20, output: 10, thought: None, cache_read: None, cache_write: None };
    assert_that!(r[0].tokens).is_equal_to(Some(tokens));
    assert_that!(r[0].cost.clone()).is_equal_to(Some(usd(0.5)));
}

#[tokio::test(flavor = "multi_thread")]
async fn turn_without_usage_records_unknown_tokens_cost_and_model() {
    let r = played(json!({ "turns": [{ "chunks": ["ok"] }] }), 1).await;
    assert_that!(r).has_length(1);
    assert_that!(r[0].tokens).is_none();
    assert_that!(r[0].cost.clone()).is_none();
    assert_that!(r[0].model.clone()).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn two_turns_with_growing_cost_record_each_turn_share() {
    let script = json!({ "turns": [
        { "cost": { "amount": 0.5, "currency": "USD" } },
        { "cost": { "amount": 0.75, "currency": "USD" } },
    ] });
    let r = played(script, 2).await;
    let costs: Vec<_> = r.iter().map(|r| r.cost.clone()).collect();
    assert_that!(costs).is_equal_to(vec![Some(usd(0.5)), Some(usd(0.25))]);
}

#[tokio::test(flavor = "multi_thread")]
async fn each_turn_gets_its_own_turn_id() {
    let r = played(json!({ "turns": [{}] }), 2).await;
    assert_that!(r[0].turn).is_not_equal_to(r[1].turn);
}

#[tokio::test(flavor = "multi_thread")]
async fn resumed_session_first_cost_is_only_the_base() {
    let script = json!({ "resume": true, "turns": [
        { "cost": { "amount": 2.0, "currency": "USD" } },
        { "cost": { "amount": 2.5, "currency": "USD" } },
    ] });
    let h = start(script).await;
    seed_session(&h, "sess-42", &h.link.agent_id()).await;
    play_turns(&h, 2).await;
    let costs: Vec<_> = records(&h, 2).await.iter().map(|r| r.cost.clone()).collect();
    assert_that!(logged(&h.log, "session/resume")).has_length(1);
    assert_that!(costs).is_equal_to(vec![None, Some(usd(0.5))]);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelled_turn_is_recorded() {
    let h = start(json!({ "turns": [{ "wait_cancel": true, "usage": [3, 2, 1] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("long".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.cancel();
    let e = until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    assert_that!(e).is_equal_to(AgentEvent::TurnEnded { turn: 1, stop: "cancelled".into() });
    assert_that!(records(&h, 1).await[0].tokens.map(|t| t.total)).is_equal_to(Some(3));
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_turn_records_nothing() {
    let h = start(json!({ "turns": [{ "error": "boom" }, { "usage": [3, 2, 1] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    wait_error(&mut rx).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(t).await;
    // Only the second turn, the one with a reply.
    let r = records(&h, 1).await;
    assert_that!(r).has_length(1);
    assert_that!(r[0].tokens.map(|t| t.total)).is_equal_to(Some(3));
}
