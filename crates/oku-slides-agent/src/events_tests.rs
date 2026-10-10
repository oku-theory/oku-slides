use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, SessionUpdate, TextContent,
    ToolCall, ToolCallStatus,
};
use std::collections::BTreeMap;

use oku_slides_i18n::Localizer;
use serde_json::json;
use speculoos::prelude::*;

use oku_slides_session::agent::AgentScope;

use super::{AgentEvent, AgentState, PlanItem, from_update};

#[test]
fn from_update_text_chunk_gives_chunk() {
    let u = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new("salut"))));
    assert_that!(from_update(4, u)).is_equal_to(Some(AgentEvent::Chunk { turn: 4, text: "salut".into() }));
}

#[test]
fn from_update_tool_call_keeps_id_title_and_snake_case_status() {
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "Lire la slide").status(ToolCallStatus::InProgress));
    assert_that!(from_update(4, u)).is_equal_to(Some(AgentEvent::ToolCall {
        turn: 4,
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
    assert_that!(from_update(4, u)).is_equal_to(Some(AgentEvent::Plan {
        turn: 4,
        entries: vec![PlanItem { content: "Lire".into(), status: "completed".into() }],
    }));
}

#[test]
fn user_text_serializes_its_turn() {
    let json =
        serde_json::to_value(AgentEvent::UserText { turn: 3, scope: AgentScope::Slides, text: "x".into() }).unwrap();
    assert_that!(json)
        .is_equal_to(serde_json::json!({ "kind": "user_text", "turn": 3, "scope": { "kind": "slides" }, "text": "x" }));
}

#[test]
fn status_json_carries_the_scope_only_when_set() {
    let busy = AgentEvent::Status { state: AgentState::Working, scope: Some(AgentScope::Slides) };
    assert_that!(serde_json::to_string(&busy).unwrap())
        .is_equal_to(r#"{"kind":"status","state":"working","scope":{"kind":"slides"}}"#.to_owned());
    assert_that!(serde_json::to_string(&AgentEvent::idle()).unwrap())
        .is_equal_to(r#"{"kind":"status","state":"idle"}"#.to_owned());
}

#[test]
fn picker_event_serializes_with_its_surface() {
    let s = crate::picker::picker_surface(&BTreeMap::new(), "x", false, false, &Localizer::new(Some("fr")));
    let v = serde_json::to_value(AgentEvent::Picker { surface: s }).unwrap();
    assert_that!(v["kind"].clone()).is_equal_to(json!("picker"));
    assert_that!(v["surface"]["id"].clone()).is_equal_to(json!("agent-picker"));
}
