//! `help`, `new_conversation` and `quit`: the help tab searched, and what runs once the turn ends (t-1000).
#![expect(clippy::panic, reason = "test failures panic")]

mod common;

use std::sync::{Arc, Mutex};

use common::refusals::assert_refused;
use common::workspace_harness::{
    activate, active_tab, document_of, start_in_help_tab, start_on_agent_workspace, tab_ids,
};
use common::*;
use oku_slides_core::Op;
use oku_slides_mcp::{AfterTurn, McpEvent};
use oku_slides_session::{TabDocument, Workspace};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// The fr text of `agent-error-quit-unsaved` for one tab.
const QUIT_UNSAVED_FR: &str = "Onglets avec du travail non enregistré : 1. Oku Slides n'a pas quitté : l'utilisateur les enregistre ou les ferme d'abord.";

/// What `f` reads in tab `tab`'s session, off the async thread.
fn read_tab<R: Send>(
    ws: &Arc<Mutex<Workspace>>,
    tab: &str,
    f: impl FnOnce(&mut oku_slides_session::Session) -> R + Send,
) -> R {
    tokio::task::block_in_place(|| Workspace::with_session(ws, tab, f).0.unwrap_or_else(|e| panic!("tab {tab}: {e}")))
}

/// The slide tab `tab` shows (its kept `UiState.slide`).
fn shown_slide(ws: &Arc<Mutex<Workspace>>, tab: &str) -> Option<String> {
    read_tab(ws, tab, |s| s.snapshot().ui.slide.clone())
}

/// The id of tab `tab`'s first slide.
fn first_slide(ws: &Arc<Mutex<Workspace>>, tab: &str) -> String {
    read_tab(ws, tab, |s| s.snapshot().slides.first().map(|x| x.id.to_owned()).unwrap_or_default())
}

/// Calls `help` with `args` in an open turn; its JSON reply.
async fn help(h: &Harness, args: Value) -> Value {
    let _turn = h.hub.begin_turn();
    let c = server(h).await;
    json_of(&call(&c, "help", args).await)
}

/// The tab a `help` reply names, checked to be a help tab of the workspace.
fn help_tab_of(ws: &Arc<Mutex<Workspace>>, reply: &Value) -> String {
    let tab = reply["tab"].as_str().unwrap_or_else(|| panic!("no tab in {reply}")).to_owned();
    assert_that!(matches!(document_of(ws, &tab), Some(TabDocument::Help { .. }))).is_true();
    tab
}

#[tokio::test(flavor = "multi_thread")]
async fn help_query_opens_at_first_hit() {
    let (h, ws) = start_on_agent_workspace().await;
    let reply = help(&h, json!({ "query": "dictée" })).await;
    assert_that!(reply["ok"]).is_equal_to(json!(true));
    assert_that!(reply["shown"]).is_equal_to(json!(true));
    let tab = help_tab_of(&ws, &reply);
    let hits = reply["hits"].as_array().cloned().unwrap_or_default();
    assert_that!(hits.is_empty()).is_false();
    assert_that!(hits.len()).is_less_than_or_equal_to(oku_slides_session::agent::HELP_HITS);
    assert_that!(hits[0]["title"].as_str().is_some_and(|t| t.contains("Dictée"))).is_true();
    assert_that!(shown_slide(&ws, &tab)).is_equal_to(hits[0]["slide"].as_str().map(str::to_owned));
    assert_that!(active_tab(&ws)).is_equal_to(tab);
}

#[tokio::test(flavor = "multi_thread")]
async fn help_unknown_query_returns_no_hits_and_opens_first_slide() {
    let (h, ws) = start_on_agent_workspace().await;
    let reply = help(&h, json!({ "query": "xylophone quantique" })).await;
    assert_that!(reply["hits"]).is_equal_to(json!([]));
    let tab = help_tab_of(&ws, &reply);
    assert_that!(shown_slide(&ws, &tab)).is_equal_to(Some(first_slide(&ws, &tab)));
}

