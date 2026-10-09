mod common;

use std::sync::Arc;

use common::{
    frozen_turn, is_tool_surface, sessions_in, stack_resumed, stack_resumed_from, stack_with_sink,
    stack_with_sink_and_log, wait_for, wait_surfaces, wait_turn_done,
};
use oku_slides_agent::AgentCommand;
use oku_slides_session::agent::AgentScope;
use oku_slides_session::pack::DEFAULT_PACK;
use serde_json::json;
use speculoos::prelude::*;

fn cmd(v: serde_json::Value) -> AgentCommand {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("commande : {e}"))
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
    // Le gardien a déjà envoyé une liste vide en relisant les fils (au démarrage de l'onglet).
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
async fn a_prompt_queued_before_tab_closed_never_reaches_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    // Fermé avant que ses outils n'aient démarré : la consigne est encore en file.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "perdue" }))).unwrap();
    stack.tab_closed("t-1");
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-2", "text": "gardée" }))).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_that!(common::logged(&log, "session/new")).has_length(1);
    let prompts = common::logged(&log, "session/prompt");
    assert_that!(prompts).has_length(1);
    assert_that!(prompts[0].to_string().contains("gardée")).is_true();
    let from_closed = rx.lock().unwrap().iter().filter(|(_, v)| v["tab"] == "t-1").count();
    assert_that!(from_closed).is_equal_to(0);
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

#[tokio::test(flavor = "multi_thread")]
async fn restart_shows_the_saved_thread_and_resumes_its_session() {
    let script = json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] });
    let (stack, rx, log, _resume) = stack_resumed(script, |s, agent| {
        s.keep_thread(AgentScope::Slides, vec![frozen_turn(3)]);
        s.keep_agent_session(AgentScope::Slides, "sess-42".into(), agent.into());
    })
    .await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1", "scope": { "kind": "slides" } }))).unwrap();
    let surfaces = wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["event"]["kind"] == "surfaces" && v["event"]["list"].as_array().is_some_and(|l| !l.is_empty())
    })
    .await;
    assert_that!(surfaces.1["event"]["list"][0]["id"].clone()).is_equal_to(json!("turn-3"));
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "encore" }))).unwrap();
    wait_turn_done(&rx, "t-1", 4).await;
    assert_that!(sessions_in(&log, "session/resume")).is_equal_to(vec!["sess-42".to_owned()]);
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
async fn command_without_scope_reads_as_slides() {
    match cmd(json!({ "cmd": "resync", "tab": "t-1" })) {
        AgentCommand::Resync { scope, .. } => assert_that!(scope).is_equal_to(AgentScope::Slides),
        other => panic!("resync attendu : {other:?}"),
    }
}

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

fn widget(name: &str) -> serde_json::Value {
    serde_json::to_value(AgentScope::Widget { pack: DEFAULT_PACK, name: name.into() })
        .unwrap_or_else(|e| panic!("portée : {e}"))
}

/// Les identifiants de la `nth` liste de surfaces relayée pour `t-1`.
async fn listed(rx: &common::Relayed, nth: usize) -> Vec<serde_json::Value> {
    let v = wait_surfaces(rx, "t-1", nth).await;
    v["event"]["list"]
        .as_array()
        .unwrap_or_else(|| panic!("liste attendue : {v}"))
        .iter()
        .map(|s| s["id"].clone())
        .collect()
}

/// Un tour dans `kpi` (vu), puis, depuis la pile, `kpi` renommé `ancien` et un nouveau `kpi`,
/// chaque changement signalé comme le fait l'hôte ; aucun ordre au gardien ensuite.
async fn kpi_renamed_then_recreated() -> (common::TestStack, common::Relayed, std::path::PathBuf) {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.on_session("t-1", |s| {
        s.edit_templates(DEFAULT_PACK).unwrap_or_else(|e| panic!("édition du pack : {e}"));
        s.set_widget("kpi", KPI.into()).unwrap_or_else(|e| panic!("widget : {e}"));
    });
    // 1re liste : la relecture des fils ; 2e : ce `resync`.
    stack
        .handle(cmd(json!({ "cmd": "resync", "tab": "t-1", "scope": widget("kpi") })))
        .unwrap_or_else(|e| panic!("commande : {e}"));
    listed(&rx, 2).await;
    stack
        .handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "scope": widget("kpi"), "text": "un" })))
        .unwrap_or_else(|e| panic!("commande : {e}"));
    wait_turn_done(&rx, "t-1", 1).await;
    stack.on_session("t-1", |s| s.rename_widget("kpi", "ancien").unwrap_or_else(|e| panic!("widget : {e}")));
    stack.after_change("t-1");
    stack.on_session("t-1", |s| s.set_widget("kpi", KPI.into()).unwrap_or_else(|e| panic!("widget : {e}")));
    stack.after_change("t-1");
    (stack, rx, log)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_widget_of_a_renamed_name_prompted_first_opens_its_own_session() {
    let (stack, rx, log) = kpi_renamed_then_recreated().await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "scope": widget("kpi"), "text": "deux" }))).unwrap();
    // La vue a suivi `ancien` : le tour de `kpi` n'est pas montré ; le journal du faux agent fait foi.
    drop(rx);
    for _ in 0..500 {
        if sessions_in(&log, "session/prompt").len() >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_that!(sessions_in(&log, "session/prompt")).is_equal_to(vec!["sess-1".to_owned(), "sess-2".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn renamed_from_the_ui_then_a_new_widget_of_the_old_name_gets_its_own_thread_and_session() {
    let (stack, rx, log) = kpi_renamed_then_recreated().await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1", "scope": widget("kpi") }))).unwrap();
    assert_that!(listed(&rx, 3).await).is_empty();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "scope": widget("kpi"), "text": "deux" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    assert_that!(sessions_in(&log, "session/prompt")).is_equal_to(vec!["sess-1".to_owned(), "sess-2".to_owned()]);
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1", "scope": widget("ancien") }))).unwrap();
    let old = listed(&rx, 4).await;
    assert_that!(old).contains(json!("turn-1"));
    assert_that!(old).does_not_contain(json!("turn-2"));
}
