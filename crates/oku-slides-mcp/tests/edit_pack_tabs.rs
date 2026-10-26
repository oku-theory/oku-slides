//! `edit_pack` follows the pack: a library pack opens in its own tab, once per turn (t-1195).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::workspace_harness::{
    ACME, BETA, activate, active_tab, open_pack_tab, start_in_pack_tab, start_on_workspace,
};
use common::*;
use oku_slides_i18n::Localizer;
use oku_slides_mcp::McpEvent;
use oku_slides_session::ESSENTIEL_PACK;
use rstest::rstest;
use rust_mcp_sdk::mcp_client::ClientRuntime;
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

/// The fr texts of the new refusals, as the user's language shows them.
const NO_TURN_FR: &str = "N'ouvrez un onglet de pack ou n'y passez qu'en répondant à l'utilisateur.";
const LIMIT_FR: &str = "Vous avez déjà ouvert un onglet de pack dans ce tour ; terminez-y, ou demandez à l'utilisateur avant d'en ouvrir un autre.";
const CONTINUE_FR: &str = "Cette demande se poursuit dans l'onglet du pack : demandez à l'utilisateur d'y continuer ; cet onglet ne modifie plus de widget pour l'instant.";

async fn edit(c: &Arc<ClientRuntime>, pack: Uuid) -> CallToolResult {
    call(c, "edit_pack", json!({ "pack": pack.to_string() })).await
}

async fn edit_main(c: &Arc<ClientRuntime>) -> CallToolResult {
    call(c, "edit_pack", json!({})).await
}

/// `r` is a refusal that says `key` (and `fr` in French).
fn assert_refused(h: &Harness, r: &CallToolResult, key: &str, fr: &str) {
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(r)).is_equal_to(h.localizer.tr(key));
    assert_that!(text_of(r)).is_equal_to(fr.to_owned());
}

/// The note the agent reads when the pack opened in tab `tab`.
fn note(l: &Localizer, tab: &str) -> String {
    l.tr_with("agent-pack-opened-in-tab", &[("tab", tab.into())])
}

