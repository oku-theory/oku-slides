//! The local agent stands in for an unusable selected agent (t-1024): notice, retry, download, tune.
#![cfg(all(unix, debug_assertions))]
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::PoisonError;

use common::{Relayed, cmd, is_tool_surface, logged, stack_with_fallback, wait_for, wait_turn_done};
use oku_slides_i18n::Localizer;
use oku_slides_platform::local_agent::Target;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// A selected agent's program that does not exist.
const MISSING: &str = "/nonexistent/oku-slides-main-absent";

/// The local agent's turn: one receipt.
fn reporting() -> Value {
    json!({ "turns": [{ "mcp": [{ "tool": "report", "args": { "title": "Fait en local", "facts": ["Rien changé"] } }] }] })
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

/// Everything relayed for `t-1` on the `mcp` channel, as text.
fn thread(rx: &Relayed) -> String {
    rx.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter(|(ch, v)| *ch == "mcp" && v["tab"] == "t-1")
        .map(|(_, v)| v.to_string())
        .collect()
}

fn prompt(stack: &common::TestStack, text: &str) {
    stack
        .handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": text })))
        .unwrap_or_else(|e| panic!("prompt : {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn fallback_missing_main_program_runs_on_the_local_agent_until_its_report() {
    let (stack, rx, local_log) = stack_with_fallback(json!({ "turns": [{}] }), Some(MISSING), reporting(), true).await;
    prompt(&stack, "un");
    wait_for(&rx, |(ch, v)| *ch == "mcp" && is_tool_surface(v, "t-1", "report-")).await;
    assert_that!(logged(&local_log, "session/new")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn fallback_turn_shows_the_notice_with_the_main_reason() {
    let (stack, rx, _) = stack_with_fallback(json!({ "turns": [{}] }), Some(MISSING), reporting(), true).await;
    prompt(&stack, "un");
    wait_turn_done(&rx, "t-1", 1).await;
    let notice = fr().tr_with("agent-fallback-notice", &[("reason", "".into())]);
    let text = thread(&rx);
    assert_that!(text.contains(notice.trim_end())).is_true();
    assert_that!(text.contains(MISSING)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn fallback_turn_tells_the_tab_it_runs_on_the_local_agent() {
    let (stack, rx, _) = stack_with_fallback(json!({ "turns": [{}] }), Some(MISSING), reporting(), true).await;
    prompt(&stack, "un");
    let (_, told) =
        wait_for(&rx, |(ch, v)| *ch == "agent" && v["tab"] == "t-1" && v["event"]["kind"] == "fallback").await;
    assert_that!(told["event"]["reason"].as_str().is_some_and(|r| r.contains(MISSING))).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn fallback_launch_failure_at_connect_retries_on_the_local_agent() {
    // The selected agent exists but cannot reach MCP over HTTP: found only at `connect`.
    let (stack, rx, local_log) =
        stack_with_fallback(json!({ "http": false, "turns": [{}] }), None, reporting(), true).await;
    prompt(&stack, "un");
    wait_for(&rx, |(ch, v)| *ch == "mcp" && is_tool_surface(v, "t-1", "report-")).await;
    assert_that!(logged(&local_log, "session/new")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn fallback_download_declined_fails_the_turn_with_the_main_reason() {
    if Target::current() != Target::AppleSilicon {
        // Elsewhere the local agent is unsupported: nothing to download.
        return;
    }
    let (stack, rx, local_log) = stack_with_fallback(json!({ "turns": [{}] }), Some(MISSING), reporting(), false).await;
    prompt(&stack, "un");
    let (_, asked) = wait_for(&rx, |(ch, v)| *ch == "mcp" && is_tool_surface(v, "t-1", "confirm-")).await;
    let surface = asked["event"]["surface"]["id"].as_str().unwrap().to_owned();
    let no = json!({ "cmd": "surface_action", "tab": "t-1", "surface": surface, "name": "confirm",
        "context": { "confirmed": false } });
    stack.handle(cmd(no)).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let reason = fr().tr_with("agent-not-found", &[("command", MISSING.into())]);
    assert_that!(thread(&rx).contains(&reason)).is_true();
    assert_that!(logged(&local_log, "initialize")).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn fallback_tune_turn_is_accepted_while_the_local_agent_is_ready() {
    let (stack, rx, local_log) = stack_with_fallback(json!({ "turns": [{}] }), Some(MISSING), reporting(), true).await;
    tokio::task::block_in_place(|| stack.tune("t-1"));
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(logged(&local_log, "session/prompt")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn fallback_availability_with_a_ready_local_agent_reports_none() {
    let (stack, rx, _) = stack_with_fallback(json!({ "turns": [] }), Some(MISSING), reporting(), true).await;
    let from = common::relayed_len(&rx);
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "availability", "tab": "t-1" })))).unwrap();
    let is_availability =
        |(ch, v): &(&'static str, Value)| *ch == "agent" && v["tab"] == "t-1" && v["event"]["kind"] == "availability";
    let told = common::wait_n_after(&rx, from, 1, is_availability).await;
    assert_that!(told[0].1.1["event"]["unusable"].clone()).is_equal_to(Value::Null);
}
