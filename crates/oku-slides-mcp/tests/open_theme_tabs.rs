//! `open_theme` opens a library theme in its own tab from a deck tab, one move per turn (t-1191).
#![expect(clippy::panic, reason = "test failures panic")]

mod common;

use common::kinds::list_tool_names;
use common::other_tabs::{start_other_tabs, until};
use common::refusals::{CONTINUE_FR, LIMIT_FR, NO_TURN_FR, assert_refused};
use common::workspace_harness::{
    ACME, activate, active_tab, document_of, new_blank_tab, open_theme_tab, start_on_workspace, tab_ids,
};
use common::*;
use oku_slides_mcp::{MAX_REQUEST_BYTES, Prefill};
use oku_slides_session::{PAPIER_THEME, TabDocument};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

/// The note the agent reads when the theme opened in tab `tab`, shown or not, without a request.
fn note(h: &Harness, tab: &str, shown: bool) -> String {
    note_with(h, tab, shown, "none")
}

/// [`note`], the request's draft outcome being `draft`.
fn note_with(h: &Harness, tab: &str, shown: bool, draft: &str) -> String {
    let shown = if shown { "yes" } else { "no" };
    let args = [("tab", tab.into()), ("shown", shown.into()), ("draft", draft.into())];
    h.localizer.tr_with("agent-theme-opened-in-tab", &args)
}

/// The tab a successful open names.
fn tab_of(r: &rust_mcp_sdk::schema::CallToolResult) -> String {
    json_of(r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_in_a_turn_opens_a_theme_tab_after_the_deck_and_activates_it() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let _turn = h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    let tab = tab_of(&r);
    assert_that!(document_of(&ws, &tab)).is_equal_to(Some(TabDocument::Theme { theme: PAPIER_THEME }));
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned(), tab.clone()]);
    assert_that!(active_tab(&ws)).is_equal_to(tab.clone());
    assert_that!(json_of(&r)["note"].as_str().map(str::to_owned)).is_equal_to(Some(note(&h, &tab, true)));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_outside_a_turn_is_refused_and_opens_nothing() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_refused(&h, &r, "agent-error-tab-no-turn", NO_TURN_FR);
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_after_open_pack_in_one_turn_is_refused_by_the_limit() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let pack = call(&c, "open_pack", json!({ "pack": ACME.to_string() })).await;
    assert_that!(pack.is_error).is_not_equal_to(Some(true));
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_refused(&h, &r, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_of_an_open_theme_tab_focuses_it_without_a_new_tab() {
    let (h, ws) = start_on_workspace().await;
    let theme_tab = open_theme_tab(&ws, "t-1", PAPIER_THEME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_that!(tab_of(&r)).is_equal_to(theme_tab.clone());
    assert_that!(active_tab(&ws)).is_equal_to(theme_tab);
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
    assert_that!(json_of(&r)["shown"].as_bool()).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_from_a_background_deck_tab_opens_it_unshown() {
    let (h, ws) = start_on_workspace().await;
    let blank = new_blank_tab(&ws).await;
    assert_that!(active_tab(&ws)).is_equal_to(blank.clone());
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    let tab = tab_of(&r);
    assert_that!(json_of(&r)["shown"].as_bool()).is_equal_to(Some(false));
    assert_that!(active_tab(&ws)).is_equal_to(blank);
    assert_that!(json_of(&r)["note"].as_str().map(str::to_owned)).is_equal_to(Some(note(&h, &tab, false)));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_of_an_unknown_theme_is_refused_before_the_gate() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "nope" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr_with("session-unknown-theme", &[("theme", "nope".into())]));
    assert_that!(text_of(&r)).contains("Thème inconnu");
    // No claim was spent: the turn can still open a pack.
    let pack = call(&c, "open_pack", json!({ "pack": ACME.to_string() })).await;
    assert_that!(pack.is_error).is_not_equal_to(Some(true));
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_after_open_theme_in_the_same_turn_is_refused() {
    let (h, _ws) = start_on_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let opened = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_that!(opened.is_error).is_not_equal_to(Some(true));
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_refused(&h, &r, "agent-error-continue-in-other-tab", CONTINUE_FR);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_by_id_and_by_name_opens_the_same_tab() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let first = h.hub.begin_turn();
    let by_id = call(&c, "open_theme", json!({ "theme": PAPIER_THEME.to_string() })).await;
    let tab = tab_of(&by_id);
    h.hub.end_turn(first, "");
    h.hub.begin_turn();
    let by_name = call(&c, "open_theme", json!({ "theme": "Papier" })).await;
    assert_that!(tab_of(&by_name)).is_equal_to(tab);
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_does_not_list_open_theme() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = server(&h).await;
    let names = list_tool_names(&c).await;
    assert_that!(names.iter().any(|n| n == "open_theme" || n == "open_pack")).is_false();
}

/// The one gate is shared: a pack open held in flight leaves no move for a concurrent theme open.
#[tokio::test(flavor = "multi_thread")]
async fn open_theme_while_open_pack_is_in_flight_in_the_same_turn_is_refused_by_the_limit() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let release = tabs.hold_next();
    let pack = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "open_pack", json!({ "pack": ACME.to_string() })).await }
    });
    until("the pack open reaches its hold", || lock(&tabs.opened_packs).len() == 1).await;
    let theme = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    release.send(()).unwrap_or_else(|e| panic!("release: {e}"));
    let pack = pack.await.unwrap_or_else(|e| panic!("pack call: {e}"));
    assert_that!(pack.is_error).is_not_equal_to(Some(true));
    assert_refused(&h, &theme, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(lock(&tabs.opened_packs).len() + tabs.opened_themes().len()).is_equal_to(1);
}

