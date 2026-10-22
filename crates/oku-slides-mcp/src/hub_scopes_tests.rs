use std::sync::Arc;

use oku_slides_i18n::{LiveLocalizer, Localizer};
use oku_slides_session::agent::forms::{self, AskChoice};
use oku_slides_session::agent::turn_view::{TurnState, TurnView, turn_surface};
use oku_slides_session::agent::{AgentScope, KEPT_TURNS, SavedConversation, Sessions, Surface, TabSession};
use oku_slides_session::pack::ESSENTIEL_PACK;
use speculoos::prelude::*;
use uuid::Uuid;

use super::{McpEvent, SurfaceHub, THREAD_BYTES};

#[derive(Debug)]
struct NoSession;
impl Sessions for NoSession {
    fn with(&self, _tab: &str, _f: &mut dyn FnMut(&mut oku_slides_session::Session)) -> oku_slides_session::Result<()> {
        Ok(())
    }
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn hub() -> Arc<SurfaceHub> {
    SurfaceHub::new(TabSession::new("t-1", Arc::new(NoSession)), LiveLocalizer::new(Arc::new(fr())))
}

fn atelier() -> AgentScope {
    AgentScope::Atelier { pack: Uuid::from_u128(1) }
}

/// La surface du tour `n`, clos.
fn turn(n: u32) -> Surface {
    let view = TurnView {
        n,
        prompt: format!("consigne {n}"),
        plan: vec![],
        steps: vec![],
        tools: 0,
        notes: 0,
        narration: String::new(),
        state: TurnState::Done { secs: 1 },
    };
    turn_surface(&view, &fr())
}

/// Une question du fil, vivante, posée dans le tour `n`.
fn question(id: &str, n: u32) -> Surface {
    let option = |id: &str| oku_slides_a2ui::ChoiceOption { id: id.into(), label: id.into(), hint: None };
    let mut s = forms::choice(
        id,
        &AskChoice { question: "?".into(), options: vec![option("a"), option("b")], anchor: None },
        &fr(),
    );
    s.turn = Some(n);
    s
}

/// La même question, répondue et figée dans son tour.
fn answered(id: &str, n: u32) -> Surface {
    let mut s = forms::frozen_choice(id, "?", Some("a"), &fr());
    s.turn = Some(n);
    s
}

fn ids(list: &[Surface]) -> Vec<String> {
    list.iter().map(|s| s.id.clone()).collect()
}

fn last_surfaces(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> Vec<String> {
    let mut found = None;
    while let Ok(e) = rx.try_recv() {
        if let McpEvent::Surfaces { list } = e {
            found = Some(ids(&list));
        }
    }
    found.unwrap_or_else(|| panic!("liste attendue"))
}

/// Ouvre puis clôt un tour de `scope` avec sa surface.
fn closed_turn(hub: &SurfaceHub, scope: AgentScope) -> u32 {
    let n = hub.begin_turn(scope);
    hub.emit(McpEvent::Surface { surface: turn(n) });
    hub.end_turn(n, "");
    n
}

#[test]
fn resync_sends_only_the_thread_of_the_viewed_scope() {
    let hub = hub();
    let one = closed_turn(&hub, AgentScope::Slides);
    let two = closed_turn(&hub, atelier());
    let mut rx = hub.subscribe();
    hub.resync(AgentScope::Slides);
    assert_that!(last_surfaces(&mut rx)).is_equal_to(vec![format!("turn-{one}")]);
    hub.resync(atelier());
    assert_that!(last_surfaces(&mut rx)).is_equal_to(vec![format!("turn-{two}")]);
    assert_that!(hub.surfaces()).has_length(2);
}

#[test]
fn surface_of_a_turn_in_another_scope_is_kept_but_not_sent() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::Surface { surface: turn(n) });
    assert_that!(rx.try_recv().is_err()).is_true();
    assert_that!(ids(&hub.surfaces())).is_equal_to(vec![format!("turn-{n}")]);
}

#[test]
fn live_surface_of_another_scope_is_sent_then_closed_once_hidden() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::Surface { surface: question("ask-1", n) });
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::Surface { .. }))).is_true();
    hub.emit(McpEvent::Surface { surface: answered("ask-1", n) });
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::CloseSurface { id }) if id == "ask-1")).is_true();
}

#[test]
fn hidden_surface_never_shown_sends_no_close() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::Surface { surface: answered("ask-1", n) });
    hub.emit(McpEvent::Surface { surface: answered("ask-1", n) });
    assert_that!(rx.try_recv().is_err()).is_true();
}

#[test]
fn patch_of_a_hidden_surface_is_not_sent() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    hub.emit(McpEvent::Surface { surface: turn(n) });
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::SurfacePatch { id: format!("turn-{n}"), messages: vec![] });
    assert_that!(rx.try_recv().is_err()).is_true();
}

