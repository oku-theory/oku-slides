mod common;

use std::sync::Arc;

use common::{is_tool_surface, stack_with_sink, stack_with_sink_and_log, wait_for, wait_turn_done};
use oku_slides_agent::AgentCommand;
use serde_json::json;
use speculoos::prelude::*;

fn cmd(v: serde_json::Value) -> AgentCommand {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("commande : {e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn forward_wraps_every_event_with_its_tab() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1", "t-2"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "prompt", "tab": "t-2", "text": "salut" })).unwrap()).unwrap();
    let surface = wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "turn-1"
    })
    .await;
    assert_that!(surface.1["tab"].clone()).is_equal_to(json!("t-2"));
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_replays_the_agent_state_then_the_turn_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" })).unwrap()).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    // L'état `idle` de fin de tour doit être relayé avant de prendre la mesure.
    wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "status" && v["event"]["state"] == "idle").await;
    let before = rx.lock().unwrap().len();
    stack.handle(serde_json::from_value(json!({ "cmd": "resync", "tab": "t-1" })).unwrap()).unwrap();
    let surfaces = wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surfaces" && v["tab"] == "t-1").await;
    let ids: Vec<_> = surfaces.1["event"]["list"].as_array().unwrap().iter().map(|s| s["id"].clone()).collect();
    assert_that!(ids).contains(json!("turn-1"));
    // L'état de l'agent d'abord, puis les surfaces.
    let relayed = rx.lock().unwrap()[before..].to_vec();
    let at = |ch: &str, kind: &str| relayed.iter().position(|(c, v)| *c == ch && v["event"]["kind"] == kind).unwrap();
    assert_that!(at("agent", "status")).is_less_than(at("mcp", "surfaces"));
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_of_an_unused_tab_sends_idle_and_no_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "resync", "tab": "t-1" })).unwrap()).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "status" && v["event"]["state"] == "idle").await;
    let surfaces = wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surfaces").await;
    assert_that!(surfaces.1["event"]["list"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn tab_closed_during_a_turn_releases_its_questions_and_other_tabs_go_on() {
    // Premier tour de chaque session : une question ; t-1 la laisse ouverte, t-2 y répond.
    let script = json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?",
        "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }], "chunks": ["fini"] }] });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1", "t-2"]).await;
    let prompt = |tab: &str| serde_json::from_value(json!({ "cmd": "prompt", "tab": tab, "text": "?" })).unwrap();
    stack.handle(prompt("t-1")).unwrap();
    stack.handle(prompt("t-2")).unwrap();
    let ask2 = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-2", "ask-")).await;
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    stack.tab_closed("t-1");
    let id = ask2.1["event"]["surface"]["id"].as_str().unwrap().to_owned();
    stack
        .handle(
            serde_json::from_value(json!({ "cmd": "surface_action", "tab": "t-2", "surface": id,
        "name": "choose", "context": { "choice": "b" } }))
            .unwrap(),
        )
        .unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    let seen = rx.lock().unwrap().len();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let late: Vec<_> = rx.lock().unwrap()[seen..].iter().filter(|(_, v)| v["tab"] == "t-1").cloned().collect();
    assert_that!(late.iter().any(|(_, v)| v["event"]["kind"] == "surface_patch")).is_false();
    // Les deux questions ont rendu leur outil à l'agent : celle de t-1 relâchée, pas pendante.
    let outs: Vec<String> = common::logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!(outs).has_length(2);
    assert_that!(outs.iter().filter(|o| o.contains("cancelled")).count()).is_equal_to(1);
    assert_that!(outs.iter().filter(|o| o.contains("choice")).count()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_while_question_waits_answers_it_instead_of_new_turn() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [ask] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "la deuxième" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let r = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "report-")).await;
    assert_that!(r.1.to_string()).contains("la deuxième");
    assert_that!(common::logged(&log, "session/prompt")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_passes_the_ui_context_with_the_prompt() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    stack
        .handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "bonjour", "ui": { "slide": "s-libre" } })))
        .unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = &common::logged(&log, "session/prompt")[0]["prompt"];
    assert_that!(prompt[2]["text"].as_str().unwrap().contains("slide: s-libre")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn surface_action_on_an_unknown_surface_or_tab_is_an_error() {
    let (stack, _) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    let action = |tab: &str| cmd(json!({ "cmd": "surface_action", "tab": tab, "surface": "x", "name": "dismiss" }));
    assert_that!(stack.handle(action("t-9"))).is_err();
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" }))).unwrap();
    assert_that!(stack.handle(action("t-1"))).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_closes_the_turn_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "close_surface" && v["event"]["id"] == "turn-1")
        .await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surfaces").await;
    let last = rx.lock().unwrap().iter().rev().find(|(_, v)| v["event"]["kind"] == "surfaces").cloned().unwrap();
    assert_that!(last.1["event"]["list"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_on_a_tab_without_agent_starts_nothing() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" }))).unwrap();
    // Aucun onglet démarré : l'état rejoué est `idle`, sans surface.
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "status" && v["event"]["state"] == "idle").await;
    let surfaces = wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surfaces").await;
    assert_that!(surfaces.1["event"]["list"].clone()).is_equal_to(json!([]));
    assert_that!(common::read_log(&log)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_stack_ends_the_forwarding() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-2" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    drop(stack);
    // Le relais tient le puits (et son clone de `rx`) : il doit finir avec la pile.
    for _ in 0..250 {
        if Arc::strong_count(&rx) == 1 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("le relais survit à la pile");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prompt_queued_before_tab_closed_never_reaches_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    // Fermé avant que ses outils n'aient démarré : la consigne est encore en file.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "perdue" }))).unwrap();
    stack.tab_closed("t-1");
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-2", "text": "gardée" }))).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_that!(common::logged(&log, "session/new")).has_length(1);
    let prompts = common::logged(&log, "session/prompt");
    assert_that!(prompts).has_length(1);
    assert_that!(prompts[0].to_string().contains("gardée")).is_true();
    let from_closed = rx.lock().unwrap().iter().filter(|(_, v)| v["tab"] == "t-1").count();
    assert_that!(from_closed).is_equal_to(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_for_a_closed_tab_are_ignored_and_start_nothing() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    stack.tab_closed("t-1");
    // Arrivées en retard, après la fermeture : acceptées sans effet.
    assert_that!(stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "tardive" })))).is_ok();
    assert_that!(stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" })))).is_ok();
    assert_that!(stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" })))).is_ok();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-2", "text": "gardée" }))).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_that!(common::logged(&log, "session/new")).has_length(1);
    let from_closed = rx.lock().unwrap().iter().filter(|(_, v)| v["tab"] == "t-1").count();
    assert_that!(from_closed).is_equal_to(0);
}

/// Le processus `pid` existe-t-il encore ? (`kill -0`, sans signal envoyé.)
#[cfg(unix)]
fn alive(pid: u64) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn shutdown_kills_the_agent_and_later_commands_start_nothing() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let pid = common::logged(&log, "pid")[0].as_u64().unwrap();
    stack.shutdown().await;
    // Tué (puis ramassé en arrière-plan par tokio) : parti en moins de 2 s.
    let mut gone = false;
    for _ in 0..100 {
        if !alive(pid) {
            gone = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_that!(gone).is_true();
    assert_that!(stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "tardive" })))).is_ok();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_that!(common::logged(&log, "initialize")).has_length(1);
}