#[tokio::test(flavor = "multi_thread")]
async fn help_reused_tab_moves_to_the_new_hit() {
    let (h, ws) = start_on_agent_workspace().await;
    let first = help(&h, json!({ "query": "dictée" })).await;
    let tab = help_tab_of(&ws, &first);
    activate(&ws, "t-1").await;
    let again = help(&h, json!({ "query": "Importer un PowerPoint" })).await;
    assert_that!(again["tab"]).is_equal_to(json!(tab));
    assert_that!(shown_slide(&ws, &tab)).is_equal_to(again["hits"][0]["slide"].as_str().map(str::to_owned));
    assert_that!(tab_ids(&ws)).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn help_without_turn_refused_localized() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    let r = call(&c, "help", json!({})).await;
    assert_refused(&h, &r, "agent-error-tab-no-turn", common::refusals::NO_TURN_FR);
    assert_that!(tab_ids(&ws)).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn help_without_workspace_refused_localized() {
    let h = start().await;
    let _turn = h.hub.begin_turn();
    let c = server(&h).await;
    let r = call(&c, "help", json!({ "query": "dictée" })).await;
    let fr = "Seule l'app Oku Slides peut changer les onglets et les fenêtres ; ce n'est pas disponible ici.";
    assert_refused(&h, &r, "agent-error-workspace-unavailable", fr);
}

#[tokio::test(flavor = "multi_thread")]
async fn focus_in_help_tab_pages_it() {
    let (h, _ws, _tab) = start_in_help_tab().await;
    let mut rx = h.hub.subscribe();
    let c = server(&h).await;
    let r = call(&c, "focus", json!({ "slide": "s-agent" })).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "ok": true }));
    let slide = wait_event(&mut rx, |e| match e {
        McpEvent::Focus { slide, .. } => Some(slide),
        _ => None,
    })
    .await;
    assert_that!(slide.as_deref()).is_equal_to(Some("s-agent"));
}

