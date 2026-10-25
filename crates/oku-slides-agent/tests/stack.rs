//! The tab agent stack: interface commands, event relay, closing.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use std::sync::Arc;

use common::{
    Relayed, TestStack, cmd, drafted, frozen_turn, is, is_tool_surface, logged, next, relayed_len, sessions_in, shown,
    stack_locked, stack_resumed, stack_resumed_from, stack_with_sink, stack_with_sink_and_log, stack_with_two_agents,
    wait_for, wait_n_after, wait_surfaces, wait_turn_done,
};
use oku_slides_agent::AgentCommand;
use serde_json::json;
use speculoos::prelude::*;

/// Ouvre la boîte des réglages de `tab`, puis y clique l'agent `id` (écrit `config.toml` : hors des fils du runtime).
fn select(stack: &TestStack, tab: &str, id: &str) -> Result<(), String> {
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": tab }))))?;
    click_agent(stack, tab, id)
}

/// Le dernier choix d'agent montré par la boîte de `tab`, après les `from` premiers relais.
async fn shown_choice_after(rx: &Relayed, tab: &str, from: usize) -> Option<String> {
    let got = wait_n_after(rx, from, 1, |(ch, v)| *ch == "mcp" && v["tab"] == tab && choice_of(v).is_some()).await;
    got.last().and_then(|(_, (_, v))| choice_of(v))
}

#[tokio::test(flavor = "multi_thread")]
async fn forward_wraps_every_event_with_its_tab() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1", "t-2"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "prompt", "tab": "t-2", "text": "salut" })).unwrap()).unwrap();
    let surface = wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "turn-1"
    })
    .await;
    assert_that!(surface.1["tab"].clone()).is_equal_to(json!("t-2"));
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_replays_the_agent_state_then_the_turn_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" })).unwrap()).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    // L'état `idle` de fin de tour doit être relayé avant de prendre la mesure.
    wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "status" && v["event"]["state"] == "idle").await;
    let before = rx.lock().unwrap().len();
    stack.handle(serde_json::from_value(json!({ "cmd": "resync", "tab": "t-1" })).unwrap()).unwrap();
    // Le gardien a déjà envoyé une liste vide en relisant le fil (au démarrage de l'onglet).
    let surfaces = wait_for(&rx, |(ch, v)| {
        *ch == "mcp"
            && v["event"]["kind"] == "surfaces"
            && v["tab"] == "t-1"
            && v["event"]["list"].as_array().is_some_and(|l| !l.is_empty())
    })
    .await;
    let ids: Vec<_> = surfaces.1["event"]["list"].as_array().unwrap().iter().map(|s| s["id"].clone()).collect();
    assert_that!(ids).contains(json!("turn-1"));
    // L'état de l'agent d'abord, puis les surfaces.
    let relayed = rx.lock().unwrap()[before..].to_vec();
    let at = |ch: &str, kind: &str| relayed.iter().position(|(c, v)| *c == ch && v["event"]["kind"] == kind).unwrap();
    assert_that!(at("agent", "status")).is_less_than(at("mcp", "surfaces"));
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_of_a_tab_without_conversation_sends_idle_and_no_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "resync", "tab": "t-1" })).unwrap()).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "status" && v["event"]["state"] == "idle").await;
    // La 1re liste vient de la relecture des fils au démarrage de l'onglet, la 2e du `resync`.
    let surfaces = wait_surfaces(&rx, "t-1", 2).await;
    assert_that!(surfaces["event"]["list"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn tab_closed_during_a_turn_releases_its_questions_and_other_tabs_go_on() {
    // Premier tour de chaque session : une question ; t-1 la laisse ouverte, t-2 y répond.
    let script = json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?",
        "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }], "chunks": ["fini"] }] });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1", "t-2"]).await;
    let prompt = |tab: &str| serde_json::from_value(json!({ "cmd": "prompt", "tab": tab, "text": "?" })).unwrap();
    stack.handle(prompt("t-1")).unwrap();
    stack.handle(prompt("t-2")).unwrap();
    let ask2 = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-2", "ask-")).await;
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    stack.tab_closed("t-1");
    let id = ask2.1["event"]["surface"]["id"].as_str().unwrap().to_owned();
    stack
        .handle(
            serde_json::from_value(json!({ "cmd": "surface_action", "tab": "t-2", "surface": id,
        "name": "choose", "context": { "choice": "b" } }))
            .unwrap(),
        )
        .unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    let seen = rx.lock().unwrap().len();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let late: Vec<_> = rx.lock().unwrap()[seen..].iter().filter(|(_, v)| v["tab"] == "t-1").cloned().collect();
    assert_that!(late.iter().any(|(_, v)| v["event"]["kind"] == "surface_patch")).is_false();
    // Les deux questions ont rendu leur outil à l'agent : celle de t-1 relâchée, pas pendante.
    let outs: Vec<String> = common::logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!(outs).has_length(2);
    assert_that!(outs.iter().filter(|o| o.contains("cancelled")).count()).is_equal_to(1);
    assert_that!(outs.iter().filter(|o| o.contains("choice")).count()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_answering_a_waiting_question_answers_it_instead_of_new_turn() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [ask] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    let asked = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    let id = asked.1["event"]["surface"]["id"].clone();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "la deuxième", "answer": id }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    // La consigne est devenue la réponse de la question (rendue à l'agent), pas un nouveau tour.
    let outs: Vec<String> = common::logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!(outs).has_length(1);
    assert_that!(outs[0]).contains("la deuxième");
    assert_that!(common::logged(&log, "session/prompt")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_answering_a_question_no_longer_waiting_runs_as_an_instruction() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let script = json!({ "turns": [{ "mcp": [ask] }, { "chunks": ["ok"] }] });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    // La question désignée a été répondue (ou annulée) entre-temps : la consigne n'est pas perdue.
    let late = cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "la deuxième", "answer": "ask-999" }));
    assert_that!(stack.handle(late)).is_ok();
    wait_turn_done(&rx, "t-1", 2).await;
    let prompts = common::logged(&log, "session/prompt");
    assert_that!(prompts).has_length(2);
    assert_that!(prompts[1].to_string().contains("la deuxième")).is_true();
    // Elle n'a répondu à aucune autre question : celle qui attendait est relâchée, sans ce texte.
    let outs: Vec<String> = common::logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!(outs.iter().any(|o| o.contains("la deuxième"))).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_passes_the_ui_context_with_the_prompt() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    stack
        .handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "bonjour", "ui": { "slide": "s-libre" } })))
        .unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = &common::logged(&log, "session/prompt")[0]["prompt"];
    assert_that!(prompt[2]["text"].as_str().unwrap().contains("slide: s-libre")).is_true();
}

