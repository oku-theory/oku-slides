//! A pack tab's own tools: selection, preview theme, copies (t-1191).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::workspace_harness::{
    ACME, ACME_WIDGET, edited_pack, library_files, next_event, preview_theme_and_dirty, selected_item,
    start_in_deck_pack_tab, start_in_pack_tab,
};
use common::*;
use oku_slides_core::Namespace;
use oku_slides_mcp::McpEvent;
use oku_slides_session::Workspace;
use oku_slides_session::agent::bounds::label;
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
