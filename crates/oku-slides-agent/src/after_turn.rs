//! What runs once a turn has ended: a new conversation, quitting, installing an update, closing
//! the agent's own tab.
//!
//! A turn's tools record these in its hub (`SurfaceHub::after_turn`); the tab's actor takes them
//! once, after the turn's reply and, for a quit, an install or a close, its receipt ([`drain`]). A
//! new conversation goes back to the actor; the rest goes to the stack's after-turn thread, never
//! to a tool or the actor: the host's quit stops the stack, which waits for the actor. A stopped,
//! cancelled or failed turn drops them all. Right before it runs, a quit checks again for unsaved
//! work, and a close for the covered tabs' dirty revisions; either is dropped, with a notice in the
//! turn, if something changed. The thread holds the stack weakly.

use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use oku_slides_i18n::Localize;
use oku_slides_mcp::{AfterTurn, CloseTarget, SurfaceHub};
use oku_slides_session::agent::CloseScope;
use uuid::Uuid;

use crate::Stack;

/// [`drain`] once the reply is out; anything recorded first waits for the turn's receipt.
///
/// The receipt is put by the relay ([`SurfaceHub::end_turn`]), which reads the turn's end after
/// the actor: waited for here, [`RECEIPT_WITHIN`] at most, so it shows before the app goes, and
/// before a new conversation clears the thread it goes in (t-1000).
pub(crate) async fn drain_after_reply(
    hub: &SurfaceHub,
    after: Option<&AfterTx>,
    tab: &str,
    turn: u32,
    finished: bool,
) -> bool {
    if finished && hub.records_after(turn) && !hub.turn_ended(turn, RECEIPT_WITHIN).await {
        tracing::warn!(tab, turn, "after turn: the turn's end not relayed in time, running anyway");
    }
    drain(hub, after, tab, turn, finished)
}

/// How long the thread waits for an action before checking the stack is still there.
const POLL: Duration = Duration::from_millis(250);

/// How long a turn that recorded an after-turn action waits for its receipt on the relay.
pub(crate) const RECEIPT_WITHIN: Duration = Duration::from_secs(2);

/// An app action of an ended turn, for the after-turn thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Queued {
    /// The tab whose turn recorded it: its thread gets the notice of a refusal.
    pub tab: String,
    /// The turn that recorded it.
    pub turn: u32,
    /// What to do.
    pub action: AppAction,
}

/// What the after-turn thread asks the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppAction {
    /// Quit, unless a tab gained unsaved work.
    Quit,
    /// Install the update and restart.
    Install,
    /// Announce the agent's own close decision, unless a covered tab changed.
    CloseOwn { decision: Uuid },
}

/// Where a tab's actor sends the app actions of its ended turns.
pub(crate) type AfterTx = Sender<Queued>;

/// Takes what turn `turn` of `tab` recorded; `finished`: it ended on its own (no stop, no failure).
///
/// Returns whether the conversation starts over. Quitting and installing go to `after`; without
/// it (an actor outside the stack) they are logged and dropped. An unfinished turn drops it all.
pub(crate) fn drain(hub: &SurfaceHub, after: Option<&AfterTx>, tab: &str, turn: u32, finished: bool) -> bool {
    if !finished {
        // A stopped turn's own close never runs: its decision goes too.
        for action in hub.take_after_turn(turn) {
            if let AfterTurn::CloseOwn { decision } = action {
                hub.drop_close(decision);
            }
        }
        hub.drop_after_turn();
        return false;
    }
    let mut new_conversation = false;
    for action in hub.take_after_turn(turn) {
        let action = match action {
            AfterTurn::NewConversation => {
                new_conversation = true;
                continue;
            }
            AfterTurn::Quit => AppAction::Quit,
            AfterTurn::Install => AppAction::Install,
            AfterTurn::CloseOwn { decision } => AppAction::CloseOwn { decision },
        };
        let queued = Queued { tab: tab.to_owned(), turn, action };
        match after {
            Some(tx) if tx.send(queued).is_ok() => tracing::debug!(tab, turn, ?action, "after turn: queued"),
            Some(_) => tracing::warn!(tab, turn, ?action, "after turn: the thread is gone, dropped"),
            None => tracing::warn!(tab, turn, ?action, "after turn: no thread outside the stack, dropped"),
        }
    }
    new_conversation
}

/// How the after-turn thread ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerEnd {
    /// The stack is gone or closing.
    Closed,
    /// Every sender is gone.
    Disconnected,
    /// A step panicked; the panic was logged.
    Panicked,
}

