//! `open_pack` follows the pack: a library pack opens in its own tab, one tab move per turn (t-1195, t-1191).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::Arc;

use common::kinds::{list_tool_names, unknown_tool};
use common::other_tabs::{OtherTabs, start_other_tabs, until};
use common::refusals::{CONTINUE_FR, LIMIT_FR, NO_TURN_FR, assert_refused};
use common::workspace_harness::{
    ACME, ACME_WIDGET, BETA, activate, active_tab, document_of, open_pack_tab, selected_item, start_in_help_tab,
    start_in_pack_tab, start_on_imported_workspace, start_on_workspace, tab_ids,
};
use common::*;
use oku_slides_i18n::Localizer;
use oku_slides_mcp::McpEvent;
use oku_slides_session::{ESSENTIEL_PACK, TabDocument, Workspace};
use rstest::rstest;
use rust_mcp_sdk::mcp_client::ClientRuntime;
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

async fn edit(c: &Arc<ClientRuntime>, pack: Uuid) -> CallToolResult {
    call(c, "open_pack", json!({ "pack": pack.to_string() })).await
}

async fn edit_main(c: &Arc<ClientRuntime>) -> CallToolResult {
    call(c, "open_pack", json!({})).await
}

/// The note the agent reads when the pack opened in tab `tab`, now shown.
fn note(l: &Localizer, tab: &str) -> String {
    l.tr_with("agent-pack-opened-in-tab", &[("tab", tab.into()), ("shown", "yes".into()), ("draft", "none".into())])
}

fn opened(tabs: &OtherTabs) -> usize {
    lock(&tabs.opened_packs).len()
}

/// The pack id `open_pack` takes without an argument on the harness's deck.
fn main_pack_of(h: &Harness) -> Uuid {
    let s = lock(&h.session);
    let snap = s.snapshot();
    snap.layouts
        .values()
        .chain(snap.widgets.values())
        .find(|p| p.id == ESSENTIEL_PACK)
        .map_or_else(|| panic!("le deck de la fixture épingle Essentiel"), |p| p.id)
}

