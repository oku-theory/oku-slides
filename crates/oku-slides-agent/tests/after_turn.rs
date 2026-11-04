//! App actions after a turn: an install runs once the reply and its receipt are in, never after a
//! stop or a failure (t-999).
//!
//! A negative check ends on a sentinel, never on a sleep: the after-turn thread runs its queue in
//! order, so once a later turn's install (refused on purpose, with a notice) has run, every install
//! queued before it has too.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use std::time::Duration;

use common::{Relayed, TestStack, cmd, is_tool_surface, logged, stack_with_sink_and_log, wait_for, wait_turn_done};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// `fr`: agent-error-app-failed, the notice of the sentinel's refused install.
const FAILED: &str = "La mise à jour d'Oku Slides a échoué";

fn install() -> Value {
    json!({ "tool": "update", "args": { "action": "install" } })
}

/// The scripts of these tests, by prompt; "sentinelle" is the sentinel's turn.
fn script(by_prompt: &Value) -> Value {
    let mut all = json!({ "sentinelle": { "mcp": [install()] } });
    if let (Some(all), Some(more)) = (all.as_object_mut(), by_prompt.as_object()) {
        all.extend(more.clone());
    }
    json!({ "by_prompt": all })
}

fn prompt(stack: &TestStack, text: &str) {
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": text }))).unwrap_or_else(|e| panic!("prompt: {e}"));
}

/// Clicks yes on the `nth` confirmation (1 = the first) shown in `t-1`.
async fn confirm(stack: &TestStack, rx: &Relayed, nth: usize) {
    let mut seen = Vec::new();
    while seen.len() < nth {
        let asked = wait_for(rx, |(_, v)| {
            is_tool_surface(v, "t-1", "confirm-")
                && v["event"]["surface"]["id"].as_str().is_some_and(|id| !seen.iter().any(|s: &String| s == id))
        })
        .await;
        seen.push(asked.1["event"]["surface"]["id"].as_str().unwrap_or_else(|| panic!("confirm id")).to_owned());
    }
    let id = &seen[nth - 1];
    let yes = json!({ "cmd": "surface_action", "tab": "t-1", "surface": id, "name": "confirm", "context": { "confirmed": true } });
    stack.handle(cmd(yes)).unwrap_or_else(|e| panic!("confirm: {e}"));
}

/// Whether `v` is the refused-install notice relayed for `t-1`.
fn is_failed_notice(v: &Value) -> bool {
    is_tool_surface(v, "t-1", "note-") && v["event"]["surface"].to_string().contains(FAILED)
}

/// Where the first relayed event matching `pred` is.
fn position(rx: &Relayed, pred: impl Fn(&Value) -> bool) -> Option<usize> {
    rx.lock().unwrap_or_else(|e| panic!("relay: {e}")).iter().position(|(_, v)| pred(v))
}

/// Plays the sentinel's turn (its confirmation is the `nth`) and waits until its install was refused.
async fn sentinel(stack: &TestStack, rx: &Relayed, nth: usize) {
    stack.app.fail_next_install();
    prompt(stack, "sentinelle");
    confirm(stack, rx, nth).await;
    wait_for(rx, |(_, v)| is_failed_notice(v)).await;
}

/// Waits until `stack`'s app ran `n` installs, 10 s at most.
async fn installs_reach(stack: &TestStack, n: usize) {
    let reached = tokio::time::timeout(Duration::from_secs(10), async {
        while stack.app.installs() < n {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert_that!(reached).is_ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_install_runs_once_the_reply_is_in() {
    let installe = json!({ "installe": { "mcp": [install()], "chunks": ["Je l'installe."] } });
    let (stack, rx, log) = stack_with_sink_and_log(script(&installe), &["t-1"]).await;
    stack.app.offer("5.2.0");
    prompt(&stack, "installe");
    confirm(&stack, &rx, 1).await;
    wait_turn_done(&rx, "t-1", 1).await;
    installs_reach(&stack, 1).await;
    let out = logged(&log, "mcp/result")[0]["out"].to_string();
    assert_that!(out.contains("after_turn")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_install_follows_the_turns_receipt() {
    let installe = json!({ "installe": { "mcp": [install()], "chunks": ["Je l'installe."] } });
    let (stack, rx, _log) = stack_with_sink_and_log(script(&installe), &["t-1"]).await;
    stack.app.offer("5.2.0");
    // The install is refused: its notice goes through the hub's one event stream, as the receipt.
    stack.app.fail_next_install();
    prompt(&stack, "installe");
    confirm(&stack, &rx, 1).await;
    wait_for(&rx, |(_, v)| is_failed_notice(v)).await;
    let receipt = position(&rx, |v| is_tool_surface(v, "t-1", "report-"));
    let notice = position(&rx, is_failed_notice);
    assert_that!(receipt.is_some()).is_true();
    assert_that!(receipt < notice).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_dropped_on_stop() {
    let attends = json!({ "installe puis attends": { "mcp": [install()], "wait_cancel": true } });
    let (stack, rx, log) = stack_with_sink_and_log(script(&attends), &["t-1"]).await;
    stack.app.offer("5.2.0");
    prompt(&stack, "installe puis attends");
    confirm(&stack, &rx, 1).await;
    wait_for(&rx, |_| !logged(&log, "mcp/result").is_empty()).await;
    stack.handle(cmd(json!({ "cmd": "cancel", "tab": "t-1" }))).unwrap_or_else(|e| panic!("cancel: {e}"));
    wait_turn_done(&rx, "t-1", 1).await;
    sentinel(&stack, &rx, 2).await;
    assert_that!(stack.app.installs()).is_equal_to(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_dropped_when_the_turn_fails() {
    let plante = json!({ "installe et plante": { "mcp": [install()], "crash": true } });
    let (stack, rx, log) = stack_with_sink_and_log(script(&plante), &["t-1"]).await;
    stack.app.offer("5.2.0");
    prompt(&stack, "installe et plante");
    confirm(&stack, &rx, 1).await;
    wait_for(&rx, |_| !logged(&log, "mcp/result").is_empty()).await;
    // The agent dies before its reply: the turn fails. The actor takes the sentinel's prompt only
    // after that failure, and starts the agent again for it.
    sentinel(&stack, &rx, 2).await;
    assert_that!(stack.app.installs()).is_equal_to(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_second_install_refused() {
    let deux = json!({ "installe deux fois": { "mcp": [install(), install()] } });
    let (stack, rx, log) = stack_with_sink_and_log(script(&deux), &["t-1"]).await;
    stack.app.offer("5.2.0");
    prompt(&stack, "installe deux fois");
    confirm(&stack, &rx, 1).await;
    wait_turn_done(&rx, "t-1", 1).await;
    let outs: Vec<String> = logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!(outs).has_length(2);
    // `fr`: agent-error-after-turn-taken.
    assert_that!(outs[1].contains("Cette réponse se termine déjà")).is_true();
    installs_reach(&stack, 1).await;
    sentinel(&stack, &rx, 2).await;
    assert_that!(stack.app.installs()).is_equal_to(1);
}
