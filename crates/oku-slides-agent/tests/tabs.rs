//! Plusieurs onglets sur un seul agent : une session ACP chacun, tours simultanés.

mod common;

use std::time::Duration;

use common::*;
use oku_slides_agent::{AgentEvent, AgentState};
use oku_slides_session::agent::AgentUi;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn two_tabs_share_one_process_and_get_two_sessions() {
    let h = start_tabs(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    h.tab("t-2").agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-1")).await;
    wait_turn_ended(h.tab("t-2")).await;
    let log = read_log(&h.log);
    assert_that!(log.iter().filter(|m| m["method"] == "initialize").count()).is_equal_to(1);
    assert_that!(log.iter().filter(|m| m["method"] == "session/new").count()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn turns_of_two_tabs_run_at_the_same_time() {
    let h = start_tabs(json!({ "turns": [{ "delay_ms": 800, "chunks": ["fini"] }] }), &["t-1", "t-2"]).await;
    let started = std::time::Instant::now();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    h.tab("t-2").agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-1")).await;
    wait_turn_ended(h.tab("t-2")).await;
    assert_that!(started.elapsed()).is_less_than(Duration::from_millis(1500));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_tab_receives_only_its_own_chunks() {
    let h = start_tabs(json!({ "turns": [{ "chunks": ["réponse"] }] }), &["t-1", "t-2"]).await;
    // Les deux onglets ont chacun leur session (et leur route) avant le tour de t-1.
    h.tab("t-2").agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-2")).await;
    let mut other = h.tab("t-2").agent.subscribe();
    let mut own = h.tab("t-1").agent.subscribe();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    let chunk = until(&mut own, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { text: "réponse".into() });
    wait_turn_ended(h.tab("t-1")).await;
    let leaked = within(Duration::from_millis(200), &mut other, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(leaked).is_none();
    assert_that!(logged(&h.log, "session/new")).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_per_session_script_replaces_turns_for_that_session_only() {
    // t-1 prompte en premier (`sess-1`, pas de script propre : le tour global) ;
    // t-2 (`sess-2`) a le sien dans `sessions`.
    let script = json!({ "turns": [{ "chunks": ["global"] }], "sessions": { "2": [{ "chunks": ["propre"] }] } });
    let h = start_tabs(script, &["t-1", "t-2"]).await;
    let mut first = h.tab("t-1").agent.subscribe();
    let mut second = h.tab("t-2").agent.subscribe();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    let chunk = until(&mut first, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { text: "global".into() });
    wait_turn_ended(h.tab("t-1")).await;
    h.tab("t-2").agent.prompt("deux".into(), AgentUi::default());
    let chunk = until(&mut second, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { text: "propre".into() });
    wait_turn_ended(h.tab("t-2")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_calls_act_on_their_own_tab() {
    // Le tour appelle `apply` sur la slide `s-libre` : seule la session de t-2 change.
    let h = start_tabs(
        json!({ "turns": [{ "mcp": [{ "tool": "apply", "args": { "label": "x", "ops": [
        { "op": "set_param", "target": { "slide": "s-libre" }, "param": "title", "value": "Changé" } ] } }] }] }),
        &["t-1", "t-2"],
    )
    .await;
    h.tab("t-2").agent.prompt("change".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-2")).await;
    let title = |t: &str| {
        h.session(t)
            .lock()
            .map(|s| s.deck().slide("s-libre").and_then(|s| s.params.text("title").map(str::to_owned)))
            .unwrap()
    };
    assert_that!(title("t-2")).is_equal_to(Some("Changé".to_owned()));
    assert_that!(title("t-1")).is_not_equal_to(Some("Changé".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_crash_ends_the_turns_of_every_tab_with_an_error() {
    // t-1 attend l'annulation ; le premier prompt de la 2e session (t-2) fait quitter l'agent.
    let h = start_tabs(json!({ "crash_on_session": 2, "turns": [{ "wait_cancel": true }] }), &["t-1", "t-2"]).await;
    let mut first = h.tab("t-1").agent.subscribe();
    let mut second = h.tab("t-2").agent.subscribe();
    h.tab("t-1").agent.prompt("attends".into(), AgentUi::default());
    wait_status(&mut first, AgentState::Working).await;
    h.tab("t-2").agent.prompt("plante".into(), AgentUi::default());
    assert_that!(wait_error(&mut first).await).contains("arrêté");
    assert_that!(wait_error(&mut second).await).contains("arrêté");
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_of_one_tab_leaves_the_other_tab_turn_running() {
    let h = start_tabs(json!({ "turns": [{ "wait_cancel": true }] }), &["t-1", "t-2"]).await;
    let mut first = h.tab("t-1").agent.subscribe();
    let mut second = h.tab("t-2").agent.subscribe();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    h.tab("t-2").agent.prompt("deux".into(), AgentUi::default());
    wait_status(&mut first, AgentState::Working).await;
    wait_status(&mut second, AgentState::Working).await;
    h.tab("t-1").agent.cancel();
    let ended = wait_turn_ended(h.tab("t-1")).await;
    assert_that!(ended).is_equal_to(AgentEvent::TurnEnded { stop: "cancelled".into() });
    let still = within(Duration::from_millis(400), &mut second, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    assert_that!(still).is_none();
    h.tab("t-2").agent.cancel();
    let ended = wait_turn_ended(h.tab("t-2")).await;
    assert_that!(ended).is_equal_to(AgentEvent::TurnEnded { stop: "cancelled".into() });
    assert_that!(logged(&h.log, "session/cancel")).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_after_crash_relaunches_the_agent_with_a_new_session() {
    let h = start_tabs(json!({ "turns": [{ "chunks": ["un"] }, { "crash": true }] }), &["t-1"]).await;
    let mut rx = h.tab("t-1").agent.subscribe();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-1")).await;
    h.tab("t-1").agent.prompt("plante".into(), AgentUi::default());
    assert_that!(wait_error(&mut rx).await).contains("arrêté");
    h.tab("t-1").agent.prompt("encore".into(), AgentUi::default());
    let ended = wait_turn_ended(h.tab("t-1")).await;
    assert_that!(ended).is_equal_to(AgentEvent::TurnEnded { stop: "end_turn".into() });
    assert_that!(logged(&h.log, "initialize")).has_length(2);
    assert_that!(logged(&h.log, "session/new")).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_tab_does_not_unroute_a_reused_session_id_after_relaunch() {
    // Le 3e prompt d'une session fait quitter l'agent ; un agent relancé
    // renumérote ses sessions depuis `sess-1`.
    let script = json!({ "turns": [{ "chunks": ["un"] }, { "chunks": ["deux"] }, { "crash": true }] });
    let h = start_tabs(script, &["t-1", "t-2"]).await;
    // t-1 ouvre `sess-1` puis reste au repos ; t-2 (`sess-2`) fait planter l'agent.
    h.tab("t-1").agent.prompt("a".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-1")).await;
    let mut second = h.tab("t-2").agent.subscribe();
    for text in ["b", "c"] {
        h.tab("t-2").agent.prompt(text.into(), AgentUi::default());
        wait_turn_ended(h.tab("t-2")).await;
    }
    h.tab("t-2").agent.prompt("plante".into(), AgentUi::default());
    assert_that!(wait_error(&mut second).await).contains("arrêté");
    // Relance par t-2 : il reçoit `sess-1`, l'identifiant que t-1 garde de l'ancien lancement.
    h.tab("t-2").agent.prompt("d".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-2")).await;
    h.tab("t-1").agent.prompt("e".into(), AgentUi::default());
    wait_turn_ended(h.tab("t-1")).await;
    let mut later = h.tab("t-2").agent.subscribe();
    h.tab("t-2").agent.prompt("f".into(), AgentUi::default());
    let chunk = until(&mut later, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { text: "deux".into() });
    wait_turn_ended(h.tab("t-2")).await;
    let ids: Vec<_> = read_log(&h.log)
        .into_iter()
        .filter(|m| m["method"] == "session/prompt")
        .map(|m| m["params"]["sessionId"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_that!(ids.last().map(String::as_str)).is_equal_to(Some("sess-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn refused_prompt_reports_the_error_and_keeps_the_session() {
    let h = start_tabs(json!({ "turns": [{ "error": "pas maintenant" }, { "chunks": ["ok"] }] }), &["t-1"]).await;
    let mut rx = h.tab("t-1").agent.subscribe();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    let message = wait_error(&mut rx).await;
    assert_that!(message).contains("refusé");
    assert_that!(message).contains("pas maintenant");
    h.tab("t-1").agent.prompt("deux".into(), AgentUi::default());
    let ended = wait_turn_ended(h.tab("t-1")).await;
    assert_that!(ended).is_equal_to(AgentEvent::TurnEnded { stop: "end_turn".into() });
    assert_that!(logged(&h.log, "session/new")).has_length(1);
    assert_that!(logged(&h.log, "initialize")).has_length(1);
}
