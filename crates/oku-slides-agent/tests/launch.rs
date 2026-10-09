mod common;

use common::*;
use oku_slides_agent::AgentEvent;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn first_prompt_launches_agent_without_fs_or_terminal() {
    let h = start(json!({ "turns": [{ "chunks": ["bonjour"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("bonjour".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    let init = &logged(&h.log, "initialize")[0];
    let caps = &init["clientCapabilities"];
    assert_that!(caps["fs"]["readTextFile"].as_bool()).is_equal_to(Some(false));
    assert_that!(caps["fs"]["writeTextFile"].as_bool()).is_equal_to(Some(false));
    assert_that!(caps["terminal"].as_bool()).is_equal_to(Some(false));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_without_http_mcp_is_refused_with_localized_error() {
    let h = start(json!({ "http": false })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("bonjour".into(), Default::default());
    let e = until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await;
    let AgentEvent::Error { message, .. } = e else { unreachable!() };
    assert_that!(message.contains("ne sait pas se connecter")).is_true();
    assert_that!(logged(&h.log, "session/new")).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_command_reports_not_found_with_command() {
    let h = start_with(json!({}), Some("/chemin/sans/agent")).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("bonjour".into(), Default::default());
    let AgentEvent::Error { message, .. } = until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await else {
        unreachable!()
    };
    assert_that!(message.contains("/chemin/sans/agent")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_that_never_gets_ready_is_stopped_and_the_next_start_works() {
    let h = start_ready_within(json!({ "hang_initialize": true }), std::time::Duration::from_millis(500)).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("bonjour".into(), Default::default());
    let message = within(std::time::Duration::from_secs(5), &mut rx, |e| matches!(e, AgentEvent::Error { .. }))
        .await
        .map(|e| match e {
            AgentEvent::Error { message, .. } => message,
            _ => unreachable!(),
        });
    assert_that!(message).is_some().contains("L'agent s'est arrêté");
    std::fs::write(script_path(&h), json!({ "turns": [{ "chunks": ["enfin"] }] }).to_string())
        .unwrap_or_else(|e| panic!("scénario : {e}"));
    t.agent.prompt("encore".into(), Default::default());
    let chunk = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { turn: 2, text: "enfin".into() });
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_command_error_comes_after_the_turn_is_opened() {
    let h = start_with(json!({}), Some("/chemin/sans/agent")).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("bonjour".into(), Default::default());
    let opened = until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. } | AgentEvent::Error { .. })).await;
    assert_that!(matches!(opened, AgentEvent::UserText { turn: 1, .. })).is_true();
    until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_id_is_the_fingerprint_of_the_launch() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    // Exactement ce que le harnais lance.
    let args: Vec<String> = vec![
        "--script".into(),
        script_path(&h).to_string_lossy().into_owned(),
        "--log".into(),
        h.log.to_string_lossy().into_owned(),
    ];
    let expected = oku_slides_agent::agent_fingerprint(env!("CARGO_BIN_EXE_oku-slides-fake-agent"), &args);
    assert_that!(h.link.agent_id()).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn fake_agent_by_prompt_plays_the_matching_turn() {
    let h =
        start(json!({ "by_prompt": { "bonjour": { "chunks": ["salut"] } }, "turns": [{ "chunks": ["autre"] }] })).await;
    let mut rx = h.tab("t-1").agent.subscribe();
    h.tab("t-1").agent.prompt("bonjour".into(), Default::default());
    let chunk = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { turn: 1, text: "salut".into() });
}