/// La clé `locale` que l'UI envoie (`AgentUiSent`, ui/src/api.ts) arrive au contexte de l'agent.
#[tokio::test(flavor = "multi_thread")]
async fn handle_passes_the_language_the_ui_shows_to_the_context() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "bonjour", "ui": { "locale": "de" } }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = common::logged(&log, "session/prompt")[0]["prompt"].to_string();
    assert_that!(prompt.as_str()).contains("interface language: German (de)");
}

#[tokio::test(flavor = "multi_thread")]
async fn surface_action_on_an_unknown_surface_or_tab_is_an_error() {
    let (stack, _) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    let action = |tab: &str| cmd(json!({ "cmd": "surface_action", "tab": tab, "surface": "x", "name": "dismiss" }));
    // L'interface affiche le texte reçu : dans la langue de l'utilisateur (`fr`), sans identifiant interne.
    let refused = "Cette carte n'est plus ouverte.".to_owned();
    assert_that!(stack.handle(action("t-9"))).is_err_containing(refused.clone());
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" }))).unwrap();
    assert_that!(stack.handle(action("t-1"))).is_err_containing(refused);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_closes_the_turn_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "close_surface" && v["event"]["id"] == "turn-1")
        .await;
    let before = common::surfaces_of(&rx, "t-1").len();
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    let last = wait_surfaces(&rx, "t-1", before + 1).await;
    assert_that!(last["event"]["list"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_on_a_tab_without_agent_starts_no_agent_process() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" }))).unwrap();
    // Aucun onglet démarré : l'état rejoué est `idle`, sans surface.
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "status" && v["event"]["state"] == "idle").await;
    // La 1re liste vient de la relecture des fils au démarrage de l'onglet, la 2e du `resync`.
    let surfaces = wait_surfaces(&rx, "t-1", 2).await;
    assert_that!(surfaces["event"]["list"].clone()).is_equal_to(json!([]));
    assert_that!(common::read_log(&log)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_stack_ends_the_forwarding() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-2" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    drop(stack);
    // Le relais tient le puits (et son clone de `rx`) : il doit finir avec la pile.
    for _ in 0..250 {
        if Arc::strong_count(&rx) == 1 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("le relais survit à la pile");
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_for_a_closed_tab_are_ignored_and_start_nothing() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    stack.tab_closed("t-1");
    // Arrivées en retard, après la fermeture : acceptées sans effet.
    assert_that!(stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "tardive" })))).is_ok();
    assert_that!(stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" })))).is_ok();
    assert_that!(stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" })))).is_ok();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-2", "text": "gardée" }))).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_that!(common::logged(&log, "session/new")).has_length(1);
    let from_closed = rx.lock().unwrap().iter().filter(|(_, v)| v["tab"] == "t-1").count();
    assert_that!(from_closed).is_equal_to(0);
}

