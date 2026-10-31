//! The `usage` and `show_usage` tools: the linked deck's ledger, as JSON or a thread box.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use std::sync::Arc;

use common::workspace_harness::start_in_pack_tab;
use common::*;
use oku_slides_core::{Range, UsageRecord};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::PAPIER_THEME;
use oku_slides_session::agent::{Sessions, TabSession};
use serde_json::json;
use speculoos::prelude::*;

/// A deck turn, as a ledger line holds it.
fn record(n: u128) -> UsageRecord {
    serde_json::from_value(json!({
        "turn": uuid::Uuid::from_u128(n), "at": "2026-10-30T14:02:11Z", "tab": "deck", "agent": "claude",
        "model": "claude-opus-5-5",
        "tokens": { "total": 1850, "input": 1840, "output": 10, "thought": null, "cache_read": 5, "cache_write": null },
        "cost": { "amount": 0.04, "currency": "USD" },
    }))
    .unwrap_or_else(|e| panic!("test record: {e}"))
}

/// Records `n` in `tab`'s deck, off the async thread as the agent stack does.
async fn record_in(tab: TabSession, n: u128) {
    tokio::task::spawn_blocking(move || tab.record_usage(record(n)))
        .await
        .unwrap_or_else(|e| panic!("record task: {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_on_a_deck_tab_reports_its_recorded_turns() {
    let h = start().await;
    record_in(h.tab.clone(), 1).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = json_of(&call(&c, "usage", json!({ "by": "model" })).await);
    assert_that!(r["by"].as_str()).is_equal_to(Some("model"));
    assert_that!(r["totals"]["turns"].as_u64()).is_equal_to(Some(1));
    assert_that!(r["rows"][0]["key"].as_str()).is_equal_to(Some("claude-opus-5-5"));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_since_after_until_is_refused_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "usage", json!({ "since": "2026-10-31", "until": "2026-10-30" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    // The harness localizer is fr: the core error's own French text.
    let expected =
        Range::parse(Some("2026-10-31"), Some("2026-10-30")).unwrap_err().localize(&Localizer::new(Some("fr")));
    assert_that!(text_of(&r)).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_malformed_day_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "usage", json!({ "since": "30/10/2026" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    // The bad-date refusal in the harness's French, naming the agent's text.
    let expected = Range::parse(Some("30/10/2026"), None).unwrap_err().localize(&Localizer::new(Some("fr")));
    assert_that!(text_of(&r)).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_in_a_pack_tab_opened_from_a_deck_reads_that_deck() {
    let (h, ws, _tab) = start_in_pack_tab().await;
    let sessions: Arc<dyn Sessions> = ws.clone();
    record_in(TabSession::new("t-1", sessions), 1).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = json_of(&call(&c, "usage", json!({})).await);
    assert_that!(r["totals"]["turns"].as_u64()).is_equal_to(Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_in_a_theme_tab_without_deck_is_refused_no_deck() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "usage", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(Localizer::new(Some("fr")).tr("agent-error-usage-no-deck"));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_usage_files_its_box_in_the_open_turn() {
    let h = start().await;
    record_in(h.tab.clone(), 1).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn();
    let r = json_of(&call(&c, "show_usage", json!({})).await);
    let id = r["shown"].as_str().unwrap().to_owned();
    assert_that!(id.as_str()).starts_with("usage");
    assert_that!(open_ids(&h)).contains(id);
}

#[tokio::test(flavor = "multi_thread")]
async fn show_usage_outside_a_turn_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "show_usage", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let fr = Localizer::new(Some("fr"));
    assert_that!(text_of(&r)).is_equal_to(fr.tr_with("agent-error-no-turn", &[("tool", "show_usage".into())]));
    assert_that!(open_ids(&h)).is_empty();
}
