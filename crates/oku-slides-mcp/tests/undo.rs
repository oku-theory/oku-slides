//! The undo, redo and `undo_turn` tools, and focus without a slide, over MCP (t-999).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use std::sync::PoisonError;

use common::*;
use oku_slides_core::{Op, Target};
use oku_slides_mcp::McpEvent;
use oku_slides_session::Session;
use serde_json::json;
use speculoos::prelude::*;

/// Retitles the third slide: one undo step.
fn retitle(s: &mut Session, title: &str) {
    let slide = s.deck().slides[2].id.clone();
    let op = Op::SetParam { target: Target { slide, element: None }, param: "title".into(), value: json!(title) };
    s.apply("Titre", vec![op], None).unwrap_or_else(|e| panic!("retitle: {e}"));
}

/// The title of the third slide, as JSON.
fn title(h: &Harness) -> Option<serde_json::Value> {
    let s = h.session.lock().unwrap_or_else(PoisonError::into_inner);
    serde_json::to_value(&s.deck().slides[2].params).ok()?.get("title").cloned()
}

/// Edits the deck through the harness's session.
fn edit(h: &Harness, f: impl FnOnce(&mut Session)) {
    f(&mut h.session.lock().unwrap_or_else(PoisonError::into_inner));
}

#[tokio::test(flavor = "multi_thread")]
async fn undo_tool_steps_two_returns_done_and_emits_changed() {
    let h = start().await;
    let before = title(&h);
    edit(&h, |s| {
        retitle(s, "Un");
        retitle(s, "Deux");
    });
    let mut rx = h.hub.subscribe();
    let c = server(&h).await;
    let r = call_within(&c, "undo", json!({ "steps": 2 })).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "done": 2 }));
    assert_that!(title(&h)).is_equal_to(before);
    assert_that!(matches!(wait_event(&mut rx, Some).await, McpEvent::Changed)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn redo_tool_beyond_stack_reports_count() {
    let h = start().await;
    edit(&h, |s| retitle(s, "Un"));
    let after = title(&h);
    let c = server(&h).await;
    call_within(&c, "undo", json!({})).await;
    let r = call_within(&c, "redo", json!({ "steps": 3 })).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "done": 1 }));
    assert_that!(title(&h)).is_equal_to(after);
}

#[tokio::test(flavor = "multi_thread")]
async fn undo_tool_steps_out_of_range_refused() {
    let h = start().await;
    let c = server(&h).await;
    let r = call_within(&c, "undo", json!({ "steps": 21 })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn undo_turn_not_on_top_refused_localized() {
    let h = start().await;
    edit(&h, |s| {
        s.begin_turn(4, "retitre");
        retitle(s, "Par l'agent");
        s.end_turn();
        retitle(s, "À la main");
    });
    let c = server(&h).await;
    let r = call_within(&c, "undo_turn", json!({ "turn": 4 })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.localizer.locale()).is_equal_to("fr");
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-turn-not-on-top"));
    assert_that!(title(&h)).is_equal_to(Some(json!("À la main")));
}

#[tokio::test(flavor = "multi_thread")]
async fn undo_turn_of_a_finished_turn_on_top_undoes_it() {
    let h = start().await;
    let before = title(&h);
    edit(&h, |s| {
        s.begin_turn(4, "retitre");
        retitle(s, "Un");
        retitle(s, "Deux");
        s.end_turn();
    });
    let c = server(&h).await;
    let r = call_within(&c, "undo_turn", json!({ "turn": 4 })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(title(&h)).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn undo_turn_of_current_turn_refused() {
    let h = start().await;
    let turn = h.hub.begin_turn();
    edit(&h, |s| {
        s.begin_turn(turn, "retitre");
        retitle(s, "Pendant");
    });
    let during = title(&h);
    let c = server(&h).await;
    let r = call_within(&c, "undo_turn", json!({ "turn": turn })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-undo-open-turn"));
    assert_that!(title(&h)).is_equal_to(during);
}

#[tokio::test(flavor = "multi_thread")]
async fn focus_without_slide_deselects() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = server(&h).await;
    let r = call_within(&c, "focus", json!({})).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    match wait_event(&mut rx, Some).await {
        McpEvent::Focus { slide } => assert_that!(slide).is_none(),
        other => panic!("unexpected event: {other:?}"),
    }
}

/// The agent never selects: an element named without a slide is refused, nothing deselects.
#[tokio::test(flavor = "multi_thread")]
async fn focus_element_without_slide_is_refused() {
    let h = start().await;
    let c = server(&h).await;
    let r = call_within(&c, "focus", json!({ "element": "e-texte" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}
