//! Une session ACP par portée ; reprise d'une session connue ; gardien.

mod common;

use common::*;
use oku_slides_agent::{AgentEvent, AgentState, Keep};
use oku_slides_session::agent::{AgentScope, AgentUi};
use oku_slides_session::pack::BASE_PACK;
use serde_json::json;
use speculoos::prelude::*;

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

fn atelier() -> AgentScope {
    AgentScope::Atelier { pack: BASE_PACK }
}

fn widget(name: &str) -> AgentScope {
    AgentScope::Widget { pack: BASE_PACK, name: name.into() }
}

/// `f` sur la session de `t-1`, hors du fil async (le stockage de la session bloque).
fn on_session<R>(h: &Harness, f: impl FnOnce(&mut oku_slides_session::Session) -> R) -> R {
    tokio::task::block_in_place(|| f(&mut h.session("t-1").lock().unwrap_or_else(|e| panic!("session : {e}"))))
}

/// `t-1` en édition du pack par défaut, avec le widget `kpi`.
fn editing_with_kpi(h: &Harness) {
    on_session(h, |s| {
        s.edit_pack(BASE_PACK).unwrap_or_else(|e| panic!("édition du pack : {e}"));
        s.set_widget(None, "kpi", KPI.into()).unwrap_or_else(|e| panic!("widget : {e}"));
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn each_scope_gets_its_own_session_and_keeps_it() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    for (scope, text) in [(AgentScope::Slides, "un"), (atelier(), "deux"), (AgentScope::Slides, "trois")] {
        t.agent.prompt_in(scope, text.into(), AgentUi::default());
        wait_turn_ended(t).await;
    }
    assert_that!(logged(&h.log, "session/new")).has_length(2);
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec![
        "sess-1".into(),
        "sess-2".into(),
        "sess-1".into(),
    ]);
}

#[tokio::test(flavor = "multi_thread")]
async fn status_carries_the_scope_of_the_turn() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt_in(atelier(), "un".into(), AgentUi::default());
    let working = until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Working, .. })).await;
    assert_that!(working).is_equal_to(AgentEvent::Status { state: AgentState::Working, scope: Some(atelier()) });
    let idle = until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Idle, .. })).await;
    assert_that!(idle).is_equal_to(AgentEvent::idle());
}

