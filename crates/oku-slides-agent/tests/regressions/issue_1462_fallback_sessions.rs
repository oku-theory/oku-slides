//! t-1462: during a fallback, the local agent's session overwrote the selected agent's: back
//! on it, a new session opened silently. The fallback's start showed no `Starting`, and the
//! fallback badge stayed once the user chose an agent.
#![cfg(all(unix, debug_assertions))]

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{Relayed, TestStack, cmd, logged, relayed_len, stack_with_fallback, wait_n_after, wait_turn_done};

/// The local agent's script: plain turns.
fn local_turns() -> Value {
    json!({ "turns": [{ "chunks": ["en local"] }] })
}

/// A link at `at` to the fake agent: the selected agent's program, removed and put back.
fn link_main(at: &Path) {
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_oku-slides-fake-agent"), at)
        .unwrap_or_else(|e| panic!("lien vers le faux agent : {e}"));
}

fn prompt(stack: &TestStack, text: &str) {
    stack
        .handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": text })))
        .unwrap_or_else(|e| panic!("prompt : {e}"));
}

/// A stack whose selected agent runs through the link `main` (in `dir`), resuming sessions.
async fn stack(dir: &Path) -> (TestStack, Relayed, PathBuf, PathBuf, PathBuf) {
    let main = dir.join("main-agent");
    link_main(&main);
    let script = json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] });
    let (stack, rx, local_log) = stack_with_fallback(script, main.to_str(), local_turns(), true).await;
    let main_log = local_log.with_file_name("log.jsonl");
    (stack, rx, main, main_log, local_log)
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1462_back_on_the_selected_agent_resumes_its_session() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("dossier : {e}"));
    let (stack, rx, main, main_log, _) = stack(dir.path()).await;
    prompt(&stack, "un");
    wait_turn_done(&rx, "t-1", 1).await;
    std::fs::remove_file(&main).unwrap_or_else(|e| panic!("lien retiré : {e}"));
    prompt(&stack, "deux");
    wait_turn_done(&rx, "t-1", 2).await;
    link_main(&main);
    prompt(&stack, "trois");
    wait_turn_done(&rx, "t-1", 3).await;
    let opened = logged(&main_log, "session/new");
    asserting("one session opened on the selected agent").that(&opened).has_length(1);
    let resumed = logged(&main_log, "session/resume");
    asserting("its session resumed after the fallback").that(&resumed).has_length(1);
    let prompted: Vec<Value> = logged(&main_log, "session/prompt").iter().map(|p| p["sessionId"].clone()).collect();
    asserting("both turns on the selected agent prompt its session")
        .that(&prompted)
        .is_equal_to(vec![resumed[0]["sessionId"].clone(); 2]);
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1462_route_switching_the_link_shows_starting() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("dossier : {e}"));
    let (stack, rx, main, _, _) = stack(dir.path()).await;
    prompt(&stack, "un");
    wait_turn_done(&rx, "t-1", 1).await;
    std::fs::remove_file(&main).unwrap_or_else(|e| panic!("lien retiré : {e}"));
    let from = relayed_len(&rx);
    prompt(&stack, "deux");
    let starting = |(ch, v): &(&'static str, Value)| {
        *ch == "agent" && v["tab"] == "t-1" && v["event"]["kind"] == "status" && v["event"]["state"] == "starting"
    };
    let seen = wait_n_after(&rx, from, 1, starting).await;
    assert_that!(seen).is_not_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1462_choosing_the_running_local_agent_clears_the_badge_without_relaunch() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("dossier : {e}"));
    let (stack, rx, main, _, local_log) = stack(dir.path()).await;
    std::fs::remove_file(&main).unwrap_or_else(|e| panic!("lien retiré : {e}"));
    prompt(&stack, "un");
    wait_turn_done(&rx, "t-1", 1).await;
    let from = relayed_len(&rx);
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" }))))
        .unwrap_or_else(|e| panic!("réglages : {e}"));
    let click = json!({ "cmd": "surface_action", "tab": "t-1", "surface": "host-voice", "name": "select", "context": { "id": "local" } });
    tokio::task::block_in_place(|| stack.handle(cmd(click))).unwrap_or_else(|e| panic!("choix : {e}"));
    let cleared = |(ch, v): &(&'static str, Value)| {
        *ch == "agent" && v["tab"] == "t-1" && v["event"]["kind"] == "fallback" && v["event"]["reason"].is_null()
    };
    wait_n_after(&rx, from, 1, cleared).await;
    prompt(&stack, "deux");
    wait_turn_done(&rx, "t-1", 2).await;
    asserting("the local agent kept running").that(&logged(&local_log, "initialize")).has_length(1);
}
