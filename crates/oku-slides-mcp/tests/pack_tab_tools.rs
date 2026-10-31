//! A pack tab's own tools: selection, preview theme, copies (t-1191).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::kinds::unknown_tool;
use common::workspace_harness::{
    ACME, ACME_WIDGET, edited_pack, library_files, next_event, pile_slide_ids, preview_theme_and_dirty, selected_item,
    start_in_deck_pack_tab, start_in_pack_tab,
};
use common::*;
use oku_slides_core::Namespace;
use oku_slides_mcp::McpEvent;
use oku_slides_session::agent::AgentUi;
use oku_slides_session::agent::bounds::label;
use oku_slides_session::{Request, Workspace};
use serde_json::json;
use speculoos::prelude::*;

/// The paths of library `files`.
fn paths(files: &[(std::path::PathBuf, Vec<u8>)]) -> Vec<std::path::PathBuf> {
    files.iter().map(|(p, _)| p.clone()).collect()
}

const ALTO: &str = "alto-wealth";

#[tokio::test(flavor = "multi_thread")]
async fn select_item_of_an_item_selects_it_and_emits_changed() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let mut events = h.hub.subscribe();
    let c = server(&h).await;
    let r = call(&c, "select_item", json!({ "name": ACME_WIDGET })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(selected_item(&ws, &tab)).is_equal_to(Some(ACME_WIDGET.to_owned()));
    assert_that!(matches!(next_event(&mut events).await, McpEvent::Changed)).is_true();
}