#[tokio::test(flavor = "multi_thread")]
async fn known_session_is_resumed_at_the_first_prompt_only() {
    let h = start(json!({ "resume": true, "load_session": true, "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    // Rien n'est repris avant un prompt (contrôle faible : l'absence au bout de 100 ms).
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/resume")).is_equal_to(vec!["sess-42".to_owned()]);
    assert_that!(logged(&h.log, "session/load")).is_empty();
    assert_that!(logged(&h.log, "session/new")).is_empty();
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-42".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn loaded_session_replay_stays_out_of_events_and_surfaces() {
    let h = start(json!({ "load_session": true, "replay": ["ancien texte"], "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/load")).is_equal_to(vec!["sess-42".to_owned()]);
    let mut chunks = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let AgentEvent::Chunk { text, .. } = e {
            chunks.push(text);
        }
    }
    assert_that!(chunks).is_equal_to(vec!["ok".to_owned()]);
    assert_that!(serde_json::to_string(&t.hub.surfaces()).unwrap()).does_not_contain("ancien texte");
}

#[tokio::test(flavor = "multi_thread")]
async fn unresumed_session_opens_a_new_one_with_instructions_and_a_card_only_if_refused() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let lost: String = l.tr("agent-memory-lost").chars().take(30).collect();
    // Les consignes (`src/instructions/*.md`) sont en anglais seul, quelle que soit la langue
    // de l'interface ; le contexte leur donne la langue que leur règle cite.
    let instructions = ["You edit an Oku Slides presentation", "Creating a widget:", "interface language: French (fr)"];
    for caps in [json!({ "resume": true }), json!({ "load_session": true }), json!({})] {
        // Sans capacité, la reprise n'est pas tentée : pas de carte (t-611).
        let refused = caps.as_object().is_some_and(|c| !c.is_empty());
        let mut script = json!({ "unknown_sessions": true, "turns": [{ "chunks": ["ok"] }] });
        script.as_object_mut().unwrap().extend(caps.as_object().unwrap().clone());
        let h = start(script).await;
        seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
        let t = h.tab("t-1");
        t.agent.prompt("encore".into(), AgentUi::default());
        wait_turn_ended(t).await;
        assert_that!(logged(&h.log, "session/new")).has_length(1);
        assert_that!(serde_json::to_string(&t.hub.surfaces()).unwrap().contains(lost.as_str())).is_equal_to(refused);
        let prompt = logged(&h.log, "session/prompt")[0].to_string();
        for part in instructions {
            assert_that!(prompt.as_str()).named(part).contains(part);
        }
        // La reprise a été tentée avec la capacité annoncée, et refusée (aucune tentative sans capacité).
        let tried = |method: &str, cap: &str| {
            let expected: Vec<String> = if caps[cap] == true { vec!["sess-42".into()] } else { vec![] };
            assert_that!(sessions_in(&h.log, method)).is_equal_to(expected);
        };
        tried("session/resume", "resume");
        tried("session/load", "load_session");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn session_of_another_agent_is_not_resumed() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, AgentScope::Slides, "sess-42", "autre-agent").await;
    let t = h.tab("t-1");
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    assert_that!(logged(&h.log, "session/new")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn session_is_resumed_after_the_agent_stops() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }, { "crash": true }, { "chunks": ["ok"] }] }))
        .await;
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
    assert_that!(sessions_in(&h.log, "session/resume")).is_equal_to(vec!["sess-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_of_another_scope_during_a_turn_runs_next_in_its_own_session() {
    let h = start(
        json!({ "by_prompt": { "long": { "delay_ms": 600, "chunks": ["lent"] }, "court": { "chunks": ["vite"] } } }),
    )
    .await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt_in(atelier(), "long".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Working, .. })).await;
    t.agent.prompt_in(AgentScope::Slides, "court".into(), AgentUi::default());
    // Le tour de l'atelier est annulé puis la consigne des slides part (le faux agent
    // répond `end_turn` même annulé, sans `wait_cancel` : seule la suite compte ici).
    // Le morceau « lent » du tour annulé arrive encore dans le tour 1 : seul le tour 2 compte.
    let vite = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { turn: 2, .. })).await;
    assert_that!(vite).is_equal_to(AgentEvent::Chunk { turn: 2, text: "vite".into() });
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-1".into(), "sess-2".into()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_then_prompt_never_resumes_the_forgotten_session() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    t.agent.prompt_in(atelier(), "un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt_in(AgentScope::Slides, "deux".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.new_conversation(atelier());
    t.agent.prompt_in(atelier(), "trois".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt_in(AgentScope::Slides, "quatre".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec![
        "sess-1".into(),
        "sess-2".into(),
        "sess-3".into(),
        "sess-2".into(),
    ]);
    assert_that!(h.session("t-1").lock().unwrap().conversation(&atelier()).and_then(|c| c.session.clone()))
        .is_equal_to(Some("sess-3".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn renamed_widget_keeps_its_session_and_a_new_widget_of_the_old_name_gets_a_new_one() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    editing_with_kpi(&h);
    let t = h.tab("t-1");
    t.agent.prompt_in(widget("kpi"), "un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    on_session(&h, |s| {
        s.rename_widget(None, "kpi", "indicateur").unwrap();
        s.set_widget(None, "kpi", KPI.into()).unwrap();
    });
    t.agent.prompt_in(widget("indicateur"), "deux".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt_in(widget("kpi"), "trois".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec![
        "sess-1".into(),
        "sess-1".into(),
        "sess-2".into(),
    ]);
}

#[tokio::test(flavor = "multi_thread")]
async fn keeper_writes_the_thread_of_a_turn_under_its_renamed_scope() {
    let h = start(json!({ "turns": [] })).await;
    editing_with_kpi(&h);
    let t = h.tab("t-1");
    let n = t.hub.begin_turn(widget("kpi"));
    t.hub.emit(oku_slides_mcp::McpEvent::Surface { surface: frozen_turn(n) });
    t.hub.end_turn(n, "");
    on_session(&h, |s| s.rename_widget(None, "kpi", "indicateur").unwrap());
    t.keeper.send(Keep::Thread(n));
    t.keeper.flush().await;
    let s = h.session("t-1").lock().unwrap();
    assert_that!(s.conversation(&widget("indicateur")).map(|c| c.thread.len())).is_equal_to(Some(1));
    assert_that!(s.conversation(&widget("kpi"))).is_none();
}

/// t-613 : annuler le renommage ramène les tours du widget sous son ancien nom ;
/// le gardien y écrit leur fil.
#[tokio::test(flavor = "multi_thread")]
async fn keeper_writes_the_thread_under_the_old_name_once_the_rename_is_undone() {
    let h = start(json!({ "turns": [] })).await;
    editing_with_kpi(&h);
    on_session(&h, |s| {
        // Une autre étape d'abord : le renommage fait la sienne (pas celle de `kpi`).
        let op = oku_slides_core::Op::SetVar { key: "client".into(), value: json!("a") };
        s.apply("Variable", vec![op], None).unwrap_or_else(|e| panic!("variable : {e}"));
        s.rename_widget(None, "kpi", "indicateur").unwrap();
    });
    let t = h.tab("t-1");
    let n = t.hub.begin_turn(widget("indicateur"));
    t.hub.emit(oku_slides_mcp::McpEvent::Surface { surface: frozen_turn(n) });
    t.hub.end_turn(n, "");
    t.keeper.send(Keep::Sync);
    t.keeper.flush().await;
    on_session(&h, |s| s.undo().unwrap());
    t.keeper.send(Keep::Thread(n));
    t.keeper.flush().await;
    let s = h.session("t-1").lock().unwrap();
    assert_that!(s.conversation(&widget("kpi")).map(|c| c.thread.len())).is_equal_to(Some(1));
    assert_that!(s.conversation(&widget("indicateur"))).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn keeper_forget_after_a_queued_write_leaves_nothing() {
    let h = start(json!({ "turns": [] })).await;
    let t = h.tab("t-1");
    let n = t.hub.begin_turn(AgentScope::Slides);
    t.hub.emit(oku_slides_mcp::McpEvent::Surface { surface: frozen_turn(n) });
    t.hub.end_turn(n, "");
    t.keeper.send(Keep::Thread(n));
    assert_that!(t.keeper.forget(AgentScope::Slides).await).is_none();
    assert_that!(h.session("t-1").lock().unwrap().conversation(&AgentScope::Slides)).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn widget_renamed_while_its_session_opens_keeps_the_session_under_its_new_name() {
    let h = start(json!({ "new_session_delay_ms": 500, "turns": [{ "chunks": ["ok"] }] })).await;
    editing_with_kpi(&h);
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt_in(widget("kpi"), "un".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    // Pendant l'ouverture de la session : renommage, puis l'interface montre le nouveau nom
    // (le gardien applique le renommage à cet ordre-là, avant que la session soit connue).
    on_session(&h, |s| s.rename_widget(None, "kpi", "indicateur").unwrap());
    t.keeper.send(Keep::Show(widget("indicateur")));
    t.keeper.flush().await;
    assert_that!(h.session("t-1").lock().unwrap().conversation(&widget("indicateur")).and_then(|c| c.session.clone()))
        .is_none();
    wait_turn_ended(t).await;
    t.keeper.flush().await;
    let s = h.session("t-1").lock().unwrap();
    assert_that!(s.conversation(&widget("indicateur")).and_then(|c| c.session.clone()))
        .is_equal_to(Some("sess-1".to_owned()));
    assert_that!(s.conversation(&widget("kpi"))).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_of_another_scope_does_not_cancel_the_running_turn() {
    let h = start(
        json!({ "by_prompt": { "long": { "delay_ms": 600, "chunks": ["lent"] } }, "turns": [{ "chunks": ["ok"] }] }),
    )
    .await;
    let t = h.tab("t-1");
    t.agent.prompt("un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    let mut rx = t.agent.subscribe();
    t.agent.prompt_in(atelier(), "long".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Working, .. })).await;
    t.agent.new_conversation(AgentScope::Slides);
    let lent = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(lent).is_equal_to(AgentEvent::Chunk { turn: 2, text: "lent".into() });
    let ended = until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    assert_that!(ended).is_equal_to(AgentEvent::TurnEnded { turn: 2, stop: "end_turn".into() });
    let forgotten = until(&mut rx, |e| matches!(e, AgentEvent::Conversation { .. })).await;
    assert_that!(forgotten).is_equal_to(AgentEvent::Conversation { scope: AgentScope::Slides });
    assert_that!(logged(&h.log, "session/cancel")).is_empty();
    t.keeper.flush().await;
    let s = h.session("t-1").lock().unwrap();
    assert_that!(s.conversation(&AgentScope::Slides)).is_none();
    assert_that!(s.conversation(&atelier()).and_then(|c| c.session.clone())).is_equal_to(Some("sess-2".to_owned()));
}
