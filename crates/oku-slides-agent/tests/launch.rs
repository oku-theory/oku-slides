mod common;

use common::*;
use oku_slides_agent::{AgentEvent, AgentState};
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

/// Un second faux agent, à côté du premier : son scénario et son journal.
fn second_launch(h: &Harness, script: serde_json::Value) -> (oku_slides_agent::Launch, std::path::PathBuf) {
    let (path, log) = (h.log.with_file_name("script-bis.json"), h.log.with_file_name("log-bis.jsonl"));
    std::fs::write(&path, script.to_string()).unwrap_or_else(|e| panic!("scénario bis : {e}"));
    let launch = oku_slides_agent::Launch {
        command: env!("CARGO_BIN_EXE_oku-slides-fake-agent").to_owned(),
        args: vec![
            "--script".into(),
            path.to_string_lossy().into_owned(),
            "--log".into(),
            log.to_string_lossy().into_owned(),
        ],
        fallback_cwd: h.log.parent().unwrap_or_else(|| panic!("dossier du journal")).to_owned(),
        ready_within: oku_slides_agent::READY_WITHIN,
    };
    (launch, log)
}

#[tokio::test(flavor = "multi_thread")]
async fn switch_between_turns_sends_the_next_prompt_to_the_new_agent() {
    let h = start(json!({ "turns": [{ "chunks": ["un"] }, { "chunks": ["deux"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("premier".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    let before = h.link.agent_id();
    let (launch, log_bis) = second_launch(&h, json!({ "turns": [{ "chunks": ["bis"] }] }));
    h.link.switch(launch);
    assert_that!(h.link.agent_id()).is_not_equal_to(before);
    t.agent.prompt("second".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Chunk { text, .. } if text == "bis")).await;
    assert_that!(logged(&log_bis, "session/prompt").len()).is_equal_to(1);
    assert_that!(logged(&h.log, "session/prompt").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn switch_during_a_turn_stops_it_and_the_next_prompt_reaches_the_new_agent() {
    // Le premier agent ne finit jamais son tour (question sans réponse).
    let h = start(json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?",
        "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }] }] }))
    .await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("premier".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Working, .. })).await;
    let (launch, log_bis) = second_launch(&h, json!({ "turns": [{ "chunks": ["bis"] }] }));
    h.link.switch(launch);
    until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await;
    t.agent.prompt("second".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Chunk { text, .. } if text == "bis")).await;
    assert_that!(logged(&log_bis, "session/prompt").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn switch_while_launching_relaunches_with_the_new_agent() {
    // Le premier agent met 1 s à répondre à `initialize`.
    let h = start(json!({ "init_delay_ms": 1000, "turns": [{ "chunks": ["un"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("premier".into(), Default::default());
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while logged(&h.log, "initialize").is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("le premier agent n'a pas reçu `initialize`"));
    let (launch, log_bis) = second_launch(&h, json!({ "turns": [{ "chunks": ["bis"] }] }));
    h.link.switch(launch);
    until(&mut rx, |e| matches!(e, AgentEvent::Chunk { text, .. } if text == "bis")).await;
    // Le premier agent avait bien commencé sa poignée de main.
    assert_that!(logged(&h.log, "initialize")).has_length(1);
    assert_that!(logged(&h.log, "session/prompt")).is_empty();
    assert_that!(logged(&log_bis, "session/prompt").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn switch_during_a_hung_handshake_reaches_the_new_agent_without_waiting_for_readiness() {
    // Le premier agent ne répondrait à `initialize` qu'au bout d'une minute (`READY_WITHIN` : 120 s).
    let h = start(json!({ "init_delay_ms": 60000, "turns": [{ "chunks": ["un"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("premier".into(), Default::default());
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while logged(&h.log, "initialize").is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("le premier agent n'a pas reçu `initialize`"));
    let (launch, log_bis) = second_launch(&h, json!({ "turns": [{ "chunks": ["bis"] }] }));
    h.link.switch(launch);
    let chunk = within(
        std::time::Duration::from_secs(5),
        &mut rx,
        |e| matches!(e, AgentEvent::Chunk { text, .. } if text == "bis"),
    )
    .await;
    assert_that!(chunk).is_some();
    assert_that!(logged(&h.log, "session/prompt")).is_empty();
    assert_that!(logged(&log_bis, "session/prompt").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_after_switch_is_still_final() {
    let h = start(json!({ "turns": [] })).await;
    let (launch, _) = second_launch(&h, json!({ "turns": [] }));
    h.link.switch(launch);
    h.link.shutdown(std::time::Duration::from_secs(2)).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("après".into(), Default::default());
    let message = wait_error(&mut rx).await;
    assert_that!(message).contains("L'agent s'est arrêté");
}