/// Le processus `pid` existe-t-il encore ? (`kill -0`, sans signal envoyé.)
#[cfg(unix)]
fn alive(pid: u64) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn shutdown_kills_the_agent_and_later_commands_start_nothing() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let pid = common::logged(&log, "pid")[0].as_u64().unwrap();
    stack.shutdown().await;
    // Tué (puis ramassé en arrière-plan par tokio) : parti en moins de 2 s.
    let mut gone = false;
    for _ in 0..100 {
        if !alive(pid) {
            gone = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_that!(gone).is_true();
    assert_that!(stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "tardive" })))).is_ok();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_that!(common::logged(&log, "initialize")).has_length(1);
}

/// A resumed stack whose tab `t-1` kept turn 3 and session `sess-42`.
///
/// Turn 3 is kept in the tab's thread.
async fn restarted() -> (TestStack, common::Relayed, std::path::PathBuf) {
    let script = json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] });
    let (stack, rx, log, _resume) = stack_resumed(script, |s, agent| {
        s.keep_thread(vec![frozen_turn(3)]);
        s.keep_agent_session("sess-42".into(), agent.into());
    })
    .await;
    (stack, rx, log)
}

/// Les identifiants de la dernière liste de surfaces de `t-1`, après un `resync`.
async fn resynced(stack: &TestStack, rx: &common::Relayed) -> Vec<serde_json::Value> {
    let at = shown(stack, rx, "t-1").await;
    let list = rx.lock().unwrap_or_else(|e| panic!("relais : {e}"))[at - 1].1["event"]["list"].clone();
    list.as_array().unwrap_or_else(|| panic!("liste attendue : {list}")).iter().map(|s| s["id"].clone()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_after_restart_shows_the_saved_thread() {
    let (stack, rx, _log) = restarted().await;
    assert_that!(resynced(&stack, &rx).await).is_equal_to(vec![json!("turn-3")]);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_after_restart_resumes_the_saved_session() {
    let (stack, rx, log) = restarted().await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "encore" }))).unwrap();
    wait_turn_done(&rx, "t-1", 4).await;
    assert_that!(sessions_in(&log, "session/resume")).is_equal_to(vec!["sess-42".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_forgets_thread_and_session() {
    let (stack, rx, log) = restarted().await;
    resynced(&stack, &rx).await;
    stack.handle(cmd(json!({ "cmd": "new_conversation", "tab": "t-1" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "close_surface" && v["event"]["id"] == "turn-3")
        .await;
    assert_that!(resynced(&stack, &rx).await).is_empty();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "encore" }))).unwrap();
    // Les numéros de tour continuent après ceux du fil relu.
    wait_turn_done(&rx, "t-1", 4).await;
    assert_that!(common::logged(&log, "session/resume")).is_empty();
    assert_that!(common::logged(&log, "session/new")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_turn_ended_before_shutdown_is_read_back_after_a_restart() {
    let script = json!({ "turns": [{ "chunks": ["ok"] }] });
    let (stack, rx, _log, resume) = stack_resumed(script.clone(), |_, _| {}).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    stack.shutdown().await; // vide la file du gardien
    drop(stack);
    // Second lancement sur le même dossier de reprise : le fil du tour 1 y est.
    let (stack, rx, _log, _) = stack_resumed_from(script, resume).await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    let surfaces = wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["event"]["kind"] == "surfaces" && v["event"]["list"].as_array().is_some_and(|l| !l.is_empty())
    })
    .await;
    assert_that!(surfaces.1["event"]["list"][0]["id"].clone()).is_equal_to(json!("turn-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_is_broadcast_with_its_tab_and_origin() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "à dire", "origin": "v-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone())
        .is_equal_to(json!({ "kind": "draft", "text": "à dire", "origin": "v-1", "images": [] }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_null_clears_the_replayed_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    let from = drafted(&stack, &rx, "t-1", None, from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_longer_than_the_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1").await;
    let long = "a".repeat(oku_slides_agent::MAX_DRAFT_BYTES + 1);
    let refused = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": long, "origin": "v-1" })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_with_an_origin_over_the_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1").await;
    let origin = "v".repeat(oku_slides_agent::MAX_ORIGIN_BYTES + 1);
    let refused = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "x", "origin": origin })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_on_a_tab_without_agent_is_refused() {
    let (stack, _rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let refused = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "perdu", "origin": "v-1" })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_on_a_tab_without_agent_starts_nothing() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let _ = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "perdu", "origin": "v-1" })));
    let from = relayed_len(&rx);
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_replays_the_draft_without_origin() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("gardé"), from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone())
        .is_equal_to(json!({ "kind": "draft", "text": "gardé", "origin": null, "images": [] }));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_sends_the_state_before_the_list() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (status, _) = next(&rx, from, is("agent", "status", "t-1")).await;
    let (list, _) = next(&rx, from, is("mcp", "surfaces", "t-1")).await;
    assert_that!(status).is_less_than(list);
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_on_a_tab_without_agent_sends_an_empty_list() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = relayed_len(&rx);
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, list) = next(&rx, from, is("mcp", "surfaces", "t-1")).await;
    assert_that!(list["event"]["list"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_replays_the_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("encore là"), from).await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone())
        .is_equal_to(json!({ "kind": "draft", "text": "encore là", "origin": null, "images": [] }));
}

