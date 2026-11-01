//! Fake agent turns: thread, tools, end, errors and fallback receipts.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::*;
use oku_slides_agent::AgentEvent;
use oku_slides_session::agent::AgentUi;
use serde_json::json;
use speculoos::prelude::*;

fn ended(e: &AgentEvent) -> bool {
    matches!(e, AgentEvent::TurnEnded { .. })
}

#[tokio::test(flavor = "multi_thread")]
async fn new_session_injects_mcp_server_with_authorization_header() {
    let h = start(json!({ "turns": [{}] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("bonjour".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let new = &logged(&h.log, "session/new")[0];
    let server = &new["mcpServers"][0];
    assert_that!(server["url"].as_str()).is_equal_to(Some(t.server.url()));
    assert_that!(server["name"].as_str()).is_equal_to(Some("oku-slides"));
    assert_that!(server["headers"][0]["name"].as_str()).is_equal_to(Some("Authorization"));
    assert_that!(server["headers"][0]["value"].as_str().map(str::to_owned)).is_equal_to(Some(t.server.authorization()));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_sends_instructions_context_then_text_and_streams_chunks() {
    let h = start(json!({ "turns": [{ "chunks": ["Je regarde."] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    let ui = AgentUi { slide: Some("s-libre".into()), ..Default::default() };
    t.agent.prompt("rends le titre plus court".into(), ui);
    let chunk = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { turn: 1, text: "Je regarde.".into() });
    until(&mut rx, ended).await;
    let blocks = logged(&h.log, "session/prompt")[0]["prompt"].as_array().cloned().unwrap();
    assert_that!(blocks.len()).is_equal_to(6);
    assert_that!(blocks[1]["text"].as_str().unwrap().starts_with("This tab edits a deck.")).is_true();
    assert_that!(blocks[2]["text"].as_str().unwrap().contains("preview_widget")).is_true();
    assert_that!(blocks[3]["text"].as_str().unwrap().starts_with("<oku-slides-guides>")).is_true();
    assert_that!(blocks[4]["text"].as_str().unwrap().starts_with("<oku-slides-context>")).is_true();
    assert_that!(blocks[4]["text"].as_str().unwrap().contains("slide: s-libre")).is_true();
    assert_that!(blocks[5]["text"].as_str()).is_equal_to(Some("rends le titre plus court"));
}

/// The texts of the blocks of prompt `n` of the log.
fn prompt_texts(log: &std::path::Path, n: usize) -> Vec<String> {
    let blocks = logged(log, "session/prompt")[n]["prompt"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("prompt {n} has no blocks"));
    blocks.iter().map(|b| b["text"].as_str().unwrap_or_default().to_owned()).collect()
}

/// A deck tab pins Base, whose guide is sent once, between the instruction files and the context.
#[tokio::test(flavor = "multi_thread")]
async fn prompt_sends_the_guides_after_the_instructions_then_not_on_an_unchanged_turn() {
    let h = start(json!({ "turns": [{}] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, ended).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let (first, second) = (prompt_texts(&h.log, 0), prompt_texts(&h.log, 1));
    let at = |texts: &[String], head: &str| texts.iter().position(|t| t.starts_with(head));
    let (widgets, guides, context) = (
        first.iter().position(|t| t.contains("preview_widget")),
        at(&first, "<oku-slides-guides>"),
        at(&first, "<oku-slides-context>"),
    );
    assert_that!(guides.is_some_and(|g| widgets < Some(g) && Some(g) < context)).is_true();
    assert_that!(first[guides.unwrap_or_default()].as_str())
        .contains("<guide for=\"use\" kind=\"widgets\" name=\"Base\"");
    assert_that!(at(&second, "<oku-slides-guides>")).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn second_prompt_of_session_has_no_instructions() {
    let h = start(json!({ "turns": [{}] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, ended).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let prompts = logged(&h.log, "session/prompt");
    assert_that!(prompts[1]["prompt"].as_array().map(Vec::len)).is_equal_to(Some(2));
    assert_that!(logged(&h.log, "session/new").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_during_turn_sends_cancel_and_ends_cancelled() {
    let h = start(json!({ "turns": [{ "wait_cancel": true }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("long".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.cancel();
    let e = until(&mut rx, ended).await;
    assert_that!(e).is_equal_to(AgentEvent::TurnEnded { turn: 1, stop: "cancelled".into() });
    assert_that!(logged(&h.log, "session/cancel")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_during_wait_cancel_turn_cancels_once_and_continues_same_session() {
    let h = start(json!({ "turns": [{ "wait_cancel": true }, {}] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    let first = until(&mut rx, ended).await;
    assert_that!(first).is_equal_to(AgentEvent::TurnEnded { turn: 1, stop: "cancelled".into() });
    until(&mut rx, ended).await;
    assert_that!(logged(&h.log, "session/cancel")).has_length(1);
    assert_that!(logged(&h.log, "session/new")).has_length(1);
    assert_that!(logged(&h.log, "session/prompt")).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_then_prompt_during_wait_cancel_turn_reopens_session() {
    // Chaque session a désormais son propre compteur de tours (t-224) : la
    // session rouverte rejoue elle aussi le tour 0, donc « deux » attend une
    // annulation comme « un ».
    let h = start(json!({ "turns": [{ "wait_cancel": true }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.new_conversation();
    t.agent.prompt("deux".into(), AgentUi::default());
    let first = until(&mut rx, ended).await;
    assert_that!(first).is_equal_to(AgentEvent::TurnEnded { turn: 1, stop: "cancelled".into() });
    until(&mut rx, |e| matches!(e, AgentEvent::Conversation)).await;
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.cancel();
    let second = until(&mut rx, ended).await;
    assert_that!(second).is_equal_to(AgentEvent::TurnEnded { turn: 2, stop: "cancelled".into() });
    assert_that!(logged(&h.log, "session/new")).has_length(2);
    assert_that!(logged(&h.log, "session/cancel")).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_then_cancel_during_wait_cancel_turn_drops_the_prompt() {
    let h = start(json!({ "turns": [{ "wait_cancel": true }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    t.agent.cancel();
    until(&mut rx, ended).await;
    assert_that!(logged(&h.log, "session/prompt")).has_length(1);
    assert_that!(logged(&h.log, "session/cancel")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_opens_a_new_session() {
    let h = start(json!({ "turns": [{}] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, ended).await;
    t.agent.new_conversation();
    until(&mut rx, |e| matches!(e, AgentEvent::Conversation)).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    until(&mut rx, ended).await;
    assert_that!(logged(&h.log, "session/new")).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_calls_of_a_turn_form_one_undo_entry() {
    let apply = |t: &str| json!({ "tool": "apply", "args": { "ops": [{ "op": "set_theme", "theme": theme_ref(t) }] } });
    let h = start(json!({ "turns": [{ "mcp": [apply("pro"), apply("papier")] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("change le thème deux fois".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let mut s = t.session.lock().unwrap();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : change le thème deux fois"));
    s.undo().unwrap();
    assert_that!(s.snapshot().can_undo).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn crash_mid_turn_reports_stopped_and_closes_group() {
    let apply = json!({ "tool": "apply", "args": { "ops": [{ "op": "set_theme", "theme": theme_ref("pro") }] } });
    let h = start(json!({ "turns": [{ "mcp": [apply], "crash": true }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("puis plante".into(), AgentUi::default());
    let AgentEvent::Error { message, .. } = until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await else {
        unreachable!()
    };
    assert_that!(message.as_str()).is_equal_to("L'agent s'est arrêté.");
    let mut s = t.session.lock().unwrap();
    // Le thème sans épinglages est complété depuis la bibliothèque, par une lecture bloquante.
    let op =
        oku_slides_core::Op::SetTheme { theme: theme_ref("papier"), themes: std::collections::BTreeMap::default() };
    tokio::task::block_in_place(|| s.apply("Après", vec![op], None)).unwrap();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Après"));
}

#[tokio::test(flavor = "multi_thread")]
async fn instructions_ask_for_a_report_at_the_end_of_each_turn() {
    let h = start(json!({ "turns": [{}] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("bonjour".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let blocks = logged(&h.log, "session/prompt")[0]["prompt"].as_array().cloned().unwrap();
    assert_that!(blocks[0]["text"].as_str().unwrap().contains("report")).is_true();
}
