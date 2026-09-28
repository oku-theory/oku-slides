mod common;

use common::*;
use oku_slides_mcp::McpEvent;
use rust_mcp_sdk::McpClient;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn apply_changes_deck_and_emits_changed() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "pro" }] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(h.session.lock().unwrap().deck().theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(matches!(rx.recv().await.unwrap(), McpEvent::Changed)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_inside_turn_joins_the_turn_group() {
    let h = start().await;
    h.session.lock().unwrap().begin_turn("plus sobre");
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "pro" }] })).await;
    call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "papier" }] })).await;
    let mut s = h.session.lock().unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : plus sobre"));
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_invalid_op_returns_error_with_path() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(
        &c,
        "apply",
        json!({ "ops": [{ "op": "set_text", "slide": "s-libre", "element": "absent", "text": [] }] }),
    )
    .await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).contains("absent")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_schema_accepts_every_op_kind_in_tool_list() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.request_tool_list(None).await.unwrap().tools;
    let apply = tools.into_iter().find(|t| t.name == "apply").unwrap();
    let schema = serde_json::to_string(&apply.input_schema).unwrap();
    for op in ["set_text", "set_chart_option", "add_slide", "set_theme"] {
        assert_that!(schema.contains(op)).named(op).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn focus_emits_focus_event_without_changing_deck() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "focus", json!({ "slide": "s-libre", "element": "e-texte" })).await;
    match rx.recv().await.unwrap() {
        McpEvent::Focus { slide, element } => {
            assert_that!(slide.as_str()).is_equal_to("s-libre");
            assert_that!(element.as_deref()).is_equal_to(Some("e-texte"));
        }
        other => panic!("évènement inattendu : {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_then_close_templates_round_trips_mode() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let set = oku_slides_session::DEFAULT_SET;
    call(&c, "edit_templates", json!({ "set": set })).await;
    assert_that!(h.session.lock().unwrap().snapshot().editing.is_some()).is_true();
    call(&c, "close_templates", json!({})).await;
    assert_that!(h.session.lock().unwrap().snapshot().editing.is_none()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn in_history_outline_says_history_and_apply_is_refused() {
    let h = start().await;
    let theme = h.session.lock().unwrap().deck().theme.clone();
    h.session.lock().unwrap().handle(oku_slides_session::Request::HistoryOpen).unwrap();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("history"));
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "pro" }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let mut s = h.session.lock().unwrap();
    s.handle(oku_slides_session::Request::HistoryClose).unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(theme);
}
