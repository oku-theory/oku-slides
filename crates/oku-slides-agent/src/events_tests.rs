use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, Diff, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, SessionUpdate, TextContent,
    ToolCall, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};

use serde_json::json;
use speculoos::prelude::*;

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
        input: None,
        output: None,
    }));
}

/// Les arguments se lisent en JSON indenté ; le résultat, en texte.
#[test]
fn from_update_tool_call_keeps_its_input_and_text_output() {
    let u = SessionUpdate::ToolCall(
        ToolCall::new("t1", "mcp__oku-slides__widgets")
            .raw_input(json!({"slide": 4}))
            .content(vec![ContentBlock::Text(TextContent::new("3 widgets")).into()]),
    );
    let Some(AgentEvent::ToolCall { input, output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(input).is_equal_to(Some("{\n  \"slide\": 4\n}".to_owned()));
    assert_that!(output).is_equal_to(Some("3 widgets".to_owned()));
}

/// Sans contenu texte, le résultat est la sortie brute : ses textes MCP, sinon son JSON.
#[test]
fn from_update_tool_update_without_content_reads_the_raw_output() {
    let fields = ToolCallUpdateFields::new()
        .status(ToolCallStatus::Completed)
        .raw_output(json!([{"type": "text", "text": "fait"}, {"type": "text", "text": "rendu"}]));
    let u = SessionUpdate::ToolCallUpdate(ToolCallUpdate::new("t1", fields));
    let Some(AgentEvent::ToolUpdate { output, input, .. }) = from_update(4, u) else { panic!("pas une mise à jour") };
    assert_that!(output).is_equal_to(Some("fait\n\nrendu".to_owned()));
    assert_that!(input).is_none();
}

#[test]
fn from_update_tool_call_diff_output_names_the_file() {
    let u =
        SessionUpdate::ToolCall(ToolCall::new("t1", "Edit").content(vec![Diff::new("deck/slides.typ", "b").into()]));
    let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(output).is_equal_to(Some("deck/slides.typ".to_owned()));
}

/// Des arguments vides (`{}`) n'ont rien à montrer.
#[test]
fn from_update_tool_call_empty_input_is_none() {
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "slides").raw_input(json!({})));
    let Some(AgentEvent::ToolCall { input, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(input).is_none();
}

#[test]
fn from_update_tool_call_long_output_is_clipped() {
    let long = "x".repeat(oku_slides_session::agent::DETAIL_CHARS + 50);
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "slides").raw_output(json!(long)));
    let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    let output = output.unwrap();
    assert_that!(output.chars().count()).is_equal_to(oku_slides_session::agent::DETAIL_CHARS);
    assert_that!(output.ends_with('…')).is_true();
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
    let json = serde_json::to_value(AgentEvent::UserText { turn: 3, text: "x".into(), images: Vec::new() }).unwrap();
    assert_that!(json).is_equal_to(serde_json::json!({ "kind": "user_text", "turn": 3, "text": "x" }));
}

#[test]
fn status_serializes_without_scope() {
    let busy = AgentEvent::Status { state: AgentState::Working };
    assert_that!(serde_json::to_string(&busy).unwrap())
        .is_equal_to(r#"{"kind":"status","state":"working"}"#.to_owned());
}

#[test]
fn idle_serializes_as_bare_status() {
    assert_that!(serde_json::to_string(&AgentEvent::idle()).unwrap())
        .is_equal_to(r#"{"kind":"status","state":"idle"}"#.to_owned());
}

#[test]
fn conversation_serializes_as_bare_kind() {
    assert_that!(serde_json::to_value(AgentEvent::Conversation).unwrap())
        .is_equal_to(json!({ "kind": "conversation" }));
}

#[test]
fn voice_event_serializes_as_its_kind_alone() {
    assert_that!(serde_json::to_value(AgentEvent::Voice).unwrap()).is_equal_to(json!({ "kind": "voice" }));
}

#[test]
fn availability_event_serializes_its_reason_or_null() {
    let usable = serde_json::to_value(AgentEvent::Availability { unusable: None }).unwrap();
    assert_that!(usable).is_equal_to(json!({ "kind": "availability", "unusable": null }));
    let unusable = serde_json::to_value(AgentEvent::Availability { unusable: Some("x".into()) }).unwrap();
    assert_that!(unusable["unusable"].clone()).is_equal_to(json!("x"));
}

/// Une sortie MCP qui mêle textes et image : les textes, pas le JSON (base64) de l'image.
#[test]
fn from_update_mixed_raw_output_keeps_only_its_texts() {
    let raw = json!([{"type": "image", "data": "iVBORw0KGgo="}, {"type": "text", "text": "rendu"}]);
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "slides").raw_output(raw));
    let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(output).is_equal_to(Some("rendu".to_owned()));
}

#[test]
fn from_update_object_raw_output_is_its_json() {
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "slides").raw_output(json!({"ok": true})));
    let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(output).is_equal_to(Some("{\n  \"ok\": true\n}".to_owned()));
}

/// Une sortie MCP faite d'images seulement n'a pas de résultat à lire (pas de base64).
#[test]
fn from_update_image_only_raw_output_has_no_output() {
    let raw = json!([{"type": "image", "data": "iVBORw0KGgo=", "mimeType": "image/png"}]);
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "slides").raw_output(raw));
    let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(output).is_none();
}

/// Le texte du contenu l'emporte sur la sortie brute.
#[test]
fn from_update_content_text_wins_over_raw_output() {
    let u = SessionUpdate::ToolCall(
        ToolCall::new("t1", "slides")
            .content(vec![ContentBlock::Text(TextContent::new("lu")).into()])
            .raw_output(json!("brut")),
    );
    let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(output).is_equal_to(Some("lu".to_owned()));
}

/// Un texte vide dans le contenu ne cache pas la sortie brute.
#[test]
fn from_update_blank_content_text_falls_back_to_raw_output() {
    let u = SessionUpdate::ToolCall(
        ToolCall::new("t1", "slides")
            .content(vec![ContentBlock::Text(TextContent::new(" ")).into()])
            .raw_output(json!("brut")),
    );
    let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(output).is_equal_to(Some("brut".to_owned()));
}

#[test]
fn from_update_empty_raw_output_has_no_output() {
    for raw in [json!([]), json!({})] {
        let u = SessionUpdate::ToolCall(ToolCall::new("t1", "slides").raw_output(raw));
        let Some(AgentEvent::ToolCall { output, .. }) = from_update(4, u) else { panic!("pas un appel") };
        assert_that!(output).is_none();
    }
}

/// Des arguments en chaîne se lisent tels quels, sans guillemets.
#[test]
fn from_update_string_input_is_shown_as_is() {
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "Bash").raw_input(json!("ls -la")));
    let Some(AgentEvent::ToolCall { input, .. }) = from_update(4, u) else { panic!("pas un appel") };
    assert_that!(input).is_equal_to(Some("ls -la".to_owned()));
}
