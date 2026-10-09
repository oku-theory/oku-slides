mod common;

use std::sync::Arc;

use common::{
    Relayed, TestStack, frozen_turn, is_tool_surface, relayed_len, sessions_in, stack_resumed, stack_resumed_from,
    stack_with_sink, stack_with_sink_and_log, wait_for, wait_n_after, wait_surfaces, wait_turn_done,
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

fn atelier_json() -> serde_json::Value {
    json!({ "kind": "atelier", "pack": DEFAULT_PACK })
}

fn is(ch: &'static str, kind: &'static str, tab: &'static str) -> impl Fn(&(&'static str, serde_json::Value)) -> bool {
    move |(c, v)| *c == ch && v["event"]["kind"] == kind && v["tab"] == tab
}

/// Le premier relais vérifiant `pred` après `from` : sa position et sa valeur.
async fn next(
    rx: &Relayed,
    from: usize,
    pred: impl Fn(&(&'static str, serde_json::Value)) -> bool,
) -> (usize, serde_json::Value) {
    let (i, (_, v)) = wait_n_after(rx, from, 1, pred).await.remove(0);
    (i, v)
}

/// Un `resync` de `tab` dans `scope` (ou les slides) : rend la position juste après sa liste.
///
/// La relecture des fils au démarrage de l'onglet envoie aussi une liste, sans `viewing` avant :
/// on attend donc la portée annoncée, puis la liste qui la suit.
async fn shown(stack: &TestStack, rx: &Relayed, tab: &'static str, scope: Option<serde_json::Value>) -> usize {
    let from = relayed_len(rx);
    let mut req = json!({ "cmd": "resync", "tab": tab });
    if let Some(s) = scope {
        req["scope"] = s;
    }
    stack.handle(cmd(req)).unwrap_or_else(|e| panic!("resync : {e}"));
    let (viewing, _) = next(rx, from, is("mcp", "viewing", tab)).await;
    let (list, _) = next(rx, viewing, is("mcp", "surfaces", tab)).await;
    list + 1
}

/// Écrit le brouillon de `tab` depuis la vue `v-1`, et rend la position juste après son écho.
async fn drafted(stack: &TestStack, rx: &Relayed, tab: &'static str, text: Option<&str>, from: usize) -> usize {
    let req = json!({ "cmd": "set_draft", "tab": tab, "text": text, "origin": "v-1" });
    stack.handle(cmd(req)).unwrap_or_else(|e| panic!("set_draft : {e}"));
    let (echo, _) = next(rx, from, move |(c, v)| {
        *c == "agent"
            && v["event"]["kind"] == "draft"
            && v["tab"] == tab
            && v["event"]["origin"] == "v-1"
            && v["event"]["text"] == json!(text)
    })
    .await;
    echo + 1
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_is_broadcast_with_its_tab_and_origin() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "à dire", "origin": "v-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone()).is_equal_to(json!({ "kind": "draft", "text": "à dire", "origin": "v-1" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_null_clears_the_replayed_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    let from = drafted(&stack, &rx, "t-1", None, from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_longer_than_the_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1", None).await;
    let long = "a".repeat(oku_slides_agent::MAX_DRAFT_BYTES + 1);
    let refused = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": long, "origin": "v-1" })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_with_an_origin_over_the_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1", None).await;
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
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("gardé"), from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone()).is_equal_to(json!({ "kind": "draft", "text": "gardé", "origin": null }));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_sends_the_state_before_the_viewed_scope_and_its_list() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", Some(atelier_json())).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (status, _) = next(&rx, from, is("agent", "status", "t-1")).await;
    let (viewing, scope) = next(&rx, from, is("mcp", "viewing", "t-1")).await;
    let (list, _) = next(&rx, viewing, is("mcp", "surfaces", "t-1")).await;
    assert_that!(status).is_less_than(viewing);
    assert_that!(viewing).is_less_than(list);
    assert_that!(scope["event"]["scope"]["kind"].clone()).is_equal_to(json!("atelier"));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_sent_right_after_a_resync_keeps_the_resync_scope() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1", "scope": atelier_json() }))).unwrap();
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    // Celle du resync, puis celle du follow : toutes deux l'atelier, jamais les slides d'avant.
    let viewings = wait_n_after(&rx, from, 2, is("mcp", "viewing", "t-1")).await;
    let kinds: Vec<_> = viewings.iter().map(|(_, (_, v))| v["event"]["scope"]["kind"].clone()).collect();
    assert_that!(kinds).is_equal_to(vec![json!("atelier"), json!("atelier")]);
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_on_a_tab_without_agent_announces_the_slides() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = relayed_len(&rx);
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, viewing) = next(&rx, from, is("mcp", "viewing", "t-1")).await;
    assert_that!(viewing["event"]["scope"].clone()).is_equal_to(json!({ "kind": "slides" }));
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
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("encore là"), from).await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone()).is_equal_to(json!({ "kind": "draft", "text": "encore là", "origin": null }));
}

#[tokio::test(flavor = "multi_thread")]
async fn draft_of_one_tab_is_not_replayed_for_another() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1", "t-2"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = shown(&stack, &rx, "t-2", None).await.max(from);
    let from = drafted(&stack, &rx, "t-1", Some("à t-1"), from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-2" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-2")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}