// --- the user's request, passed to the opened tab's draft (t-1277) ---------------------------

const REQUEST: &str = "Passe les titres en bleu.";

/// `open_theme` of Papier among other tabs (it opens `t-9`), with `request` as given.
async fn open_with(request: Value) -> (Harness, rust_mcp_sdk::schema::CallToolResult) {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier", "request": request })).await;
    (h, r)
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_that_opened_a_tab_prefills_its_draft() {
    let (h, r) = open_with(json!(REQUEST)).await;
    assert_that!(h.drafts.calls()).is_equal_to(vec![("t-9".to_owned(), REQUEST.to_owned())]);
    assert_that!(json_of(&r)["draft"].as_str()).is_equal_to(Some("set"));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_that_focused_a_tab_prefills_its_draft() {
    let (h, ws) = start_on_workspace().await;
    let theme_tab = open_theme_tab(&ws, "t-1", PAPIER_THEME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier", "request": REQUEST })).await;
    assert_that!(tab_of(&r)).is_equal_to(theme_tab.clone());
    assert_that!(h.drafts.calls()).is_equal_to(vec![(theme_tab, REQUEST.to_owned())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_outside_a_turn_never_calls_the_port() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let r = call(&c, "open_theme", json!({ "theme": "papier", "request": REQUEST })).await;
    assert_refused(&h, &r, "agent-error-tab-no-turn", NO_TURN_FR);
    assert_that!(h.drafts.calls()).is_empty();
}

#[rstest]
#[case::omitted(Value::Null)]
#[case::empty(json!(""))]
#[case::whitespace_only(json!("  \n\t "))]
#[tokio::test(flavor = "multi_thread")]
async fn open_theme_without_request_text_never_calls_the_port_nor_adds_draft(#[case] request: Value) {
    let (h, r) = open_with(request).await;
    assert_that!(h.drafts.calls()).is_empty();
    assert_that!(json_of(&r).get("draft")).is_none();
    assert_that!(json_of(&r)["note"].as_str().map(str::to_owned)).is_equal_to(Some(note(&h, "t-9", true)));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_of_exactly_the_cap_prefills_it() {
    let request = "a".repeat(MAX_REQUEST_BYTES);
    let (h, r) = open_with(json!(request)).await;
    assert_that!(h.drafts.calls()).is_equal_to(vec![("t-9".to_owned(), request)]);
    assert_that!(json_of(&r)["draft"].as_str()).is_equal_to(Some("set"));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_landing_here_never_calls_the_port_nor_spends_the_move() {
    let (h, tabs) = start_other_tabs().await;
    tabs.theme_here_next.store(true, std::sync::atomic::Ordering::SeqCst);
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier", "request": REQUEST })).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "ok": true }));
    assert_that!(h.drafts.calls()).is_empty();
    // Nothing moved: the turn can still open a pack.
    let pack = call(&c, "open_pack", json!({ "pack": ACME.to_string() })).await;
    assert_that!(pack.is_error).is_not_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_over_the_cap_opens_the_tab_and_reports_too_long() {
    let (h, r) = open_with(json!("a".repeat(MAX_REQUEST_BYTES + 1))).await;
    assert_that!(json_of(&r)["ok"].as_bool()).is_equal_to(Some(true));
    assert_that!(tab_of(&r)).is_equal_to("t-9".to_owned());
    assert_that!(json_of(&r)["draft"].as_str()).is_equal_to(Some("too_long"));
    assert_that!(h.drafts.calls()).is_empty();
}

/// 1025 × "é": 1025 characters, under the cap, but 2050 bytes, over it.
#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_multibyte_request_counts_its_bytes() {
    let (h, r) = open_with(json!("é".repeat(1025))).await;
    assert_that!(json_of(&r)["draft"].as_str()).is_equal_to(Some("too_long"));
    assert_that!(h.drafts.calls()).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_control_and_bidi_characters_passes_them_stripped() {
    let (h, _r) = open_with(json!(" Passe\u{7} les titres\u{202e} en\u{2066} bleu.\r ")).await;
    assert_that!(h.drafts.calls()).is_equal_to(vec![("t-9".to_owned(), REQUEST.to_owned())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_never_echoes_its_text() {
    let (_h, r) = open_with(json!(REQUEST)).await;
    assert_that!(text_of(&r)).does_not_contain("titres");
}

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_prefilled_draft_tells_the_note_in_french() {
    let (h, r) = open_with(json!(REQUEST)).await;
    let note = json_of(&r)["note"].as_str().map(str::to_owned);
    assert_that!(note).is_equal_to(Some(note_with(&h, "t-9", true, "set")));
    assert_that!(note_with(&h, "t-9", true, "set")).contains("brouillon");
}

#[rstest]
#[case::kept(Prefill::Kept, "kept")]
#[case::unavailable(Prefill::Unavailable, "unavailable")]
#[tokio::test(flavor = "multi_thread")]
async fn open_theme_with_a_request_reports_the_port_answer(#[case] answer: Prefill, #[case] draft: &str) {
    let (h, _tabs) = start_other_tabs().await;
    *lock(&h.drafts.answer) = Some(answer);
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier", "request": REQUEST })).await;
    assert_that!(json_of(&r)["draft"].as_str()).is_equal_to(Some(draft));
    assert_that!(json_of(&r)["note"].as_str().map(str::to_owned)).is_equal_to(Some(note_with(&h, "t-9", true, draft)));
}
