//! L'hôte lance l'agent local, qui modifie le deck par MCP et finit par un reçu.

mod common;

use oku_slides_agent::AgentEvent;
use oku_slides_session::agent::AgentUi;
use serde_json::json;
use speculoos::prelude::*;

fn edit_script() -> serde_json::Value {
    json!({"turns": [{"route": "edit", "replies": [
        {"call": "apply", "args": {"ops": [{"op": "set_deck_param", "key": "title", "value": "Feuille de route 2027"}]}},
        {"call": "report", "args": {"title": "Titre changé", "facts": ["Le titre du deck a changé"]}}
    ]}]})
}

#[tokio::test(flavor = "multi_thread")]
async fn roundtrip_local_agent_edits_deck_and_reports() {
    let h = common::start(edit_script()).await;
    h.tab.agent.prompt("renomme le deck".into(), AgentUi::default());
    let mut rx = h.tab.events.lock().await;
    common::until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    let title = h.tab.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).deck().params.title.clone();
    assert_that!(title.as_deref()).is_equal_to(Some("Feuille de route 2027"));
}

#[tokio::test(flavor = "multi_thread")]
async fn roundtrip_report_shows_in_the_turn() {
    let h = common::start(edit_script()).await;
    h.tab.agent.prompt("renomme le deck".into(), AgentUi::default());
    let mut rx = h.tab.surfaces.lock().await;
    let receipt = common::receipt(&mut rx).await;
    assert_that!(serde_json::to_string(&receipt).unwrap_or_default().contains("Titre changé")).is_true();
    assert_that!(h.tab.hub.has_report(1)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn roundtrip_failure_report_is_accepted_by_the_server() {
    let h = common::start(json!({"turns": [{"route": "nothing"}, {"route": "nothing"}]})).await;
    h.tab.agent.prompt("renomme le deck".into(), AgentUi::default());
    let mut rx = h.tab.surfaces.lock().await;
    let receipt = common::receipt(&mut rx).await;
    let title = oku_slides_i18n::Localizer::new(Some("fr")).tr("local-agent-no-route");
    assert_that!(serde_json::to_string(&receipt).unwrap_or_default().contains(&title)).is_true();
    assert_that!(h.tab.hub.has_report(1)).is_true();
}
