//! t-611 : la carte « mémoire perdue » s'affichait à chaque session connue non reprise,
//! même quand l'agent n'annonce ni `session/load` ni `session/resume` (donc après
//! chaque arrêt de l'agent). Décision : la carte seulement quand une reprise a été
//! tentée et refusée.

use oku_slides_session::agent::{AgentScope, AgentUi};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{logged, seed_session, start, wait_error, wait_turn_ended};

fn memory_lost() -> String {
    oku_slides_i18n::Localizer::new(Some("fr")).tr("agent-memory-lost").chars().take(30).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_0611_agent_without_resume_capability_shows_no_card() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    asserting("une nouvelle session est ouverte").that(&logged(&h.log, "session/new")).has_length(1);
    let surfaces = serde_json::to_string(&t.hub.surfaces()).unwrap_or_default();
    asserting("aucune reprise tentée : pas de carte").that(&surfaces).does_not_contain(memory_lost().as_str());
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_0611_agent_restarted_without_resume_capability_shows_no_card() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }, { "crash": true }, { "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    t.agent.prompt("un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    {
        let mut rx = t.events.lock().await;
        wait_error(&mut rx).await;
    }
    t.agent.prompt("trois".into(), AgentUi::default());
    wait_turn_ended(t).await;
    asserting("une session par lancement").that(&logged(&h.log, "session/new")).has_length(2);
    let surfaces = serde_json::to_string(&t.hub.surfaces()).unwrap_or_default();
    asserting("après un arrêt, pas de carte").that(&surfaces).does_not_contain(memory_lost().as_str());
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_0611_refused_resume_shows_the_card() {
    for caps in [json!({ "resume": true }), json!({ "load_session": true })] {
        let mut script = json!({ "unknown_sessions": true, "turns": [{ "chunks": ["ok"] }] });
        if let (Some(s), Some(c)) = (script.as_object_mut(), caps.as_object()) {
            s.extend(c.clone());
        }
        let h = start(script).await;
        seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
        let t = h.tab("t-1");
        t.agent.prompt("encore".into(), AgentUi::default());
        wait_turn_ended(t).await;
        let surfaces = serde_json::to_string(&t.hub.surfaces()).unwrap_or_default();
        asserting(&format!("reprise refusée ({caps}) : la carte")).that(&surfaces).contains(memory_lost().as_str());
    }
}