#[tokio::test(flavor = "multi_thread")]
async fn draft_of_one_tab_is_not_replayed_for_another() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1", "t-2"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = shown(&stack, &rx, "t-2").await.max(from);
    let from = drafted(&stack, &rx, "t-1", Some("à t-1"), from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-2" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-2")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_saves_the_choice_and_the_next_prompt_reaches_it() {
    let (stack, rx, log, data) = stack_with_two_agents(
        json!({ "turns": [{ "chunks": ["un"] }] }),
        json!({ "turns": [{ "chunks": ["bis"] }] }),
        &["t-1"],
    )
    .await;
    select(&stack, "t-1", "bis").unwrap();
    let toml = std::fs::read_to_string(data.join("config.toml")).unwrap();
    assert_that!(toml.contains("selected = \"bis\"")).is_true();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["tab"] == "t-1" && choice_of(v).as_deref() == Some("bis")).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(sessions_in(&data.join("log-bis.jsonl"), "session/prompt").len()).is_equal_to(1);
    assert_that!(sessions_in(&log, "session/prompt")).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_during_a_turn_stops_it_with_agent_stopped() {
    let ask = json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?",
        "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }] }] });
    let (stack, rx, _, _) = stack_with_two_agents(ask, json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "?" }))).unwrap();
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" })))).unwrap();
    wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["tab"] == "t-1" && choice_of(v).is_some() && v.to_string().contains("interrompra")
    })
    .await;
    click_agent(&stack, "t-1", "bis").unwrap();
    // Le tour échoue : sa surface (canal `mcp`) le dit dans `outcome`.
    wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["tab"] == "t-1" && v["event"].to_string().contains("L'agent s'est arrêté")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_unknown_or_locked_or_unsaved_is_refused_and_nothing_switches() {
    let (stack, rx, _, data) = stack_with_two_agents(json!({ "turns": [] }), json!({ "turns": [] }), &["t-1"]).await;
    let unknown = select(&stack, "t-1", "mistral");
    assert_that!(unknown).is_err_containing("Cet agent n'est plus dans la configuration.".to_owned());
    // `config.toml` impossible à écrire : un dossier à sa place.
    std::fs::create_dir_all(data.join("config.toml")).unwrap();
    let unsaved = select(&stack, "t-1", "bis");
    assert_that!(unsaved).is_err_containing("Le choix de l'agent n'a pas pu être enregistré.".to_owned());
    // Rien n'a basculé : la boîte, rouverte, montre toujours `faux`.
    let from = relayed_len(&rx);
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" })))).unwrap();
    assert_that!(shown_choice_after(&rx, "t-1", from).await).is_equal_to(Some("faux".to_owned()));
    let (locked, _) = stack_locked(json!({ "turns": [] }), json!({ "turns": [] })).await;
    assert_that!(select(&locked, "t-1", "bis"))
        .is_err_containing("Le choix de l'agent est imposé par l'environnement.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_does_not_resume_the_other_agent_session() {
    let lost: String = oku_slides_i18n::Localizer::new(Some("fr")).tr("agent-memory-lost").chars().take(30).collect();
    let (stack, rx, _, data) = stack_with_two_agents(
        json!({ "turns": [{ "chunks": ["un"] }] }),
        json!({ "resume": true, "load_session": true, "turns": [{ "chunks": ["bis"] }] }),
        &["t-1"],
    )
    .await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    select(&stack, "t-1", "bis").unwrap();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "deux" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    let bis = data.join("log-bis.jsonl");
    assert_that!(sessions_in(&bis, "session/load")).is_empty();
    assert_that!(sessions_in(&bis, "session/resume")).is_empty();
    assert_that!(sessions_in(&bis, "session/new")).has_length(1);
    let relayed = rx.lock().unwrap().clone();
    assert_that!(relayed.iter().any(|(_, v)| v.to_string().contains(&lost))).is_false();
}

/// Le texte du tour de fine-tuning dans `locale`.
fn tune_text(locale: &str) -> String {
    oku_slides_i18n::Localizer::new(Some(locale)).tr("agent-tune-prompt")
}

#[test]
fn tune_command_from_the_ui_is_refused() {
    // Seul l'hôte lance le tour de fine-tuning (`Stack::tune`), jamais l'interface.
    let parsed = serde_json::from_value::<AgentCommand>(json!({ "cmd": "tune", "tab": "t-1" }));
    assert_that!(parsed.is_err()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn tune_sends_the_import_instructions_before_the_context_and_its_text_last() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.tune("t-1"));
    wait_turn_done(&rx, "t-1", 1).await;
    let blocks = common::logged(&log, "session/prompt")[0]["prompt"].as_array().cloned().unwrap();
    assert_that!(blocks.len()).is_equal_to(5);
    let import = blocks[2]["text"].as_str().unwrap();
    assert_that!(import).starts_with("Fine-tuning an imported deck");
    assert_that!(blocks[3]["text"].as_str().unwrap()).starts_with("<oku-slides-context>");
    // La langue de l'interface (`fr` dans la pile de test), lue au lancement du tour.
    assert_that!(blocks[4]["text"].as_str().map(str::to_owned)).is_equal_to(Some(tune_text("fr")));
}

#[tokio::test(flavor = "multi_thread")]
async fn tune_during_a_turn_replaces_it_with_the_import_instructions() {
    let script = json!({ "turns": [{ "wait_cancel": true }, {}] });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "turn-1")
        .await;
    tokio::task::block_in_place(|| stack.tune("t-1"));
    wait_turn_done(&rx, "t-1", 2).await;
    let blocks = common::logged(&log, "session/prompt")[1]["prompt"].as_array().cloned().unwrap();
    // Même session, pas un premier tour : les consignes d'import, le contexte, le texte.
    assert_that!(blocks.len()).is_equal_to(3);
    assert_that!(blocks[0]["text"].as_str().unwrap()).starts_with("Fine-tuning an imported deck");
    assert_that!(blocks[2]["text"].as_str().map(str::to_owned)).is_equal_to(Some(tune_text("fr")));
}

