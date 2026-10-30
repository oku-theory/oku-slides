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
use oku_slides_session::{PAPIER_THEME, TabDocument};
use serde_json::json;
use speculoos::prelude::*;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

/// The note the agent reads when the theme opened in tab `tab`, shown or not.
fn note(h: &Harness, tab: &str, shown: bool) -> String {
    let shown = if shown { "yes" } else { "no" };
    h.localizer.tr_with("agent-theme-opened-in-tab", &[("tab", tab.into()), ("shown", shown.into())])
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
