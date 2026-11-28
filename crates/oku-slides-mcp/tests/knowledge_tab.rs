//! `open_knowledge`: the deck's knowledge tab opened or shown, the turn's one tab move (t-1786).
#![expect(clippy::panic, reason = "test failures panic")]

mod common;

use std::sync::Arc;

use common::refusals::{LIMIT_FR, NO_TURN_FR, assert_refused};
use common::workspace_harness::{
    activate, active_tab, document_of, start_in_help_tab, start_on_agent_workspace, tab_ids,
};
use common::*;
use oku_slides_session::TabDocument;
use oku_slides_session::agent::{AgentWorkspace, FocusedTab, TabOpen, WorkspaceRefusal};
use serde_json::json;
use speculoos::prelude::*;

/// The fr text of `agent-error-workspace-unknown-tab`.
const UNKNOWN_TAB_FR: &str = "Cette fenêtre n'a pas cet onglet ou cette fenêtre.";

/// A host that answers with an outcome no knowledge tab produces.
#[derive(Debug)]
struct Answering(fn() -> TabOpen);

impl AgentWorkspace for Answering {
    fn focused_tab(&self) -> Option<FocusedTab> {
        None
    }

    fn unsaved_tabs(&self) -> usize {
        0
    }

    fn open_knowledge(&self, _: &str) -> Result<TabOpen, WorkspaceRefusal> {
        Ok((self.0)())
    }
}

#[rstest::rstest]
#[case::here(|| TabOpen::Here)]
#[case::refused(|| TabOpen::Refused)]
#[tokio::test(flavor = "multi_thread")]
async fn open_knowledge_outcome_without_a_tab_is_told_as_unknown_tab(#[case] answer: fn() -> TabOpen) {
    let h = start_with_port(Arc::new(Answering(answer))).await;
    let _turn = h.hub.begin_turn();
    let c = server(&h).await;
    let r = call(&c, "open_knowledge", json!({})).await;
    assert_refused(&h, &r, "agent-error-workspace-unknown-tab", UNKNOWN_TAB_FR);
}

/// A help tab's agent opens no knowledge tab: only a deck tab has one.
#[tokio::test(flavor = "multi_thread")]
async fn open_knowledge_from_a_help_tab_opens_nothing() {
    let (h, ws, _help) = start_in_help_tab().await;
    let before = tab_ids(&ws);
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call(&c, "open_knowledge", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(tab_ids(&ws)).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_knowledge_deck_tab_opens_and_claims_move() {
    let (h, ws) = start_on_agent_workspace().await;
    let _turn = h.hub.begin_turn();
    let c = server(&h).await;
    let reply = json_of(&call(&c, "open_knowledge", json!({})).await);
    assert_that!(reply["ok"]).is_equal_to(json!(true));
    assert_that!(reply["shown"]).is_equal_to(json!(true));
    let tab = reply["tab"].as_str().unwrap_or_else(|| panic!("no tab in {reply}")).to_owned();
    assert_that!(document_of(&ws, &tab)).is_equal_to(Some(TabDocument::Knowledge { deck_tab: "t-1".to_owned() }));
    assert_that!(active_tab(&ws)).is_equal_to(tab.clone());
    // The turn's one move is spent: another tab change is refused, a repeat of the same one is not.
    let moved = call(&c, "tab", json!({ "action": "activate", "tab": "t-1" })).await;
    assert_refused(&h, &moved, "agent-error-tab-limit", LIMIT_FR);
    let again = json_of(&call(&c, "open_knowledge", json!({})).await);
    assert_that!(again["tab"]).is_equal_to(json!(tab));
    assert_that!(tab_ids(&ws)).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_knowledge_reopened_focuses_the_same_tab() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let first = json_of(&call(&c, "open_knowledge", json!({})).await);
    activate(&ws, "t-1").await;
    h.hub.begin_turn();
    let again = json_of(&call(&c, "open_knowledge", json!({})).await);
    assert_that!(again["tab"]).is_equal_to(first["tab"].clone());
    assert_that!(again["shown"]).is_equal_to(json!(true));
    assert_that!(tab_ids(&ws)).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_knowledge_second_move_refused_tab_gate() {
    let (h, ws) = start_on_agent_workspace().await;
    let _turn = h.hub.begin_turn();
    let c = server(&h).await;
    let help = call(&c, "help", json!({})).await;
    assert_that!(help.is_error).is_not_equal_to(Some(true));
    let r = call(&c, "open_knowledge", json!({})).await;
    assert_refused(&h, &r, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(tab_ids(&ws)).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_knowledge_no_turn_refused() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    let r = call(&c, "open_knowledge", json!({})).await;
    assert_refused(&h, &r, "agent-error-tab-no-turn", NO_TURN_FR);
    assert_that!(tab_ids(&ws)).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_knowledge_unavailable_workspace_refused() {
    let h = start().await;
    let _turn = h.hub.begin_turn();
    let c = server(&h).await;
    let r = call(&c, "open_knowledge", json!({})).await;
    let fr = "Seule l'app Oku Slides peut changer les onglets et les fenêtres ; ce n'est pas disponible ici.";
    assert_refused(&h, &r, "agent-error-workspace-unavailable", fr);
}
