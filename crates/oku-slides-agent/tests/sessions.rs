//! The tab's ACP session: resuming a known session, new conversation, keeper.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::*;
use oku_slides_agent::{AgentEvent, Keep};
use oku_slides_session::agent::AgentUi;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn known_session_is_resumed_at_the_first_prompt_only() {
    let h = start(json!({ "resume": true, "load_session": true, "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    // Rien n'est repris avant un prompt (contrôle faible : l'absence au bout de 100 ms).
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/resume")).is_equal_to(vec!["sess-42".to_owned()]);
    assert_that!(logged(&h.log, "session/load")).is_empty();
    assert_that!(logged(&h.log, "session/new")).is_empty();
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-42".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn loaded_session_replay_stays_out_of_events_and_surfaces() {
    let h = start(json!({ "load_session": true, "replay": ["ancien texte"], "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/load")).is_equal_to(vec!["sess-42".to_owned()]);
    let mut chunks = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let AgentEvent::Chunk { text, .. } = e {
            chunks.push(text);
        }
    }
    assert_that!(chunks).is_equal_to(vec!["ok".to_owned()]);
    assert_that!(serde_json::to_string(&t.hub.surfaces()).unwrap()).does_not_contain("ancien texte");
}

#[tokio::test(flavor = "multi_thread")]
async fn unresumed_session_opens_a_new_one_with_instructions_and_a_card_only_if_refused() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let lost: String = l.tr("agent-memory-lost").chars().take(30).collect();
    // Les consignes (`src/instructions/*.md`) sont en anglais seul, quelle que soit la langue
    // de l'interface ; le contexte leur donne la langue que leur règle cite.
    let instructions = ["You edit an Oku Slides presentation", "Creating a widget:", "interface language: French (fr)"];
    for caps in [json!({ "resume": true }), json!({ "load_session": true }), json!({})] {
        // Sans capacité, la reprise n'est pas tentée : pas de carte (t-611).
        let refused = caps.as_object().is_some_and(|c| !c.is_empty());
        let mut script = json!({ "unknown_sessions": true, "turns": [{ "chunks": ["ok"] }] });
        script.as_object_mut().unwrap().extend(caps.as_object().unwrap().clone());
        let h = start(script).await;
        seed_session(&h, "sess-42", &h.link.agent_id()).await;
        let t = h.tab("t-1");
        t.agent.prompt("encore".into(), AgentUi::default());
        wait_turn_ended(t).await;
        assert_that!(logged(&h.log, "session/new")).has_length(1);
        assert_that!(serde_json::to_string(&t.hub.surfaces()).unwrap().contains(lost.as_str())).is_equal_to(refused);
        let prompt = logged(&h.log, "session/prompt")[0].to_string();
        for part in instructions {
            assert_that!(prompt.as_str()).named(part).contains(part);
        }
        // La reprise a été tentée avec la capacité annoncée, et refusée (aucune tentative sans capacité).
        let tried = |method: &str, cap: &str| {
            let expected: Vec<String> = if caps[cap] == true { vec!["sess-42".into()] } else { vec![] };
            assert_that!(sessions_in(&h.log, method)).is_equal_to(expected);
        };
        tried("session/resume", "resume");
        tried("session/load", "load_session");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn session_of_another_agent_is_not_resumed() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, "sess-42", "autre-agent").await;
    let t = h.tab("t-1");
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    assert_that!(logged(&h.log, "session/new")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn session_is_resumed_after_the_agent_stops() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }, { "crash": true }, { "chunks": ["ok"] }] }))
        .await;
    let t = h.tab("t-1");
    t.agent.prompt("un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    {
        let mut rx = t.events.lock().await;
        wait_error(&mut rx).await;
    }
    t.agent.prompt("trois".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/resume")).is_equal_to(vec!["sess-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_then_prompt_never_resumes_the_forgotten_session() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.new_conversation();
    until(&mut rx, |e| matches!(e, AgentEvent::Conversation)).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.keeper.flush().await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-1".into(), "sess-2".into()]);
    assert_that!(h.session("t-1").lock().unwrap().conversation().and_then(|c| c.session.clone()))
        .is_equal_to(Some("sess-2".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn keeper_forget_after_a_queued_write_leaves_nothing() {
    let h = start(json!({ "turns": [] })).await;
    let t = h.tab("t-1");
    let n = t.hub.begin_turn();
    t.hub.emit(oku_slides_mcp::McpEvent::Surface { surface: frozen_turn(n) });
    t.hub.end_turn(n, "");
    t.keeper.send(Keep::Thread(n));
    assert_that!(t.keeper.forget().await).is_none();
    assert_that!(h.session("t-1").lock().unwrap().conversation()).is_none();
}
