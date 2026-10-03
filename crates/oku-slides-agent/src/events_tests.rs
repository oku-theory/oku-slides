use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, SessionUpdate, TextContent,
    ToolCall, ToolCallStatus,
};
use speculoos::prelude::*;

use super::{AgentEvent, PlanItem, from_update};

#[test]
fn from_update_text_chunk_gives_chunk() {
    let u = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new("salut"))));
    assert_that!(from_update(u)).is_equal_to(Some(AgentEvent::Chunk { text: "salut".into() }));
}

#[test]
fn from_update_tool_call_keeps_id_title_and_snake_case_status() {
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "Lire la slide").status(ToolCallStatus::InProgress));
    assert_that!(from_update(u)).is_equal_to(Some(AgentEvent::ToolCall {
        id: "t1".into(),
        title: "Lire la slide".into(),
        status: "in_progress".into(),
    }));
}

#[test]
fn from_update_plan_gives_items() {
    let u = SessionUpdate::Plan(Plan::new(vec![PlanEntry::new(
        "Lire",
        PlanEntryPriority::High,
        PlanEntryStatus::Completed,
    )]));
    assert_that!(from_update(u)).is_equal_to(Some(AgentEvent::Plan {
        entries: vec![PlanItem { content: "Lire".into(), status: "completed".into() }],
    }));
}

#[test]
fn user_text_serializes_its_turn() {
    let json = serde_json::to_value(AgentEvent::UserText { turn: 3, text: "x".into() }).unwrap();
    assert_that!(json).is_equal_to(serde_json::json!({ "kind": "user_text", "turn": 3, "text": "x" }));
}