async fn pending_ask(hub: &Arc<SurfaceHub>, id: &'static str, n: u32) -> tokio::task::JoinHandle<crate::Answer> {
    let task = tokio::spawn({
        let hub = hub.clone();
        async move { hub.ask_then(Some(n), question(id, n), Box::new(move |_| answered(id, n))).await }
    });
    while !hub.surfaces().iter().any(|s| s.id == id) {
        tokio::task::yield_now().await;
    }
    task
}

#[tokio::test]
async fn clear_thread_keeps_the_questions_of_other_scopes() {
    let hub = hub();
    let n = hub.begin_turn(AgentScope::Slides);
    let waiting = pending_ask(&hub, "ask-1", n).await;
    hub.clear_thread(&atelier());
    tokio::task::yield_now().await;
    assert_that!(waiting.is_finished()).is_false();
    assert_that!(hub.has_pending_ask()).is_true();
    hub.cancel_asks();
}

#[tokio::test]
async fn clear_thread_releases_the_questions_of_its_scope() {
    let hub = hub();
    let n = hub.begin_turn(AgentScope::Slides);
    let waiting = pending_ask(&hub, "ask-1", n).await;
    hub.clear_thread(&AgentScope::Slides);
    assert_that!(matches!(waiting.await.unwrap(), crate::Answer::Cancelled)).is_true();
    assert_that!(hub.has_pending_ask()).is_false();
}

#[test]
fn clear_thread_leaves_the_other_scopes() {
    let hub = hub();
    let a = closed_turn(&hub, AgentScope::Slides);
    let b = closed_turn(&hub, atelier());
    hub.clear_thread(&atelier());
    assert_that!(ids(&hub.surfaces())).is_equal_to(vec![format!("turn-{a}")]);
    assert_that!(hub.turn_scope(b)).is_none();
}

#[test]
fn begin_turn_closes_the_oldest_turns_of_its_scope_only() {
    let hub = hub();
    let other = closed_turn(&hub, atelier());
    for _ in 0..=KEPT_TURNS {
        closed_turn(&hub, AgentScope::Slides);
    }
    hub.resync(AgentScope::Slides);
    assert_that!(hub.visible_surfaces()).has_length(KEPT_TURNS);
    assert_that!(ids(&hub.surfaces())).contains(format!("turn-{other}"));
}

#[test]
fn restore_puts_threads_back_and_numbers_new_turns_after_them() {
    let hub = hub();
    hub.restore(vec![SavedConversation {
        scope: atelier(),
        agent: None,
        session: None,
        thread: vec![turn(7), answered("report-12", 7)],
    }]);
    hub.resync(atelier());
    assert_that!(ids(&hub.visible_surfaces())).is_equal_to(vec!["turn-7".to_owned(), "report-12".to_owned()]);
    hub.resync(AgentScope::Slides);
    assert_that!(hub.visible_surfaces()).is_empty();
    assert_that!(hub.begin_turn(AgentScope::Slides)).is_equal_to(8);
    assert_that!(hub.next_id("report")).is_equal_to("report-13".to_owned());
}

#[test]
fn restore_cleans_what_it_reads() {
    let hub = hub();
    let mut tampered = answered("report-3", 3);
    tampered.live = true;
    tampered.question = true;
    tampered.messages = question("report-3", 3).messages; // avec ses actions
    let mut huge = turn(4);
    huge.turn = Some(u32::MAX);
    let mut no_turn = turn(5);
    no_turn.turn = None;
    hub.restore(vec![SavedConversation {
        scope: atelier(),
        agent: None,
        session: None,
        thread: vec![tampered, huge, no_turn],
    }]);
    hub.resync(atelier());
    let kept = hub.visible_surfaces();
    assert_that!(ids(&kept)).is_equal_to(vec!["report-3".to_owned()]);
    assert_that!(kept[0].live || kept[0].question || kept[0].has_actions()).is_false();
    assert_that!(hub.begin_turn(AgentScope::Slides)).is_equal_to(4);
}

#[test]
fn restore_ignores_a_turn_already_taken() {
    let hub = hub();
    let open = hub.begin_turn(AgentScope::Slides);
    hub.restore(vec![
        SavedConversation { scope: atelier(), agent: None, session: None, thread: vec![turn(open), turn(9)] },
        SavedConversation {
            scope: AgentScope::Widget { pack: Uuid::from_u128(1), name: "kpi".into() },
            agent: None,
            session: None,
            thread: vec![answered("report-20", 9)],
        },
    ]);
    assert_that!(hub.turn_scope(open)).is_equal_to(Some(AgentScope::Slides));
    assert_that!(hub.turn_scope(9)).is_equal_to(Some(atelier()));
    assert_that!(ids(&hub.surfaces())).does_not_contain("report-20".to_owned());
}

