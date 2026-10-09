//! t-612 : la route d'une session reprise par `session/load` n'est posée qu'après la
//! réponse, ce qui écarte le rejeu de l'historique d'un agent conforme (ACP l'envoie
//! avant). Un agent qui l'envoie après sa réponse le faisait fuiter dans le tour
//! ouvert : garde défensive, le rejeu est ignoré tant que le premier `session/prompt`
//! de la session n'est pas parti.

use std::time::{Duration, Instant};

use oku_slides_agent::AgentEvent;
use oku_slides_session::agent::{AgentScope, AgentUi};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{logged, seed_session, sessions_in, start, wait_turn_ended};

#[tokio::test(flavor = "multi_thread")]
async fn issue_0612_replay_after_the_load_response_stays_out_of_the_turn() {
    // Réponse à `session/load` au bout de 300 ms, rejeu 100 ms plus tard.
    let h = start(json!({
        "load_session": true,
        "load_delay_ms": 300,
        "late_replay_ms": 100,
        "replay": ["ancien texte"],
        "turns": [{ "chunks": ["ok"] }],
    }))
    .await;
    seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    // Le prompt attend la session de l'onglet (`begin_turn`) : la tenir de `session/load`
    // jusqu'après le rejeu le retient, route posée et tour ouvert.
    let (session, log) = (h.session("t-1").clone(), h.log.clone());
    let holder = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while logged(&log, "session/load").is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let _held = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        std::thread::sleep(Duration::from_millis(800));
    });
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    holder.join().unwrap_or_else(|_| panic!("le fil qui tient la session a paniqué"));
    assert_that!(sessions_in(&h.log, "session/load")).is_equal_to(vec!["sess-42".to_owned()]);
    let mut chunks = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let AgentEvent::Chunk { text, .. } = e {
            chunks.push(text);
        }
    }
    asserting("le rejeu tardif n'entre pas dans le tour").that(&chunks).is_equal_to(vec!["ok".to_owned()]);
    let surfaces = serde_json::to_string(&t.hub.surfaces()).unwrap_or_default();
    assert_that!(surfaces).does_not_contain("ancien texte");
}
