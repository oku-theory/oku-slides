//! The agent's close decisions: stored once the user confirmed, redeemed once by the page.
//!
//! A `tab close` or `window close` the user confirmed is kept here under a fresh UUID v4 with
//! its target, its window and each covered tab's dirty revision, then announced
//! ([`McpEvent::Close`]). The page closes through its own flows (saves, guards) and redeems the
//! id ([`SurfaceHub::take_close_decision`]) only to answer its discard question, and only while
//! each revision still matches. A decision is taken once, by its own window, within
//! [`CLOSE_DECISION_TTL`] of its announcement; the agent's own tab waits for the turn's end
//! (`AfterTurn::CloseOwn`), unannounced and unredeemable until then.

use std::time::{Duration, Instant};

use serde::Serialize;
use uuid::Uuid;

use super::{McpEvent, SurfaceHub};

/// How long an announced decision waits for its page.
pub const CLOSE_DECISION_TTL: Duration = Duration::from_secs(30);

/// Decisions a hub keeps at most: the oldest goes first.
const KEPT: usize = 8;

/// What an agent's close covers, as the page reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CloseTarget {
    /// One tab, closed through the page's tab flow.
    Tab {
        /// The tab's id, `t-<n>`.
        tab: String,
    },
    /// A window, closed through the page's window flow.
    Window {
        /// The window's label, `w-<n>`.
        window: String,
    },
}

/// A hub decision a page redeems once to answer its own discard question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CloseDecision {
    /// The user confirmed losing what cannot be saved.
    pub discard: bool,
    /// Each covered tab and its dirty revision when confirmed; `None`: only while still clean.
    pub revisions: Vec<(String, Option<u64>)>,
}

/// A stored decision, with what redeeming it checks.
#[derive(Debug)]
pub(super) struct StoredClose {
    id: Uuid,
    target: CloseTarget,
    /// The window whose page may redeem it.
    window: String,
    decision: CloseDecision,
    /// `None` until announced: a pending decision never expires and cannot be redeemed.
    expires: Option<Instant>,
}

impl SurfaceHub {
    /// Stores `decision` for `target` in `window`; announced later by [`Self::send_close`].
    ///
    /// Past [`KEPT`] decisions, the oldest sent one is forgotten, a pending one (an own close
    /// waiting for its turn's end) only when none was sent.
    pub fn store_close(&self, target: CloseTarget, window: &str, decision: CloseDecision) -> Uuid {
        let id = Uuid::new_v4();
        let mut state = self.state();
        let now = Instant::now();
        state.closes.retain(|c| c.expires.is_none_or(|at| at > now));
        if state.closes.len() >= KEPT {
            let oldest = state.closes.iter().position(|c| c.expires.is_some()).unwrap_or(0);
            state.closes.remove(oldest);
        }
        state.closes.push(StoredClose { id, target, window: window.to_owned(), decision, expires: None });
        id
    }

    /// Announces decision `id` to the page (`McpEvent::Close`); `false` when it is gone.
    ///
    /// Its [`CLOSE_DECISION_TTL`] starts now.
    pub fn send_close(&self, id: Uuid) -> bool {
        self.send_close_at(id, Instant::now())
    }

    /// [`Self::send_close`] at `now`.
    pub(crate) fn send_close_at(&self, id: Uuid, now: Instant) -> bool {
        let mut state = self.state();
        let Some(stored) = state.closes.iter_mut().find(|c| c.id == id) else { return false };
        let Some(expires) = now.checked_add(CLOSE_DECISION_TTL) else { return false };
        stored.expires = Some(expires);
        let target = stored.target.clone();
        self.record_and_send(&mut state, McpEvent::Close { target, decision: id.to_string() });
        true
    }

    /// Takes decision `id` once, for a page of `window`; `None` when unknown, pending or expired.
    ///
    /// Another window's page gets `None` and leaves it in place.
    pub fn take_close_decision(&self, window: &str, id: &str) -> Option<CloseDecision> {
        self.take_close_decision_at(window, id, Instant::now())
    }

    /// [`Self::take_close_decision`] at `now`.
    pub(crate) fn take_close_decision_at(&self, window: &str, id: &str, now: Instant) -> Option<CloseDecision> {
        let id = Uuid::try_parse(id).ok()?;
        let mut state = self.state();
        let at = state.closes.iter().position(|c| c.id == id)?;
        let stored = &state.closes[at];
        // Another window's page, or a decision not sent yet (an own close waiting for its turn's
        // end): left in place, so an early take cannot cancel it.
        let expires = stored.expires.filter(|_| stored.window == window)?;
        let live = expires > now;
        let taken = state.closes.remove(at);
        live.then_some(taken.decision)
    }

    /// The target and decision stored under `id`, for the after-turn re-check; kept in place.
    #[must_use]
    pub fn close_decision(&self, id: Uuid) -> Option<(CloseTarget, CloseDecision)> {
        self.state().closes.iter().find(|c| c.id == id).map(|c| (c.target.clone(), c.decision.clone()))
    }

    /// Forgets decision `id`: denied, refused after the turn, or gone stale.
    pub fn drop_close(&self, id: Uuid) {
        self.state().closes.retain(|c| c.id != id);
    }
}

#[cfg(test)]
#[path = "close_tests.rs"]
mod close_tests;
