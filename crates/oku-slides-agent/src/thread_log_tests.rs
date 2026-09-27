use speculoos::prelude::*;

use super::ThreadLog;
use crate::{AgentEvent, AgentState, PlanItem};

fn user(text: &str) -> AgentEvent {
    AgentEvent::UserText { text: text.into() }
}

#[test]
fn thread_log_keeps_the_last_200_entries() {
    let mut log = ThreadLog::default();
    for i in 0..250 {
        log.push(&user(&i.to_string()));
    }
    let entries = log.entries();
    assert_that!(entries).has_length(200);
    assert_that!(entries[0].clone()).is_equal_to(user("50"));
}

#[test]
fn thread_log_conversation_clears_the_thread() {
    let mut log = ThreadLog::default();
    log.push(&AgentEvent::Chunk { text: "a".into() });
    log.push(&AgentEvent::Conversation);
    assert_that!(log.entries()).is_equal_to(vec![AgentEvent::Conversation]);
}

#[test]
fn thread_log_merges_streamed_chunks_and_replays_the_state_last() {
    let mut log = ThreadLog::default();
    log.push(&AgentEvent::Status { state: AgentState::Working });
    log.push(&user("résume"));
    let mut full = String::new();
    for i in 0..500 {
        let text = format!("{i} ");
        full.push_str(&text);
        log.push(&AgentEvent::Chunk { text });
    }
    assert_that!(log.entries()).is_equal_to(vec![
        user("résume"),
        AgentEvent::Chunk { text: full },
        AgentEvent::Status { state: AgentState::Working },
    ]);
}

#[test]
fn thread_log_folds_tool_updates_into_their_call() {
    let mut log = ThreadLog::default();
    log.push(&AgentEvent::ToolCall { id: "t1".into(), title: "lire".into(), status: "pending".into() });
    log.push(&AgentEvent::Chunk { text: "a".into() });
    log.push(&AgentEvent::ToolUpdate { id: "t1".into(), title: None, status: Some("completed".into()) });
    log.push(&AgentEvent::ToolUpdate { id: "inconnu".into(), title: None, status: Some("failed".into()) });
    assert_that!(log.entries()).is_equal_to(vec![
        AgentEvent::ToolCall { id: "t1".into(), title: "lire".into(), status: "completed".into() },
        AgentEvent::Chunk { text: "a".into() },
    ]);
}

#[test]
fn thread_log_keeps_only_the_last_plan() {
    let plan = |s: &str| AgentEvent::Plan { entries: vec![PlanItem { content: "étape".into(), status: s.into() }] };
    let mut log = ThreadLog::default();
    log.push(&plan("pending"));
    log.push(&user("x"));
    log.push(&plan("completed"));
    assert_that!(log.entries()).is_equal_to(vec![plan("completed"), user("x")]);
}