#[tokio::test(flavor = "multi_thread")]
async fn help_from_the_help_tab_pages_it_to_the_first_hit() {
    let (h, ws, tab) = start_in_help_tab().await;
    let mut rx = h.hub.subscribe();
    let reply = help(&h, json!({ "query": "dictée" })).await;
    assert_that!(reply["tab"]).is_equal_to(json!(tab));
    let slide = wait_event(&mut rx, |e| match e {
        McpEvent::Focus { slide, .. } => Some(slide),
        _ => None,
    })
    .await;
    assert_that!(slide).is_equal_to(reply["hits"][0]["slide"].as_str().map(str::to_owned));
    assert_that!(tab_ids(&ws)).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn help_from_the_help_tab_claims_no_tab_move() {
    let (h, ws, tab) = start_in_help_tab().await;
    let _turn = h.hub.begin_turn();
    let c = server(&h).await;
    let first = json_of(&call(&c, "help", json!({ "query": "dictée" })).await);
    let again = json_of(&call(&c, "help", json!({ "query": "Importer un PowerPoint" })).await);
    assert_that!(first["tab"]).is_equal_to(json!(tab));
    assert_that!(again["tab"]).is_equal_to(json!(tab));
    // The turn's one tab move is still free: activating t-1 takes it.
    let moved = call(&c, "tab", json!({ "action": "activate", "tab": "t-1" })).await;
    assert_that!(moved.is_error).is_not_equal_to(Some(true));
    assert_that!(active_tab(&ws)).is_equal_to("t-1".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn help_reused_tab_pages_its_live_page_to_the_new_hit() {
    let (h, ws) = start_on_agent_workspace().await;
    let first = help(&h, json!({ "query": "dictée" })).await;
    let tab = help_tab_of(&ws, &first);
    activate(&ws, "t-1").await;
    let again = help(&h, json!({ "query": "Importer un PowerPoint" })).await;
    let hit = again["hits"][0]["slide"].as_str().unwrap_or_else(|| panic!("no hit in {again}")).to_owned();
    assert_that!(h.drafts.pages()).is_equal_to(vec![(tab, hit)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn help_reused_tab_with_no_page_replies_not_paged() {
    let (h, ws) = start_on_agent_workspace().await;
    let first = help(&h, json!({ "query": "dictée" })).await;
    let _ = help_tab_of(&ws, &first);
    activate(&ws, "t-1").await;
    *lock(&h.drafts.no_page) = true;
    let again = help(&h, json!({ "query": "Importer un PowerPoint" })).await;
    assert_that!(again["paged"].clone()).is_equal_to(json!(false));
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_runs_after_turn() {
    let h = start().await;
    let turn = h.hub.begin_turn();
    let c = server(&h).await;
    let r = call(&c, "new_conversation", json!({})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "new_conversation": "after_turn" }));
    assert_that!(h.hub.take_after_turn(turn)).is_equal_to(vec![AfterTurn::NewConversation]);
}

/// Gives tab `tab`'s deck the title `title`: an edit, so the tab holds unsaved work.
async fn retitle(ws: &Arc<Mutex<Workspace>>, tab: &str, title: &str) {
    let (ws, tab, title) = (ws.clone(), tab.to_owned(), title.to_owned());
    tokio::task::spawn_blocking(move || {
        let op = Op::SetDeckParam { key: "title".into(), value: json!(title) };
        Workspace::with_session(&ws, &tab, |s| s.apply("Titre", vec![op], None))
            .0
            .unwrap_or_else(|e| panic!("tab {tab}: {e}"))
            .unwrap_or_else(|e| panic!("apply: {e}"));
    })
    .await
    .unwrap_or_else(|e| panic!("workspace task: {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn quit_with_dirty_tab_refused_localized() {
    let (h, ws) = start_on_agent_workspace().await;
    retitle(&ws, "t-1", "Secret de la réunion").await;
    let turn = h.hub.begin_turn();
    let c = server(&h).await;
    let r = call(&c, "quit", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr_with("agent-error-quit-unsaved", &[("count", 1.into())]));
    assert_that!(text_of(&r)).is_equal_to(QUIT_UNSAVED_FR.to_owned());
    // Never a tab title: a count only.
    assert_that!(text_of(&r).contains("Secret")).is_false();
    assert_that!(open_ids(&h)).is_empty();
    assert_that!(h.hub.take_after_turn(turn)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn quit_asks_then_runs_after_turn() {
    let h = start().await;
    let turn = h.hub.begin_turn();
    let mut rx = h.hub.subscribe();
    let c = server(&h).await;
    let pending = tokio::spawn(async move { call(&c, "quit", json!({})).await });
    let surface = next_surface(&mut rx).await;
    let (message, danger) = confirmation_of(&surface);
    let yes = confirm_button_label(&surface);
    answer(&h, &surface.id, true);
    let r = pending.await.unwrap_or_else(|e| panic!("tool task: {e}"));
    assert_that!(message).is_equal_to(h.localizer.tr("agent-confirm-quit"));
    assert_that!(danger).is_true();
    // The button, and the word said to confirm, is ConfirmVerb::Quit's.
    assert_that!(yes).is_equal_to(h.localizer.tr("agent-surface-verb-quit"));
    assert_that!(json_of(&r)).is_equal_to(json!({ "quit": "after_turn" }));
    // The tool itself never quits: the stack does, once the turn has ended.
    assert_that!(h.app.quits()).is_equal_to(0);
    assert_that!(h.hub.take_after_turn(turn)).is_equal_to(vec![AfterTurn::Quit]);
}

#[tokio::test(flavor = "multi_thread")]
async fn quit_denied_records_nothing() {
    let h = start().await;
    let turn = h.hub.begin_turn();
    let r = guarded(&h, "quit", json!({}), false).await;
    assert_refused(&h, &r, "agent-error-denied", "L'utilisateur a refusé.");
    assert_that!(h.hub.take_after_turn(turn)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn quit_on_serve_unavailable_localized() {
    let h = start_on_serve().await;
    let turn = h.hub.begin_turn();
    let c = server(&h).await;
    let r = call(&c, "quit", json!({})).await;
    assert_refused(
        &h,
        &r,
        "agent-error-app-unavailable",
        "Seule l'app Oku Slides peut le faire ; ce n'est pas disponible ici.",
    );
    assert_that!(open_ids(&h)).is_empty();
    assert_that!(h.hub.take_after_turn(turn)).is_empty();
}
