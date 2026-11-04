use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use async_trait::async_trait;
use oku_slides_mcp::{AfterTurn, CloseDecision, CloseTarget, McpEvent, SurfaceHub};
use oku_slides_session::agent::{
    AgentWorkspace, AppPort, AppRefusal, CloseScope, CloseTabs, FocusedTab, Sessions, TabGate, TabOpen, TabRevision,
    UnavailableApp, UpdateAction, UpdateState, WorkspaceRefusal,
};
use oku_slides_session::{DocumentKind, Fonts, Platform, Session, Settings};
use speculoos::prelude::*;
use tokio::runtime::Handle;

use super::{AppAction, Queued, RunnerEnd, drain, drain_after_reply, run, spawn};
use crate::Stack;

/// A workspace whose unsaved tab count, and `t-1`'s dirty revision, the test sets.
#[derive(Debug, Default)]
struct Unsaved(AtomicUsize, Mutex<Option<u64>>);

impl AgentWorkspace for Unsaved {
    fn focused_tab(&self) -> Option<FocusedTab> {
        None
    }

    fn unsaved_tabs(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }

    /// `t-1` alone, in window `w-1`, whatever the scope.
    fn dirty_revisions(&self, _: &str, _: CloseScope<'_>) -> Result<CloseTabs, WorkspaceRefusal> {
        let revision = *self.1.lock().unwrap();
        let tab = TabRevision { tab: "t-1".into(), title: "Deck".into(), dirty: revision.is_some(), revision };
        Ok(CloseTabs { window: "w-1".into(), tabs: vec![tab], last_window: false })
    }
}

/// A stored close of `t-1`, confirmed while clean.
fn clean_close(hub: &SurfaceHub) -> uuid::Uuid {
    let decision = CloseDecision { discard: false, revisions: vec![("t-1".into(), None)] };
    hub.store_close(CloseTarget::Tab { tab: "t-1".into() }, "w-1", decision)
}

/// An app whose quit stops the stack, as the host's does, then says so on `quit_done`.
#[derive(Debug)]
struct StoppingApp {
    stack: OnceLock<Weak<Stack>>,
    rt: Handle,
    quit_done: Mutex<mpsc::Sender<()>>,
    quits: AtomicUsize,
}

#[async_trait]
impl AppPort for StoppingApp {
    async fn update(&self, _: UpdateAction) -> Result<UpdateState, AppRefusal> {
        Ok(UpdateState::UpToDate)
    }

    fn can_quit(&self) -> Result<(), AppRefusal> {
        Ok(())
    }

    fn quit(&self) -> Result<(), AppRefusal> {
        self.quits.fetch_add(1, Ordering::SeqCst);
        if let Some(stack) = self.stack.get().and_then(Weak::upgrade) {
            self.rt.block_on(stack.shutdown());
        }
        // The test may have stopped waiting: nothing to tell then.
        let _ = self.quit_done.lock().unwrap_or_else(std::sync::PoisonError::into_inner).send(());
        Ok(())
    }

    fn install(&self) -> Result<(), AppRefusal> {
        Err(AppRefusal::unavailable())
    }
}

/// A stack in French on one deck session (any tab id is that deck).
struct Fixture {
    stack: Arc<Stack>,
    platform: Platform,
    workspace: Arc<Unsaved>,
    /// The tab sessions: their `Session`'s storage owns a runtime, dropped off it in `finish`.
    shown: Arc<Shown>,
    _dir: tempfile::TempDir,
}

impl Fixture {
    /// Stops the stack, then drops the storage (`hub` and the sessions hold it) off the runtime's threads.
    async fn finish(self, hub: Option<Arc<SurfaceHub>>) {
        let Self { stack, platform, shown, _dir, .. } = self;
        stack.shutdown().await;
        tokio::task::spawn_blocking(move || drop((stack, platform, hub, shown))).await.unwrap();
    }
}

/// Tab sessions whose tab is shown in its window only while `.1` holds.
#[derive(Debug)]
struct Shown(Mutex<Session>, std::sync::atomic::AtomicBool);

impl Sessions for Shown {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        self.0.with(tab, f)
    }

    fn shown(&self, _tab: &str) -> bool {
        self.1.load(Ordering::SeqCst)
    }

    fn open_pack(
        &self,
        tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.0.open_pack(tab, pack, widget, gate)
    }

    fn open_theme(
        &self,
        tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.0.open_theme(tab, theme, gate)
    }

    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<DocumentKind> {
        self.0.document_kind(tab)
    }

    fn usage_owner(&self, tab: &str) -> Option<String> {
        self.0.usage_owner(tab)
    }
}