#[tokio::test(flavor = "multi_thread")]
async fn tune_shows_its_short_text_in_the_thread_never_the_instructions() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.tune("t-1"));
    wait_turn_done(&rx, "t-1", 1).await;
    // La pile de test est en `fr` : le fil montre le texte court, jamais les consignes.
    let thread: String = rx
        .lock()
        .unwrap()
        .iter()
        .filter(|(ch, v)| *ch == "mcp" && v["tab"] == "t-1")
        .map(|(_, v)| v.to_string())
        .collect();
    assert_that!(thread.contains(&tune_text("fr"))).is_true();
    assert_that!(thread.contains("import_report")).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_after_a_tune_sends_no_import_instructions() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.tune("t-1"));
    wait_turn_done(&rx, "t-1", 1).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "merci" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    let second = common::logged(&log, "session/prompt")[1]["prompt"].to_string();
    assert_that!(second.contains("import_report")).is_false();
}

/// Relayé : une surface `host-theme-picker`.
fn is_picker(v: &(&'static str, serde_json::Value)) -> bool {
    v.0 == "mcp" && v.1["event"]["kind"] == "surface" && v.1["event"]["surface"]["id"] == "host-theme-picker"
}

/// Le `slide` de la première carte d'une surface relayée.
fn first_card_slide(v: &serde_json::Value) -> Option<String> {
    v["event"]["surface"]["messages"].as_array()?.iter().find_map(|m| {
        m["updateComponents"]["components"]
            .as_array()?
            .iter()
            .find_map(|c| (c["component"] == "ThemeChoice").then(|| c["slide"].as_str().map(str::to_owned)).flatten())
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_shows_the_picker_on_the_slide_shown() {
    let (stack, rx, _log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    let prompt = json!({ "cmd": "prompt", "tab": "t-1", "text": "Thème.", "ui": { "slide": "s-libre" } });
    tokio::task::block_in_place(|| stack.handle(cmd(prompt))).unwrap();
    let shown = wait_for(&rx, is_picker).await;
    assert_that!(first_card_slide(&shown.1)).is_equal_to(Some("s-libre".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_sends_nothing_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "Thème." }))))
        .unwrap();
    wait_for(&rx, is_picker).await;
    // Témoin : les commandes d'un onglet passent dans l'ordre, une consigne envoyée à tort serait la première.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompts = common::logged(&log, "session/prompt");
    assert_that!((
        prompts.len(),
        prompts[0].to_string().contains("salut"),
        common::logged(&log, "session/cancel").len()
    ))
    .is_equal_to((1, true, 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_answering_a_waiting_question_answers_it() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [ask] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    let asked = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    let id = asked.1["event"]["surface"]["id"].clone();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "thème", "answer": id }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let outs: Vec<String> = common::logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!((outs.len(), outs.first().is_some_and(|o| o.contains("thème")))).is_equal_to((1, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_during_a_turn_leaves_the_turn_running() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [ask] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    let asked = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    let id = asked.1["event"]["surface"]["id"].as_str().unwrap().to_owned();
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "thème" }))))
        .unwrap();
    wait_for(&rx, is_picker).await;
    // La question attend toujours : y répondre finit le tour, sans annulation.
    let answer =
        json!({ "cmd": "surface_action", "tab": "t-1", "surface": id, "name": "choose", "context": { "choice": "a" } });
    stack.handle(cmd(answer)).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let outs = common::logged(&log, "mcp/result");
    assert_that!((
        outs.len(),
        common::logged(&log, "session/cancel").len(),
        common::logged(&log, "session/prompt").len()
    ))
    .is_equal_to((1, 0, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_outside_deck_mode_goes_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.on_session("t-1", |s| s.edit_pack(oku_slides_session::pack::ESSENTIEL_PACK).unwrap());
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "thème" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(common::logged(&log, "session/prompt")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_containing_theme_in_a_sentence_goes_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "change le thème" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(common::logged(&log, "session/prompt")).has_length(1);
}

#[test]
fn show_packs_command_deserializes_with_its_tab() {
    assert_that!(cmd(json!({ "cmd": "show_packs", "tab": "t" })).tab().to_owned()).is_equal_to("t".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_relays_the_packs_panel_to_its_tab() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_packs", "tab": "t-1" }))))
        .unwrap_or_else(|e| panic!("show_packs : {e}"));
    let (_, ev) = wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "host-packs"
    })
    .await;
    assert_that!(ev["tab"].clone()).is_equal_to(json!("t-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_outside_deck_mode_shows_nothing() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    stack.on_session("t-1", |s| s.edit_pack(oku_slides_session::pack::ESSENTIEL_PACK).unwrap());
    let shown = tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_packs", "tab": "t-1" }))));
    // Witness: the tab's commands run in order, a packs panel would be relayed before the settings box.
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" })))).unwrap();
    wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "host-voice"
    })
    .await;
    let packs =
        rx.lock().unwrap_or_else(|e| panic!("relais : {e}")).iter().any(|(ch, v)| {
            *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "host-packs"
        });
    assert_that!(shown).is_ok();
    assert_that!(packs).is_false();
}

#[test]
fn show_settings_command_deserializes_with_its_tab() {
    assert_that!(cmd(json!({ "cmd": "show_settings", "tab": "t" })).tab().to_owned()).is_equal_to("t".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn show_settings_relays_the_box_to_its_tab() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" })))).unwrap();
    let (_, ev) = wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "host-voice"
    })
    .await;
    assert_that!(ev["tab"].clone()).is_equal_to(json!("t-1"));
}

/// Le choix d'agent d'une boîte relayée, entière ou en données seules.
fn choice_of(v: &serde_json::Value) -> Option<String> {
    let messages = v["event"]["surface"]["messages"].as_array().or_else(|| v["event"]["messages"].as_array())?;
    messages.iter().find_map(|m| m["updateDataModel"]["value"]["choice"].as_str().map(str::to_owned))
}

/// Le clic sur la pastille `id` dans la boîte de `tab` (écrit `config.toml` : hors des fils du runtime).
fn click_agent(stack: &TestStack, tab: &str, id: &str) -> Result<(), String> {
    let click = json!({ "cmd": "surface_action", "tab": tab, "surface": "host-voice", "name": "select", "context": { "id": id } });
    tokio::task::block_in_place(|| stack.handle(cmd(click)))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_in_one_tab_refreshes_the_box_of_the_other() {
    let (stack, rx, _, _) =
        stack_with_two_agents(json!({ "turns": [] }), json!({ "turns": [] }), &["t-1", "t-2"]).await;
    for tab in ["t-1", "t-2"] {
        tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": tab })))).unwrap();
    }
    click_agent(&stack, "t-1", "bis").unwrap();
    // t-2 ne voit ce choix que par la révision : le clic n'a refait que la boîte de t-1.
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["tab"] == "t-2" && choice_of(v).as_deref() == Some("bis")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_turn_in_one_tab_shows_busy_in_the_box_of_the_other() {
    let ask = json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }] }] });
    let (stack, rx, _, _) = stack_with_two_agents(ask, json!({ "turns": [] }), &["t-1", "t-2"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-2" })))).unwrap();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "?" }))).unwrap();
    wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["tab"] == "t-2" && v.to_string().contains("Changer d'agent interrompra le tour en cours.")
    })
    .await;
}