/// Polls until `done`, at most 10 s.
async fn until(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert_that!(Instant::now() < deadline).named(what).is_true();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn opened(tabs: &OtherTabs) -> usize {
    lock(&tabs.opened_packs).len()
}

/// The pack id `edit_pack` takes without an argument on the harness's deck.
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
async fn edit_pack_of_a_library_pack_returns_the_tab_it_opened() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let expected = json!({ "ok": true, "tab": "t-7", "note": note(&h.live.current(), "t-7") });
    assert_that!(json_of(&r)).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_of_a_library_pack_emits_changed() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let mut rx = h.hub.subscribe();
    h.hub.begin_turn();
    edit(&c, ACME).await;
    wait_event(&mut rx, |e| matches!(e, McpEvent::Changed).then_some(())).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_of_a_library_pack_records_the_open() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    assert_that!(lock(&tabs.opened_packs).clone()).is_equal_to(vec![("t-1".to_owned(), ACME, None)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_without_a_pack_opens_the_deck_main_pack() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit_main(&c).await;
    let packs: Vec<Uuid> = lock(&tabs.opened_packs).iter().map(|(_, pack, _)| *pack).collect();
    assert_that!(packs).is_equal_to(vec![main_pack_of(&h)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_without_a_pack_in_a_pack_tab_stays_on_the_edited_pack() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit_main(&c).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(json_of(&r)).is_equal_to(json!({ "ok": true }));
    assert_that!(common::workspace_harness::tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned(), tab.clone()]);
    assert_that!(active_tab(&ws)).is_equal_to(tab);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_of_a_deck_only_pack_stays_in_this_tab() {
    let h = start_imported("alto-wealth", hifi(), |_| {}).await;
    let c = server(&h).await;
    let pack = lock(&h.session)
        .deck()
        .layouts
        .get(&oku_slides_core::Namespace::deck_layouts())
        .map_or_else(|| panic!("pack du deck"), |p| p.id);
    let r = edit(&c, pack).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(json!({ "ok": true }).to_string());
}

// --- one new tab per turn, only in a turn ----------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_of_a_second_library_pack_in_one_turn_is_refused() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = edit(&c, BETA).await;
    assert_refused(&h, &r, "agent-error-pack-tab-limit", LIMIT_FR);
    assert_that!(opened(&tabs)).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_of_the_same_pack_twice_in_one_turn_is_allowed() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = edit(&c, ACME).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(opened(&tabs)).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_of_another_library_pack_in_the_next_turn_is_allowed() {
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
async fn edit_pack_outside_a_turn_is_refused() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let r = edit(&c, ACME).await;
    assert_refused(&h, &r, "agent-error-pack-tab-no-turn", NO_TURN_FR);
    assert_that!(lock(&tabs.gate_answers).clone()).is_equal_to(vec![false]);
    assert_that!(opened(&tabs)).is_equal_to(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_in_place_needs_no_turn() {
    let h = start().await;
    let c = server(&h).await;
    let r = call(&c, "edit_pack", json!({ "pack": ESSENTIEL_PACK })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(json_of(&r)).is_equal_to(json!({ "ok": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_of_another_pack_while_the_first_is_held_is_refused() {
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
    assert_refused(&h, &second, "agent-error-pack-tab-limit", LIMIT_FR);
    release.send(()).unwrap_or_else(|e| panic!("relâche : {e}"));
    let first = first.await.unwrap_or_else(|e| panic!("premier appel : {e}"));
    assert_that!(first.is_error).is_not_equal_to(Some(true));
    assert_that!(json_of(&first)["tab"].as_str()).is_equal_to(Some("t-7"));
    assert_that!(opened(&tabs)).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_failure_releases_the_reservation() {
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

// --- continue in the pack tab ----------------------------------------------------------------

#[rstest]
#[case::set_widget("set_widget", json!({ "name": "_commun", "source": COMMON }))]
#[case::remove_widget("remove_widget", json!({ "name": "_commun" }))]
#[tokio::test(flavor = "multi_thread")]
async fn widget_write_after_edit_pack_opened_a_tab_is_refused_this_turn(#[case] tool: &str, #[case] args: Value) {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = call(&c, tool, args).await;
    assert_refused(&h, &r, "agent-error-continue-in-pack-tab", CONTINUE_FR);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_while_edit_pack_is_in_flight_is_refused() {
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
    assert_refused(&h, &r, "agent-error-continue-in-pack-tab", CONTINUE_FR);
    release.send(()).unwrap_or_else(|e| panic!("relâche : {e}"));
    first.await.unwrap_or_else(|e| panic!("premier appel : {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_after_edit_pack_focused_a_tab_is_refused_this_turn() {
    let (h, ws) = start_on_workspace().await;
    let pack_tab = open_pack_tab(&ws, "t-1", ACME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let focused = edit(&c, ACME).await;
    assert_that!(json_of(&focused)["tab"].as_str()).is_equal_to(Some(pack_tab.as_str()));
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_refused(&h, &r, "agent-error-continue-in-pack-tab", CONTINUE_FR);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_after_edit_pack_opened_a_tab_is_allowed_next_turn() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let first = h.hub.begin_turn();
    edit(&c, ACME).await;
    h.hub.end_turn(first, "");
    h.hub.begin_turn();
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_after_edit_pack_in_place_is_allowed() {
    let h = start().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let opened = call(&c, "edit_pack", json!({ "pack": ESSENTIEL_PACK })).await;
    assert_that!(opened.is_error).is_not_equal_to(Some(true));
    let r = call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_after_edit_pack_opened_a_tab_is_not_refused_for_the_pack_tab() {
    let (h, _tabs) = start_other_tabs().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = call(&c, "set_widget_trial", json!({ "name": "x", "values": {} })).await;
    assert_that!(text_of(&r)).is_not_equal_to(h.localizer.tr("agent-error-continue-in-pack-tab"));
}

// --- on a real workspace -----------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_on_a_real_workspace_opens_a_pack_tab_beside_the_deck() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(json_of(&r)["tab"].as_str()).is_equal_to(Some("t-2"));
    assert_that!(common::workspace_harness::tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned(), "t-2".to_owned()]);
    assert_that!(active_tab(&ws)).is_equal_to("t-2".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_on_a_real_workspace_refuses_a_second_new_tab_in_one_turn() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    edit(&c, ACME).await;
    let r = edit(&c, BETA).await;
    assert_refused(&h, &r, "agent-error-pack-tab-limit", LIMIT_FR);
    assert_that!(common::workspace_harness::tab_ids(&ws).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_on_a_real_workspace_outside_a_turn_never_focuses() {
    let (h, ws) = start_on_workspace().await;
    open_pack_tab(&ws, "t-1", ACME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    let r = edit(&c, ACME).await;
    assert_refused(&h, &r, "agent-error-pack-tab-no-turn", NO_TURN_FR);
    assert_that!(active_tab(&ws)).is_equal_to("t-1".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_on_a_real_workspace_in_a_turn_focuses_the_open_pack_tab() {
    let (h, ws) = start_on_workspace().await;
    let acme_tab = open_pack_tab(&ws, "t-1", ACME).await;
    activate(&ws, "t-1").await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = edit(&c, ACME).await;
    assert_that!(json_of(&r)["tab"].as_str()).is_equal_to(Some(acme_tab.as_str()));
    assert_that!(active_tab(&ws)).is_equal_to(acme_tab);
    // Focusing consumed no claim: this turn can still open one new pack tab.
    let beta = edit(&c, BETA).await;
    assert_that!(beta.is_error).is_not_equal_to(Some(true));
    assert_that!(common::workspace_harness::tab_ids(&ws).len()).is_equal_to(3);
}

#[tokio::test(flavor = "multi_thread")]
async fn close_pack_in_a_pack_tab_is_refused_for_the_agent() {
    let (h, _ws, _tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let r = call(&c, "close_pack", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-in-pack-tab"));
    assert_that!(text_of(&r)).starts_with("Cet onglet édite un pack");
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