async fn fixture(app: impl FnOnce() -> Arc<dyn AppPort>) -> Fixture {
    fixture_shown(app, true).await
}

/// [`fixture`], its tab shown in its window while `Fixture::shown` says so.
async fn fixture_shown(app: impl FnOnce() -> Arc<dyn AppPort>, shown: bool) -> Fixture {
    let dir = tempfile::Builder::new().prefix("oku-slides-after-turn-").tempdir().unwrap();
    let data = dir.path().to_owned();
    let (platform, shown) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap();
        settings.paths.decks = Some(data.join("decks"));
        settings.paths.data = Some(data);
        settings.i18n.locale = Some("fr".into());
        let platform = Platform::with_settings(settings).unwrap();
        let session = Mutex::new(Session::new(Fonts::default(), &platform).unwrap());
        (platform, Arc::new(Shown(session, std::sync::atomic::AtomicBool::new(shown))))
    })
    .await
    .unwrap();
    let workspace = Arc::new(Unsaved::default());
    let sessions: Arc<dyn Sessions> = shown.clone();
    let stack =
        Stack::start(sessions, &platform, crate::Voices::default(), workspace.clone(), app(), Handle::current());
    Fixture { stack, platform, workspace, shown, _dir: dir }
}

/// Turn `n` of tab `t-1`, ended, with `actions` recorded while it was open.
fn ended_turn(stack: &Stack, actions: &[AfterTurn]) -> (Arc<SurfaceHub>, u32) {
    let hub = stack.open("t-1").unwrap().hub;
    let n = hub.begin_turn();
    for a in actions {
        assert_that!(hub.after_turn(n, *a)).is_true();
    }
    hub.end_turn(n, "Fait.");
    (hub, n)
}

fn queued(turn: u32, action: AppAction) -> Queued {
    Queued { tab: "t-1".into(), turn, action }
}

/// Whether some surface of `hub` shows `text`.
fn shows(hub: &SurfaceHub, text: &str) -> bool {
    hub.surfaces().iter().any(|s| serde_json::to_string(s).unwrap().contains(text))
}

#[tokio::test(flavor = "multi_thread")]
async fn drain_finished_turn_queues_quit_and_returns_new_conversation() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let (hub, n) = ended_turn(&f.stack, &[AfterTurn::NewConversation, AfterTurn::Quit]);
    let (tx, rx) = mpsc::channel();
    assert_that!(drain(&hub, Some(&tx), "t-1", n, true)).is_true();
    assert_that!(rx.try_iter().collect::<Vec<_>>()).is_equal_to(vec![queued(n, AppAction::Quit)]);
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn drain_unfinished_turn_drops_everything() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let (hub, n) = ended_turn(&f.stack, &[AfterTurn::NewConversation, AfterTurn::Install]);
    let (tx, rx) = mpsc::channel();
    assert_that!(drain(&hub, Some(&tx), "t-1", n, false)).is_false();
    assert_that!(rx.try_iter().count()).is_equal_to(0);
    assert_that!(hub.take_after_turn(n)).is_empty();
    f.finish(Some(hub)).await;
}

#[test]
fn run_panicking_step_is_caught_and_ends_panicked() {
    let (tx, rx) = mpsc::channel();
    tx.send(queued(1, AppAction::Quit)).unwrap();
    let end = run(&rx, |q| if q.is_some() { panic!("step failed") } else { true });
    assert_that!(end).is_equal_to(RunnerEnd::Panicked);
}

#[test]
fn run_without_senders_ends_disconnected() {
    let (tx, rx) = mpsc::channel::<Queued>();
    drop(tx);
    assert_that!(run(&rx, |_| true)).is_equal_to(RunnerEnd::Disconnected);
}