/// Sonde `ready` toutes les 20 ms, 10 s au plus.
async fn wait_until(ready: impl Fn() -> bool) {
    for _ in 0..500 {
        if ready() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("condition non atteinte en 10 s");
}

/// Un tour où l'agent demande `bis`, puis pose une question qui le garde ouvert.
fn switching_script() -> serde_json::Value {
    json!({ "turns": [{ "mcp": [
        { "tool": "select_agent", "args": { "id": "bis" } },
        { "tool": "ask_choice", "args": { "question": "?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }
    ] }, { "chunks": ["deux"] }] })
}

/// Confirme la fenêtre de l'onglet `tab`.
async fn confirm_modal(stack: &TestStack, rx: &Relayed, tab: &str) {
    let (_, modal) =
        wait_for(rx, |(ch, v)| *ch == "mcp" && v["tab"] == tab && v["event"]["surface"]["place"] == "modal").await;
    let id = modal["event"]["surface"]["id"].as_str().unwrap_or_else(|| panic!("fenêtre sans id")).to_owned();
    let yes = json!({ "cmd": "surface_action", "tab": tab, "surface": id, "name": "confirm", "context": { "confirmed": true } });
    tokio::task::block_in_place(|| stack.handle(cmd(yes))).unwrap_or_else(|e| panic!("confirmation : {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_switch_asked_during_a_turn_answers_when_idle() {
    let (stack, rx, log, _) = stack_with_two_agents(switching_script(), json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "passe à bis" }))).unwrap();
    confirm_modal(&stack, &rx, "t-1").await;
    wait_until(|| {
        logged(&log, "mcp/result")
            .iter()
            .any(|c| c["tool"] == "select_agent" && c["out"].to_string().contains("when_idle"))
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_switch_waits_across_a_queued_prompt_then_reaches_bis() {
    let (stack, rx, log, data) =
        stack_with_two_agents(switching_script(), json!({ "turns": [{ "chunks": ["bis"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "passe à bis" }))).unwrap();
    confirm_modal(&stack, &rx, "t-1").await;
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    // Une consigne en file pendant la question : elle part au même agent, le changement attend.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "encore" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    assert_that!(sessions_in(&log, "session/prompt")).has_length(2);
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "et maintenant" }))).unwrap();
    wait_turn_done(&rx, "t-1", 3).await;
    assert_that!(sessions_in(&data.join("log-bis.jsonl"), "session/prompt")).has_length(1);
}