// --- the reply -----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_a_library_pack_returns_the_tab_it_opened() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let expected = json!({ "ok": true, "tab": "t-7", "shown": true, "note": note(&h.live.current(), "t-7") });
    assert_that!(json_of(&r)).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_a_library_pack_emits_changed() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let mut rx = h.hub.subscribe();
    h.hub.begin_turn();
    edit(&c, ACME).await;
    wait_event(&mut rx, |e| matches!(e, McpEvent::Changed).then_some(())).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_a_library_pack_records_the_open() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    assert_that!(lock(&tabs.opened_packs).clone()).is_equal_to(vec![("t-1".to_owned(), ACME, None)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_without_a_pack_opens_the_deck_main_pack() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit_main(&c).await;
    let packs: Vec<Uuid> = lock(&tabs.opened_packs).iter().map(|(_, pack, _)| *pack).collect();
    assert_that!(packs).is_equal_to(vec![main_pack_of(&h)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_a_deck_only_pack_opens_its_deck_pack_tab() {
    let (h, ws) = start_on_imported_workspace("alto-wealth").await;
    let c = server(&h).await;
    let pack = Workspace::with_session(&ws, "t-1", |s| {
        s.deck().layouts.get(&oku_slides_core::Namespace::deck_layouts()).map(|p| p.id)
    })
    .0
    .ok()
    .flatten()
    .unwrap_or_else(|| panic!("pack du deck"));
    h.hub.begin_turn();
    let r = edit(&c, pack).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let tab = json_of(&r)["tab"].as_str().map_or_else(|| panic!("un onglet : {r:?}"), str::to_owned);
    assert_that!(document_of(&ws, &tab)).is_equal_to(Some(TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack }));
}

// --- one new tab per turn, only in a turn ----------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_a_second_library_pack_in_one_turn_is_refused() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = edit(&c, BETA).await;
    assert_refused(&h, &r, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(opened(&tabs)).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_the_same_pack_twice_in_one_turn_is_allowed() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = edit(&c, ACME).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(opened(&tabs)).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_another_library_pack_in_the_next_turn_is_allowed() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let first = h.hub.begin_turn();
    edit(&c, ACME).await;
    h.hub.end_turn(first, "");
    h.hub.begin_turn();
    let r = edit(&c, BETA).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(opened(&tabs)).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_outside_a_turn_is_refused() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let r = edit(&c, ACME).await;
    assert_refused(&h, &r, "agent-error-tab-no-turn", NO_TURN_FR);
    assert_that!(lock(&tabs.gate_answers).clone()).is_equal_to(vec![false]);
    assert_that!(opened(&tabs)).is_equal_to(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn start_pack_tab_edits_widgets_of_its_pack() {
    let h = start_pack_tab(oku_slides_session::BASE_PACK).await;
    let c = server(&h).await;
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let s = lock(&h.session);
    let snap = s.snapshot();
    let edited = snap.editing.as_ref().map(|e| (e.pack, e.session_version.is_some()));
    assert_that!(edited).is_equal_to(Some((oku_slides_session::BASE_PACK, true)));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_another_pack_while_the_first_is_held_is_refused() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let release = tabs.hold_next();
    let first = tokio::spawn({
        let c = c.clone();
        async move { edit(&c, ACME).await }
    });
    until("the first open reaches its hold", || opened(&tabs) == 1).await;
    let second = edit(&c, BETA).await;
    assert_refused(&h, &second, "agent-error-tab-limit", LIMIT_FR);
    release.send(()).unwrap_or_else(|e| panic!("relâche : {e}"));
    let first = first.await.unwrap_or_else(|e| panic!("premier appel : {e}"));
    assert_that!(first.is_error).is_not_equal_to(Some(true));
    assert_that!(json_of(&first)["tab"].as_str()).is_equal_to(Some("t-7"));
    assert_that!(opened(&tabs)).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_failure_releases_the_reservation() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    tabs.fail_next.store(true, std::sync::atomic::Ordering::SeqCst);
    let failed = edit(&c, ACME).await;
    assert_that!(failed.is_error).is_equal_to(Some(true));
    assert_that!(lock(&tabs.gate_answers).clone()).is_equal_to(vec![true]);
    let r = edit(&c, BETA).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(lock(&tabs.gate_answers).clone()).is_equal_to(vec![true, true]);
}

#[tokio::test(flavor = "multi_thread")]
async fn pack_tab_does_not_list_open_pack() {
    let (h, _ws, _tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let names = list_tool_names(&c).await;
    assert_that!(names.iter().any(|n| n == "open_pack" || n == "open_theme")).is_false();
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "open_pack"));
}

// --- continue in the pack tab ----------------------------------------------------------------

#[rstest]
#[case::set_widget("set_widget", json!({ "name": "_commun", "source": COMMON }))]
#[case::remove_widget("remove_widget", json!({ "name": "_commun" }))]
#[tokio::test(flavor = "multi_thread")]
async fn widget_write_after_open_pack_opened_a_tab_is_refused_this_turn(#[case] tool: &str, #[case] args: Value) {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = call(&c, tool, args).await;
    assert_refused(&h, &r, "agent-error-continue-in-other-tab", CONTINUE_FR);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_while_open_pack_is_in_flight_is_refused() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let release = tabs.hold_next();
    let first = tokio::spawn({
        let c = c.clone();
        async move { edit(&c, ACME).await }
    });
    until("the open reaches its hold", || opened(&tabs) == 1).await;
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_refused(&h, &r, "agent-error-continue-in-other-tab", CONTINUE_FR);
    release.send(()).unwrap_or_else(|e| panic!("relâche : {e}"));
    first.await.unwrap_or_else(|e| panic!("premier appel : {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_after_open_pack_focused_a_tab_is_refused_this_turn() {
    let (h, ws) = start_on_workspace().await;
    let pack_tab = open_pack_tab(&ws, "t-1", ACME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let focused = edit(&c, ACME).await;
    assert_that!(json_of(&focused)["tab"].as_str()).is_equal_to(Some(pack_tab.as_str()));
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_refused(&h, &r, "agent-error-continue-in-other-tab", CONTINUE_FR);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_after_open_pack_opened_a_tab_is_allowed_next_turn() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let first = h.hub.begin_turn();
    edit(&c, ACME).await;
    h.hub.end_turn(first, "");
    h.hub.begin_turn();
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
}

// --- on a real workspace -----------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_on_a_real_workspace_opens_a_pack_tab_beside_the_deck() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(json_of(&r)["tab"].as_str()).is_equal_to(Some("t-2"));
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned(), "t-2".to_owned()]);
    assert_that!(active_tab(&ws)).is_equal_to("t-2".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_then_open_pack_of_another_pack_in_one_turn_is_refused_by_the_limit() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = edit(&c, BETA).await;
    assert_refused(&h, &r, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_on_a_real_workspace_outside_a_turn_never_focuses() {
    let (h, ws) = start_on_workspace().await;
    open_pack_tab(&ws, "t-1", ACME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    let r = edit(&c, ACME).await;
    assert_refused(&h, &r, "agent-error-tab-no-turn", NO_TURN_FR);
    assert_that!(active_tab(&ws)).is_equal_to("t-1".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_on_a_real_workspace_in_a_turn_focuses_the_open_pack_tab() {
    let (h, ws) = start_on_workspace().await;
    let acme_tab = open_pack_tab(&ws, "t-1", ACME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(json_of(&r)["tab"].as_str()).is_equal_to(Some(acme_tab.as_str()));
    assert_that!(active_tab(&ws)).is_equal_to(acme_tab);
    // Decision 20: the focus spent the turn's one move, so a second open is refused.
    let beta = edit(&c, BETA).await;
    assert_refused(&h, &beta, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

#[rstest]
#[case::fr("fr")]
#[case::en("en")]
#[tokio::test(flavor = "multi_thread")]
async fn set_widget_on_a_pack_open_in_a_tab_tells_the_agent_the_generic_text(#[case] locale: &str) {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let added = call(&c, "add_pack", json!({ "kind": "widgets", "namespace": "acme" })).await;
    assert_that!(added.is_error).is_not_equal_to(Some(true));
    open_pack_tab(&ws, "t-1", ACME).await;
    let l = Arc::new(Localizer::new(Some(locale)));
    h.live.set(l.clone());
    let r = call(&c, "set_widget", json!({ "pack": "acme", "name": "card", "source": COMMON })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(l.tr("agent-error-pack-open-in-tab"));
    assert_that!(text_of(&r)).does_not_contain("acme");
}

#[rstest]
#[case::fr("fr")]
#[case::en("en")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_pack_opened_note_is_localized(#[case] locale: &str) {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let l = Arc::new(Localizer::new(Some(locale)));
    h.live.set(l.clone());
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(json_of(&r)["note"].as_str().map(str::to_owned)).is_equal_to(Some(note(&l, "t-7")));
    assert_that!(note(&l, "t-7")).contains("t-7");
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_a_widget_selects_it_in_the_new_tab() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let turn = h.hub.begin_turn();
    let r = call(&c, "open_pack", json!({ "pack": ACME.to_string(), "widget": ACME_WIDGET })).await;
    let tab = json_of(&r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned();
    assert_that!(selected_item(&ws, &tab)).is_equal_to(Some(ACME_WIDGET.to_owned()));
    h.hub.close_turn(turn);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_an_open_pack_tab_focuses_it_and_spends_the_turn() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let first = h.hub.begin_turn();
    let opened = edit(&c, ACME).await;
    let acme_tab = json_of(&opened)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {opened:?}")).to_owned();
    h.hub.end_turn(first, "");
    activate(&ws, "t-1").await;
    h.hub.begin_turn();
    let focused = edit(&c, ACME).await;
    assert_that!(json_of(&focused)["tab"].as_str()).is_equal_to(Some(acme_tab.as_str()));
    assert_that!(active_tab(&ws)).is_equal_to(acme_tab);
    let theme = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_refused(&h, &theme, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

/// The help tab `help`'s snapshot, as the interface reads it.
fn help_snapshot(ws: &Arc<std::sync::Mutex<Workspace>>, help: &str) -> String {
    tokio::task::block_in_place(|| {
        Workspace::with_session(ws, help, |s| serde_json::to_string(&s.snapshot()))
            .0
            .unwrap_or_else(|e| panic!("help tab: {e}"))
            .unwrap_or_else(|e| panic!("help snapshot: {e}"))
    })
}

/// A help tab's agent opens a library pack in its own tab; the read-only help deck is unchanged.
#[tokio::test(flavor = "multi_thread")]
async fn open_pack_from_a_help_tab_opens_a_pack_tab_and_leaves_the_help_unchanged() {
    let (h, ws, help) = start_in_help_tab().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let before = help_snapshot(&ws, &help);
    let r = edit(&c, ACME).await;
    let tab = json_of(&r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned();
    assert_that!(document_of(&ws, &tab)).is_equal_to(Some(TabDocument::Pack { pack: ACME }));
    assert_that!(help_snapshot(&ws, &help)).is_equal_to(before);
}

/// Without a pack, a help tab's agent opens the help deck's main pack, Essentiel.
#[tokio::test(flavor = "multi_thread")]
async fn open_pack_without_a_pack_from_a_help_tab_opens_essentiel() {
    let (h, ws, _help) = start_in_help_tab().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit_main(&c).await;
    let tab = json_of(&r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned();
    assert_that!(document_of(&ws, &tab)).is_equal_to(Some(TabDocument::Pack { pack: ESSENTIEL_PACK }));
}

/// A help tab's agent opens a library theme in its own tab, as a deck's does.
#[tokio::test(flavor = "multi_thread")]
async fn open_theme_from_a_help_tab_opens_a_theme_tab() {
    let (h, ws, _help) = start_in_help_tab().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    let tab = json_of(&r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned();
    assert_that!(document_of(&ws, &tab))
        .is_equal_to(Some(TabDocument::Theme { theme: oku_slides_session::PAPIER_THEME }));
}

// --- the user's request, passed to the moved tab's draft (t-1277) ----------------------------

const REQUEST: &str = "Agrandis la carte.";

async fn edit_with_request(c: &Arc<ClientRuntime>, pack: Uuid) -> CallToolResult {
    call(c, "open_pack", json!({ "pack": pack.to_string(), "request": REQUEST })).await
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_a_request_that_opened_a_tab_prefills_its_draft() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit_with_request(&c, ACME).await;
    assert_that!(h.drafts.calls()).is_equal_to(vec![("t-7".to_owned(), REQUEST.to_owned())]);
    assert_that!(json_of(&r)["draft"].as_str()).is_equal_to(Some("set"));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_a_request_that_focused_a_tab_prefills_its_draft() {
    let (h, ws) = start_on_workspace().await;
    let acme_tab = open_pack_tab(&ws, "t-1", ACME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit_with_request(&c, ACME).await;
    assert_that!(h.drafts.calls()).is_equal_to(vec![(acme_tab, REQUEST.to_owned())]);
}

/// A help tab's agent passes its request to the draft of the pack tab it opened.
#[tokio::test(flavor = "multi_thread")]
async fn open_pack_from_a_help_tab_with_a_request_prefills_the_new_pack_tab() {
    let (h, _ws, help) = start_in_help_tab().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit_with_request(&c, ACME).await;
    let tab = json_of(&r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned();
    assert_that!(tab.as_str()).is_not_equal_to(help.as_str());
    assert_that!(h.drafts.calls()).is_equal_to(vec![(tab, REQUEST.to_owned())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_a_request_landing_here_never_calls_the_port() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit_with_request(&c, OtherTabs::HERE_PACK).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "ok": true }));
    assert_that!(h.drafts.calls()).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_a_request_refused_by_the_limit_never_calls_the_port() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = edit_with_request(&c, BETA).await;
    assert_refused(&h, &r, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(h.drafts.calls()).is_empty();
}