#[test]
fn spawn_on_a_gone_stack_ends_closed() {
    let (_tx, rx) = mpsc::channel();
    let runner = spawn(Weak::new(), rx).unwrap();
    assert_that!(runner.join().unwrap()).is_equal_to(RunnerEnd::Closed);
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_quit_runs_after_reply_without_deadlock() {
    let (done_tx, done_rx) = mpsc::channel();
    let app = Arc::new(StoppingApp {
        stack: OnceLock::new(),
        rt: Handle::current(),
        quit_done: Mutex::new(done_tx),
        quits: AtomicUsize::new(0),
    });
    let port = app.clone();
    let f = fixture(move || port).await;
    app.stack.set(Arc::downgrade(&f.stack)).unwrap();
    let (_, n) = ended_turn(&f.stack, &[AfterTurn::Quit]);
    let (tx, rx) = mpsc::channel();
    let runner = spawn(Arc::downgrade(&f.stack), rx).unwrap();
    tx.send(queued(n, AppAction::Quit)).unwrap();
    // The quit stops the stack from the after-turn thread: it must come back, not wait on itself.
    let waited = tokio::task::spawn_blocking(move || done_rx.recv_timeout(Duration::from_secs(10))).await.unwrap();
    assert_that!(waited).is_ok();
    assert_that!(app.quits.load(Ordering::SeqCst)).is_equal_to(1);
    let end = tokio::task::spawn_blocking(move || runner.join().unwrap()).await.unwrap();
    assert_that!(end).is_equal_to(RunnerEnd::Closed);
    f.finish(None).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_quit_dropped_when_tab_became_dirty() {
    let (done_tx, _done_rx) = mpsc::channel();
    let app = Arc::new(StoppingApp {
        stack: OnceLock::new(),
        rt: Handle::current(),
        quit_done: Mutex::new(done_tx),
        quits: AtomicUsize::new(0),
    });
    let port = app.clone();
    let f = fixture(move || port).await;
    f.workspace.0.store(1, Ordering::SeqCst);
    let (hub, n) = ended_turn(&f.stack, &[AfterTurn::Quit]);
    let stack = f.stack.clone();
    tokio::task::spawn_blocking(move || stack.run_after_turn(&queued(n, AppAction::Quit))).await.unwrap();
    assert_that!(app.quits.load(Ordering::SeqCst)).is_equal_to(0);
    let notice = f.platform.live_localizer.current().tr("agent-after-turn-quit-unsaved");
    assert_that!(shows(&hub, &notice)).is_true();
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_install_refused_shows_localized_notice() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let (hub, n) = ended_turn(&f.stack, &[AfterTurn::Install]);
    let stack = f.stack.clone();
    tokio::task::spawn_blocking(move || stack.run_after_turn(&queued(n, AppAction::Install))).await.unwrap();
    assert_that!(shows(&hub, "Seule l'app Oku Slides peut le faire")).is_true();
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn drain_after_reply_queues_an_install_only_once_the_receipt_is_in() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let n = hub.begin_turn();
    assert_that!(hub.after_turn(n, AfterTurn::Install)).is_true();
    let (tx, rx) = mpsc::channel();
    let mut drained = Box::pin(drain_after_reply(&hub, Some(&tx), "t-1", n, true));
    // The turn has not ended on the relay yet: the drain waits, nothing is queued.
    assert_that!(futures::poll!(drained.as_mut()).is_pending()).is_true();
    assert_that!(rx.try_iter().count()).is_equal_to(0);
    hub.end_turn(n, "Fait.");
    assert_that!(drained.await).is_false();
    assert_that!(rx.try_iter().collect::<Vec<_>>()).is_equal_to(vec![queued(n, AppAction::Install)]);
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_close_own_announced_when_unchanged() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let id = clean_close(&hub);
    let (_, n) = ended_turn(&f.stack, &[AfterTurn::CloseOwn { decision: id }]);
    let mut rx = hub.subscribe();
    let stack = f.stack.clone();
    tokio::task::spawn_blocking(move || stack.run_after_turn(&queued(n, AppAction::CloseOwn { decision: id })))
        .await
        .unwrap();
    let closes: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|e| match e {
            McpEvent::Close { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();
    assert_that!(closes).is_equal_to(vec![id.to_string()]);
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_close_own_dropped_when_tab_changed() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let id = clean_close(&hub);
    let (_, n) = ended_turn(&f.stack, &[AfterTurn::CloseOwn { decision: id }]);
    *f.workspace.1.lock().unwrap() = Some(3);
    let mut rx = hub.subscribe();
    let stack = f.stack.clone();
    tokio::task::spawn_blocking(move || stack.run_after_turn(&queued(n, AppAction::CloseOwn { decision: id })))
        .await
        .unwrap();
    let closed = std::iter::from_fn(|| rx.try_recv().ok()).any(|e| matches!(e, McpEvent::Close { .. }));
    assert_that!(closed).is_false();
    assert_that!(hub.close_decision(id)).is_none();
    let notice = f.platform.live_localizer.current().tr("agent-after-turn-close-unsaved");
    assert_that!(shows(&hub, &notice)).is_true();
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn drain_queues_close_own_and_waits_for_the_receipt() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let id = clean_close(&hub);
    let n = hub.begin_turn();
    assert_that!(hub.after_turn(n, AfterTurn::CloseOwn { decision: id })).is_true();
    assert_that!(hub.after_turn(n, AfterTurn::Quit)).is_false();
    let (tx, rx) = mpsc::channel();
    let mut drained = Box::pin(drain_after_reply(&hub, Some(&tx), "t-1", n, true));
    assert_that!(futures::poll!(drained.as_mut()).is_pending()).is_true();
    hub.end_turn(n, "Fait.");
    assert_that!(drained.await).is_false();
    assert_that!(rx.try_iter().collect::<Vec<_>>()).is_equal_to(vec![queued(n, AppAction::CloseOwn { decision: id })]);
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_close_own_announced_when_tab_saved_since() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let dirty = CloseDecision { discard: true, revisions: vec![("t-1".into(), Some(3))] };
    let id = hub.store_close(CloseTarget::Tab { tab: "t-1".into() }, "w-1", dirty);
    let (_, n) = ended_turn(&f.stack, &[AfterTurn::CloseOwn { decision: id }]);
    let mut rx = hub.subscribe();
    let stack = f.stack.clone();
    tokio::task::spawn_blocking(move || stack.run_after_turn(&queued(n, AppAction::CloseOwn { decision: id })))
        .await
        .unwrap();
    let closed = std::iter::from_fn(|| rx.try_recv().ok()).any(|e| matches!(e, McpEvent::Close { .. }));
    assert_that!(closed).is_true();
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_close_own_dropped_when_tab_no_longer_shown() {
    let f = fixture_shown(|| Arc::new(UnavailableApp), true).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let id = clean_close(&hub);
    let (_, n) = ended_turn(&f.stack, &[AfterTurn::CloseOwn { decision: id }]);
    // The user showed another tab before the reply ended: no page would receive the close.
    f.shown.1.store(false, Ordering::SeqCst);
    let mut rx = hub.subscribe();
    let stack = f.stack.clone();
    tokio::task::spawn_blocking(move || stack.run_after_turn(&queued(n, AppAction::CloseOwn { decision: id })))
        .await
        .unwrap();
    let closed = std::iter::from_fn(|| rx.try_recv().ok()).any(|e| matches!(e, McpEvent::Close { .. }));
    assert_that!(closed).is_false();
    assert_that!(hub.close_decision(id)).is_none();
    let notice = f.platform.live_localizer.current().tr("agent-after-turn-close-not-shown");
    assert_that!(shows(&hub, &notice)).is_true();
    f.finish(Some(hub)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn drain_unfinished_turn_drops_its_own_close_decision() {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let id = clean_close(&hub);
    let (_, n) = ended_turn(&f.stack, &[AfterTurn::CloseOwn { decision: id }]);
    assert_that!(drain(&hub, None, "t-1", n, false)).is_false();
    assert_that!(hub.close_decision(id)).is_none();
    f.finish(Some(hub)).await;
}

/// Runs `t-1`'s own close, confirmed at revision 3, with the tab now at `now`: sent? kept?
async fn close_own_confirmed_at_3(now: Option<u64>) -> (bool, Option<(CloseTarget, CloseDecision)>) {
    let f = fixture(|| Arc::new(UnavailableApp)).await;
    let hub = f.stack.open("t-1").unwrap().hub;
    let dirty = CloseDecision { discard: true, revisions: vec![("t-1".into(), Some(3))] };
    let id = hub.store_close(CloseTarget::Tab { tab: "t-1".into() }, "w-1", dirty);
    let (_, n) = ended_turn(&f.stack, &[AfterTurn::CloseOwn { decision: id }]);
    *f.workspace.1.lock().unwrap() = now;
    let mut rx = hub.subscribe();
    let stack = f.stack.clone();
    tokio::task::spawn_blocking(move || stack.run_after_turn(&queued(n, AppAction::CloseOwn { decision: id })))
        .await
        .unwrap();
    let closed = std::iter::from_fn(|| rx.try_recv().ok()).any(|e| matches!(e, McpEvent::Close { .. }));
    let kept = hub.close_decision(id);
    f.finish(Some(hub)).await;
    (closed, kept)
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_close_own_announced_when_still_at_the_confirmed_revision() {
    let (closed, _) = close_own_confirmed_at_3(Some(3)).await;
    assert_that!(closed).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn after_turn_close_own_dropped_when_edited_past_the_confirmed_revision() {
    let (closed, kept) = close_own_confirmed_at_3(Some(4)).await;
    assert_that!(closed).is_false();
    assert_that!(kept).is_none();
}