/// The tab's agent context as the interface sends it: the selected item is the open widget.
fn context_of(ws: &std::sync::Arc<std::sync::Mutex<Workspace>>, tab: &str) -> String {
    let widget = selected_item(ws, tab);
    tokio::task::block_in_place(|| {
        Workspace::with_session(ws, tab, |s| s.agent_context(&AgentUi { widget: widget.clone(), ..AgentUi::default() }))
            .0
            .unwrap_or_else(|e| panic!("tab {tab}: {e}"))
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn select_item_of_an_item_with_trial_values_puts_them_in_the_context() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let trial = call(&c, "set_widget_trial", json!({ "name": ACME_WIDGET, "values": { "label": "Essai" } })).await;
    assert_that!(trial.is_error).is_not_equal_to(Some(true));
    assert_that!(context_of(&ws, &tab).contains("trial:")).is_false();
    let r = call(&c, "select_item", json!({ "name": ACME_WIDGET })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(context_of(&ws, &tab).as_str())
        .contains(format!("widget: {ACME_WIDGET} · trial: \"label\" = \"Essai\"\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn save_pack_outside_a_pack_tab_is_refused_as_an_unknown_tool() {
    let h = start().await;
    let c = server(&h).await;
    let r = call(&c, "save_pack", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "save_pack"));
}

#[tokio::test(flavor = "multi_thread")]
async fn select_item_of_an_unknown_item_is_refused_and_selects_nothing() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let before = selected_item(&ws, &tab);
    let c = server(&h).await;
    let r = call(&c, "select_item", json!({ "name": "nope" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let expected = h.localizer.tr_with("session-unknown-widget", &[("name", label("nope").into())]);
    assert_that!(text_of(&r)).is_equal_to(expected);
    assert_that!(text_of(&r)).contains("introuvable");
    assert_that!(selected_item(&ws, &tab)).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_preview_theme_changes_the_preview_and_leaves_the_tab_clean() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let r = call(&c, "set_preview_theme", json!({ "theme": "pro" })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(preview_theme_and_dirty(&ws, &tab)).is_equal_to((Some("pro".to_owned()), false));
}

#[tokio::test(flavor = "multi_thread")]
async fn save_pack_confirmed_saves_a_copy_under_a_new_id() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let before = library_files(h.data());
    let r = guarded(&h, "save_pack", json!({}), true).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let edited = edited_pack(&ws, &tab);
    assert_that!(edited.is_some() && edited != Some(ACME)).is_true();
    let (before, after) = (paths(&before), paths(&library_files(h.data())));
    assert_that!(after.len()).is_greater_than(before.len());
    assert_that!(before.iter().all(|p| after.contains(p))).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn save_pack_denied_saves_nothing() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let before = library_files(h.data());
    let r = guarded(&h, "save_pack", json!({}), false).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-denied"));
    assert_that!(edited_pack(&ws, &tab)).is_equal_to(Some(ACME));
    assert_that!(library_files(h.data())).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn save_pack_asks_with_the_pack_name_as_a_label() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let name = Workspace::with_session(&ws, &tab, |s| s.snapshot().editing.map(|e| e.mode.name.clone())).0;
    let name = name.unwrap_or_else(|e| panic!("tab {tab}: {e}")).unwrap_or_else(|| panic!("a pack tab"));
    let (message, danger, _r) = guarded_message(&h, "save_pack", json!({}), false).await;
    let expected = h.localizer.tr_with("agent-confirm-copy-pack", &[("name", label(&name).into())]);
    assert_that!(message).is_equal_to(expected);
    assert_that!(danger).is_false();
}

/// A deck-pack copy stays deck-only (session rule): the owner deck pins it, the library gains nothing.
#[tokio::test(flavor = "multi_thread")]
async fn save_pack_in_a_deck_pack_tab_copies_deck_only_and_never_writes_the_library() {
    let (h, ws, tab, pack) = start_in_deck_pack_tab(ALTO).await;
    let owner_pack = |ws: &_| {
        let found = tokio::task::block_in_place(|| {
            Workspace::with_session(ws, "t-1", |s| s.deck().layouts.get(&Namespace::deck_layouts()).map(|p| p.id)).0
        });
        found.unwrap_or_else(|e| panic!("tab t-1: {e}"))
    };
    let before = library_files(h.data());
    let r = guarded(&h, "save_pack", json!({}), true).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let edited = edited_pack(&ws, &tab);
    assert_that!(edited.is_some() && edited != Some(pack)).is_true();
    assert_that!(library_files(h.data())).is_equal_to(before);
    assert_that!(owner_pack(&ws)).is_equal_to(edited);
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_widget_in_a_pack_tab_answers_the_copy_name() {
    let (h, _ws, _tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let r = call(&c, "duplicate_widget", json!({ "from": ACME_WIDGET })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(text_of(&r).as_str()).contains(format!("\"name\":\"{ACME_WIDGET}-2\""));
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_widget_in_a_pack_tab_emits_changed_then_widget_touched() {
    let (h, _ws, _tab) = start_in_pack_tab().await;
    let mut events = h.hub.subscribe();
    let c = server(&h).await;
    let r = call(&c, "duplicate_widget", json!({ "from": ACME_WIDGET })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(matches!(next_event(&mut events).await, McpEvent::Changed)).is_true();
    let touched = next_event(&mut events).await;
    assert_that!(matches!(&touched, McpEvent::WidgetTouched { name, removed: false } if name == "badge-2")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn reorder_widgets_with_a_missing_item_is_refused_in_french() {
    let (h, _ws, _tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let r = call(&c, "reorder_widgets", json!({ "order": [] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let expected =
        h.localizer.tr_with("session-widget-order-mismatch", &[("missing", ACME_WIDGET.into()), ("extra", "—".into())]);
    assert_that!(text_of(&r)).is_equal_to(expected);
    assert_that!(text_of(&r).as_str()).contains("Manquants");
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_widget_outside_a_pack_tab_is_refused_as_an_unknown_tool() {
    let h = start().await;
    let c = server(&h).await;
    let r = call(&c, "duplicate_widget", json!({ "from": "x" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "duplicate_widget"));
}

#[tokio::test(flavor = "multi_thread")]
async fn reorder_widgets_in_a_pack_tab_emits_changed() {
    let (h, _ws, _tab) = start_in_pack_tab().await;
    let mut events = h.hub.subscribe();
    let c = server(&h).await;
    let r = call(&c, "reorder_widgets", json!({ "order": [ACME_WIDGET] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(matches!(next_event(&mut events).await, McpEvent::Changed)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn reorder_widgets_in_a_pack_tab_moves_the_copy_first() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let copied = call(&c, "duplicate_widget", json!({ "from": ACME_WIDGET })).await;
    assert_that!(copied.is_error).is_not_equal_to(Some(true));
    assert_that!(pile_slide_ids(&ws, &tab)).is_equal_to(vec!["widget-badge".to_owned(), "widget-badge-2".to_owned()]);
    let r = call(&c, "reorder_widgets", json!({ "order": ["badge-2", ACME_WIDGET] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(pile_slide_ids(&ws, &tab)).is_equal_to(vec!["widget-badge-2".to_owned(), "widget-badge".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_widget_then_user_edit_of_the_copy_tells_the_agent() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let copied = call(&c, "duplicate_widget", json!({ "from": ACME_WIDGET })).await;
    assert_that!(copied.is_error).is_not_equal_to(Some(true));
    let edited = tokio::task::block_in_place(|| {
        Workspace::with_session(&ws, &tab, |s| {
            let source = s.widget_source(None, "badge-2")?;
            s.handle(Request::SetWidget { pack: None, name: "badge-2".into(), source: format!("{source}// v2\n") })
        })
        .0
        .unwrap_or_else(|e| panic!("tab {tab}: {e}"))
    });
    assert_that!(edited).is_ok();
    assert_that!(context_of(&ws, &tab).as_str()).contains("user edited widget badge-2 after you");
}