/// Feeds `step` each queued action (`None` after each idle [`POLL`]) until it returns false.
///
/// A panic in `step` is caught and logged: the thread ends cleanly.
pub(crate) fn run(rx: &Receiver<Queued>, mut step: impl FnMut(Option<Queued>) -> bool) -> RunnerEnd {
    let looped = std::panic::catch_unwind(AssertUnwindSafe(|| {
        loop {
            let next = match rx.recv_timeout(POLL) {
                Ok(q) => Some(q),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return RunnerEnd::Disconnected,
            };
            if !step(next) {
                return RunnerEnd::Closed;
            }
        }
    }));
    looped.unwrap_or_else(|_| {
        tracing::error!("after-turn thread panicked: no more app actions run after a turn");
        RunnerEnd::Panicked
    })
}

/// Starts the after-turn thread over `rx` for `stack`, held weakly.
pub(crate) fn spawn(stack: Weak<Stack>, rx: Receiver<Queued>) -> std::io::Result<JoinHandle<RunnerEnd>> {
    std::thread::Builder::new().name("after-turn".into()).spawn(move || {
        run(&rx, |q| {
            let Some(stack) = stack.upgrade() else { return false };
            if stack.is_closing() {
                return false;
            }
            if let Some(q) = q {
                stack.run_after_turn(&q);
            }
            true
        })
    })
}

impl Stack {
    /// Starts the after-turn thread over `rx`; logged if it cannot start.
    pub(crate) fn run_after_turns(self: &Arc<Self>, rx: Receiver<Queued>) {
        // The thread ends by itself once the stack closes: never joined.
        if let Err(e) = spawn(Arc::downgrade(self), rx) {
            tracing::warn!(error = %e, "after-turn thread not started: no quit or install after a turn");
        }
    }

    /// Runs `q` on this thread; a refusal or a dropped quit shows as a notice in its turn.
    pub(crate) fn run_after_turn(&self, q: &Queued) {
        let l = self.platform.live_localizer.current();
        let refused = match q.action {
            AppAction::Quit => match self.workspace.unsaved_tabs() {
                0 => self.app.quit().err().map(|e| e.localize(&l)),
                unsaved => {
                    tracing::info!(tab = %q.tab, turn = q.turn, unsaved, "after turn: quit dropped, unsaved work");
                    Some(l.tr("agent-after-turn-quit-unsaved"))
                }
            },
            AppAction::Install => self.app.install().err().map(|e| e.localize(&l)),
            AppAction::CloseOwn { decision } => self.close_own(&q.tab, decision).err(),
        };
        tracing::debug!(tab = %q.tab, turn = q.turn, action = ?q.action, refused = refused.is_some(), "after turn: ran");
        if let (Some(text), Some(t)) = (refused, self.tab(&q.tab)) {
            t.hub.after_turn_notice(q.turn, &text);
        }
    }
}

impl Stack {
    /// Announces `tab`'s own close decision if its tabs' dirty revisions are still those confirmed.
    ///
    /// Otherwise the decision is dropped: `Err` holds the localized notice for the turn.
    fn close_own(&self, tab: &str, decision: Uuid) -> Result<(), String> {
        let l = self.platform.live_localizer.current();
        // A closed tab took its hub and decision along: nothing left to close.
        let Some(t) = self.tab(tab) else { return Ok(()) };
        let Some((target, stored)) = t.hub.close_decision(decision) else {
            // Forgotten meanwhile (evicted): nothing changed in the tab, nothing to tell the user.
            tracing::warn!(tab, "after turn: own close decision gone, not closed");
            return Ok(());
        };
        // Only the shown tab's page receives the close: a tab put behind another stays open.
        if !t.session.shown() {
            tracing::info!(tab, "after turn: close dropped, the tab is no longer shown");
            t.hub.drop_close(decision);
            return Err(l.tr("agent-after-turn-close-not-shown"));
        }
        let scope = match &target {
            CloseTarget::Tab { tab: target } => CloseScope::Tab(target),
            CloseTarget::Window { .. } => CloseScope::Window,
        };
        let live = self
            .workspace
            .dirty_revisions(tab, scope)
            .map(|covered| covered.tabs.into_iter().map(|t| (t.tab, t.revision)).collect::<Vec<_>>());
        match live {
            // Clean now (saved since) loses nothing; still dirty, only at the revision confirmed.
            Ok(live) if live.iter().all(|now| now.1.is_none() || stored.revisions.contains(now)) => {
                if !t.hub.send_close(decision) {
                    tracing::warn!(tab, "after turn: own close decision gone, not closed");
                }
                Ok(())
            }
            Ok(_) => {
                tracing::info!(tab, "after turn: close dropped, a covered tab changed");
                t.hub.drop_close(decision);
                Err(l.tr("agent-after-turn-close-unsaved"))
            }
            Err(e) => {
                t.hub.drop_close(decision);
                Err(e.localize(&l))
            }
        }
    }
}

#[cfg(test)]
#[path = "after_turn_tests.rs"]
mod after_turn_tests;
