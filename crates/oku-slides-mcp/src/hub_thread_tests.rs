use std::sync::Arc;

use oku_slides_i18n::{LiveLocalizer, Localizer};
use oku_slides_session::agent::forms::{self, AskChoice};
use oku_slides_session::agent::turn_view::{TurnState, TurnView, turn_surface};
use oku_slides_session::agent::{KEPT_TURNS, SavedConversation, Sessions, Surface, TabSession};
use speculoos::prelude::*;

use super::{McpEvent, SurfaceHub, THREAD_BYTES};

#[derive(Debug)]
struct NoSession;
impl Sessions for NoSession {
    fn with(&self, _tab: &str, _f: &mut dyn FnMut(&mut oku_slides_session::Session)) -> oku_slides_session::Result<()> {
        Ok(())
    }

    fn open_pack(
        &self,
        _tab: &str,
        _pack: uuid::Uuid,
        _widget: Option<&str>,
        _gate: &mut dyn FnMut(oku_slides_session::agent::PackGate) -> bool,
    ) -> oku_slides_session::Result<oku_slides_session::agent::PackOpen> {
        unreachable!("NoSession is never asked to open a pack")
    }
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn hub() -> Arc<SurfaceHub> {
    SurfaceHub::new(
        TabSession::new("t-1", Arc::new(NoSession)),
        LiveLocalizer::new(Arc::new(fr())),
        Arc::new(crate::NoVoice),
    )
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
        images: Vec::new(),
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

/// Ouvre puis clôt un tour avec sa surface.
fn closed_turn(hub: &SurfaceHub) -> u32 {
    let n = hub.begin_turn();
    hub.emit(McpEvent::Surface { surface: turn(n) });
    hub.end_turn(n, "");
    n
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

fn saved(thread: Vec<Surface>) -> SavedConversation {
    SavedConversation { agent: None, session: None, thread }
}

#[test]
fn resync_sends_the_shown_surfaces_of_the_thread() {
    let hub = hub();
    let one = closed_turn(&hub);
    let mut rx = hub.subscribe();
    hub.resync();
    assert_that!(last_surfaces(&mut rx)).is_equal_to(vec![format!("turn-{one}")]);
}

#[test]
fn restore_shows_saved_thread_frozen_without_actions() {
    let hub = hub();
    let mut with_action = answered("report-3", 3);
    with_action.live = true;
    with_action.question = true;
    with_action.messages = question("report-3", 3).messages;
    hub.restore(Some(saved(vec![turn(3), with_action])));
    let kept = hub.visible_surfaces();
    assert_that!(ids(&kept)).is_equal_to(vec!["turn-3".to_owned(), "report-3".to_owned()]);
    assert_that!(kept.iter().any(|s| s.live || s.question || s.has_actions())).is_false();
}

#[test]
fn restore_numbers_new_turns_after_the_saved_ones() {
    let hub = hub();
    hub.restore(Some(saved(vec![turn(7), answered("report-12", 7)])));
    assert_that!(hub.begin_turn()).is_equal_to(8);
    assert_that!(hub.next_id("report")).is_equal_to("report-13".to_owned());
}

#[test]
fn restore_without_a_conversation_changes_nothing() {
    let hub = hub();
    hub.restore(None);
    assert_that!(hub.surfaces()).is_empty();
    assert_that!(hub.begin_turn()).is_equal_to(1);
}

#[test]
fn restore_keeps_only_the_last_kept_turns() {
    let hub = hub();
    let count = u32::try_from(KEPT_TURNS).unwrap() + 3;
    hub.restore(Some(saved((1..=count).map(turn).collect())));
    let turns: Vec<u32> = hub.visible_surfaces().iter().filter_map(|s| s.turn).collect();
    assert_that!(turns).has_length(KEPT_TURNS);
    assert_that!(turns.first().copied()).is_equal_to(Some(count + 1 - u32::try_from(KEPT_TURNS).unwrap()));
    assert_that!(turns.last().copied()).is_equal_to(Some(count));
}

#[test]
fn restore_skips_a_turn_already_open() {
    let hub = hub();
    let open = hub.begin_turn();
    hub.restore(Some(saved(vec![turn(open), turn(9)])));
    assert_that!(ids(&hub.surfaces())).is_equal_to(vec!["turn-9".to_owned()]);
}

#[test]
fn restore_drops_surfaces_outside_the_thread_place() {
    let hub = hub();
    let mut dock = turn(4);
    dock.place = oku_slides_session::agent::Place::Drawer;
    hub.restore(Some(saved(vec![dock, turn(5)])));
    assert_that!(ids(&hub.surfaces())).is_equal_to(vec!["turn-5".to_owned()]);
}

#[test]
fn restore_cleans_what_it_reads() {
    let hub = hub();
    let mut huge = turn(4);
    huge.turn = Some(u32::MAX);
    let mut no_turn = turn(5);
    no_turn.turn = None;
    hub.restore(Some(saved(vec![huge, no_turn, turn(6)])));
    assert_that!(ids(&hub.visible_surfaces())).is_equal_to(vec!["turn-6".to_owned()]);
}

#[test]
fn begin_turn_files_the_turn_in_the_thread() {
    let hub = hub();
    let n = hub.begin_turn();
    assert_that!(hub.in_thread(n)).is_true();
    assert_that!(hub.in_thread(n + 1)).is_false();
}

#[test]
fn begin_turn_beyond_kept_turns_closes_the_oldest() {
    let hub = hub();
    let first = closed_turn(&hub);
    for _ in 0..KEPT_TURNS {
        closed_turn(&hub);
    }
    assert_that!(hub.in_thread(first)).is_false();
    assert_that!(ids(&hub.surfaces())).does_not_contain(format!("turn-{first}"));
    assert_that!(hub.visible_surfaces()).has_length(KEPT_TURNS);
}

#[test]
fn clear_thread_forgets_every_turn() {
    let hub = hub();
    let a = closed_turn(&hub);
    let b = closed_turn(&hub);
    hub.clear_thread();
    assert_that!(hub.in_thread(a) || hub.in_thread(b)).is_false();
    assert_that!(hub.visible_surfaces()).is_empty();
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
async fn clear_thread_releases_the_questions() {
    let hub = hub();
    let n = hub.begin_turn();
    let waiting = pending_ask(&hub, "ask-1", n).await;
    hub.clear_thread();
    assert_that!(matches!(waiting.await.unwrap(), crate::Answer::Cancelled)).is_true();
    assert_that!(hub.has_pending_ask()).is_false();
}

#[test]
fn surface_outside_any_turn_is_shown() {
    let hub = hub();
    let mut free = turn(1);
    free.turn = None;
    hub.emit(McpEvent::Surface { surface: free });
    assert_that!(hub.visible_surfaces()).has_length(1);
}

#[test]
fn surface_of_a_turn_never_filed_is_not_shown() {
    let hub = hub();
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::Surface { surface: turn(42) });
    assert_that!(rx.try_recv().is_err()).is_true();
    assert_that!(hub.visible_surfaces()).is_empty();
}

#[test]
fn thread_snapshot_returns_closed_turns_only() {
    let hub = hub();
    let a = hub.begin_turn();
    hub.emit(McpEvent::Surface { surface: turn(a) });
    hub.emit(McpEvent::Surface { surface: question("ask-1", a) });
    hub.emit(McpEvent::Surface { surface: answered("report-2", a) });
    hub.end_turn(a, "");
    let open = hub.begin_turn();
    hub.emit(McpEvent::Surface { surface: turn(open) });
    let kept = hub.thread_snapshot().unwrap_or_default();
    assert_that!(ids(&kept)).is_equal_to(vec![format!("turn-{a}"), "report-2".to_owned()]);
    assert_that!(kept.iter().any(Surface::has_actions)).is_false();
}

#[test]
fn thread_snapshot_of_an_empty_thread_is_empty() {
    assert_that!(hub().thread_snapshot()).is_equal_to(Some(Vec::new()));
}

fn big_turn(hub: &SurfaceHub, kib: usize) -> u32 {
    let n = hub.begin_turn();
    let mut big = turn(n);
    big.set_value("narration", &serde_json::Value::String("x".repeat(kib * 1024)));
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
    let kept = hub.thread_snapshot().unwrap_or_default();
    let bytes: usize = kept.iter().map(|s| serde_json::to_vec(s).map_or(0, |b| b.len())).sum();
    assert_that!(bytes).is_less_than_or_equal_to(THREAD_BYTES);
    let turns: Vec<u32> = kept.iter().filter_map(|s| s.turn).collect();
    assert_that!(turns.len()).is_greater_than(1);
    assert_that!(turns.last().copied()).is_equal_to(Some(last));
    assert_that!(turns.windows(2).all(|w| w[1] == w[0] + 1)).is_true();
}

#[test]
fn thread_snapshot_over_budget_returns_none() {
    let hub = hub();
    big_turn(&hub, 900);
    assert_that!(hub.thread_snapshot()).is_none();
}