#[test]
fn thread_snapshot_keeps_closed_frozen_surfaces_of_the_scope_without_actions() {
    let hub = hub();
    let a = hub.begin_turn(AgentScope::Slides);
    hub.emit(McpEvent::Surface { surface: turn(a) });
    hub.emit(McpEvent::Surface { surface: question("ask-1", a) });
    hub.emit(McpEvent::Surface { surface: answered("report-2", a) });
    hub.end_turn(a, "");
    closed_turn(&hub, atelier());
    let open = hub.begin_turn(AgentScope::Slides);
    hub.emit(McpEvent::Surface { surface: turn(open) });
    let kept = hub.thread_snapshot(&AgentScope::Slides).unwrap_or_default();
    assert_that!(ids(&kept)).is_equal_to(vec![format!("turn-{a}"), "report-2".to_owned()]);
    assert_that!(kept.iter().any(Surface::has_actions)).is_false();
}

fn big_turn(hub: &SurfaceHub, kib: usize) -> u32 {
    let n = hub.begin_turn(AgentScope::Slides);
    let mut big = turn(n);
    big.set_value("narration", serde_json::Value::String("x".repeat(kib * 1024)));
    hub.emit(McpEvent::Surface { surface: big });
    hub.end_turn(n, "");
    n
}

#[test]
fn thread_snapshot_keeps_a_contiguous_suffix_within_its_byte_budget() {
    let hub = hub();
    let mut last = 0;
    for _ in 0..KEPT_TURNS {
        last = big_turn(&hub, 40);
    }
    let kept = hub.thread_snapshot(&AgentScope::Slides).unwrap_or_default();
    let bytes: usize = kept.iter().map(|s| serde_json::to_vec(s).map_or(0, |b| b.len())).sum();
    assert_that!(bytes).is_less_than_or_equal_to(THREAD_BYTES);
    let turns: Vec<u32> = kept.iter().filter_map(|s| s.turn).collect();
    assert_that!(turns.len()).is_greater_than(1);
    assert_that!(turns.last().copied()).is_equal_to(Some(last));
    assert_that!(turns.windows(2).all(|w| w[1] == w[0] + 1)).is_true();
}

#[test]
fn thread_snapshot_with_an_oversized_last_turn_writes_nothing() {
    let hub = hub();
    big_turn(&hub, 900);
    assert_that!(hub.thread_snapshot(&AgentScope::Slides)).is_none();
}

#[test]
fn rename_scope_moves_turns_and_the_viewed_scope() {
    let hub = hub();
    let from = AgentScope::Widget { pack: Uuid::from_u128(1), name: "kpi".into() };
    let to = AgentScope::Widget { pack: Uuid::from_u128(1), name: "indicateur".into() };
    let n = closed_turn(&hub, from.clone());
    hub.resync(from.clone());
    hub.rename_scope(&from, &to);
    assert_that!(hub.turn_scope(n)).is_equal_to(Some(to.clone()));
    hub.resync(to);
    assert_that!(hub.visible_surfaces()).has_length(1);
}

fn viewing_events(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> Vec<AgentScope> {
    let mut found = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let McpEvent::Viewing { scope } = e {
            found.push(scope);
        }
    }
    found
}

#[test]
fn resync_announces_the_viewed_scope_before_the_list() {
    let hub = hub();
    let mut rx = hub.subscribe();
    hub.resync(atelier());
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::Viewing { scope }) if scope == atelier())).is_true();
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::Surfaces { .. }))).is_true();
}

#[test]
fn viewing_before_any_resync_is_slides() {
    assert_that!(hub().viewing()).is_equal_to(AgentScope::Slides);
}

#[test]
fn viewing_after_resync_is_the_scope_shown() {
    let hub = hub();
    hub.resync(atelier());
    assert_that!(hub.viewing()).is_equal_to(atelier());
}

#[test]
fn rename_of_the_viewed_scope_announces_the_new_one() {
    let hub = hub();
    let from = AgentScope::Widget { pack: ESSENTIEL_PACK, name: "kpi".into() };
    let to = AgentScope::Widget { pack: ESSENTIEL_PACK, name: "score".into() };
    hub.resync(from.clone());
    let mut rx = hub.subscribe();
    hub.rename_scope(&from, &to);
    assert_that!(viewing_events(&mut rx)).is_equal_to(vec![to]);
}

#[test]
fn rename_of_another_scope_announces_nothing() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let mut rx = hub.subscribe();
    hub.rename_scope(&atelier(), &AgentScope::Atelier { pack: uuid::Uuid::nil() });
    assert_that!(viewing_events(&mut rx)).is_empty();
}
