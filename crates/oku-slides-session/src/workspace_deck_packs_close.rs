//! Closing and quitting with deck-pack tabs: flushed first, closed with their owner.
//!
//! A deck-pack tab's unsaved or unsent work reaches its owner before a close
//! decides anything: the pre-phase, outside the workspace lock, flushes the
//! tabs the close takes through the public forward path (`flush_linked`).
//! Under the lock an owner closes with its linked tabs, each checked by
//! `ensure_closable`; a close whose linked tabs would empty the workspace
//! closes nothing (`Closed::LastWindow`), while a plain last tab closes as
//! before. A quit flushes every awake deck-pack tab, except those a refused
//! close discarded (`flush_all_deck_packs`), whose unsent saves are dropped
//! (`drop_unsent`). A link pending while its tab is built is cancelled
//! explicitly (`PendingLink`).

use std::sync::{Mutex, TryLockError};

use snafu::OptionExt;

use crate::Session;
use crate::error::{Result, UnknownTabSnafu};

use super::deck_packs::Forward;
use super::{Slot, Tab, Workspace, WorkspaceOutcome};

/// Counts the quit flushes that got past the closed check (tests only).
///
/// Nextest only: one process per test, so no other test moves it.
#[cfg(test)]
pub(super) static FLUSH_ALL_SCANS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// What closing a tab with its links did.
pub(super) enum Closed {
    /// These tabs left the layout; their slots go to the trash.
    Gone(Vec<(String, Tab)>),
    /// Every tab, linked ones included, would have closed: nothing was.
    ///
    /// Carries the linked tabs a discard recorded, whose unsent saves the caller drops.
    LastWindow(Vec<String>),
}

impl std::fmt::Debug for Closed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gone(gone) => {
                f.debug_tuple("Gone").field(&gone.iter().map(|(id, _)| id).collect::<Vec<_>>()).finish()
            }
            Self::LastWindow(discarded) => f.debug_tuple("LastWindow").field(discarded).finish(),
        }
    }
}

/// A pending deck-pack link to cancel unless the tab reaches the layout.
#[derive(Debug)]
pub(super) struct PendingLink<'a> {
    /// The workspace to reach the owner through.
    ws: &'a Mutex<Workspace>,
    /// The owner deck's tab.
    owner: String,
    /// The tab being opened.
    tab: String,
    /// Cleared by `disarm` (in the layout) or `cancel` (unlinked).
    armed: bool,
}

impl<'a> PendingLink<'a> {
    /// Arms the link of `tab` that `owner` just recorded.
    pub(super) fn new(ws: &'a Mutex<Workspace>, owner: &str, tab: &str) -> Self {
        Self { ws, owner: owner.to_owned(), tab: tab.to_owned(), armed: true }
    }

    /// The tab is in the layout: nothing to undo.
    pub(super) fn disarm(mut self) {
        self.armed = false;
    }

    /// Unlinks the tab from its owner; call with no workspace guard held.
    pub(super) fn cancel(mut self) {
        self.armed = false;
        let (ws, owner, tab) = (self.ws, self.owner.clone(), self.tab.clone());
        if let Err(e) = Workspace::with_quiet(ws, &owner, |s| s.unlink_deck_pack(&tab)).0 {
            tracing::warn!(tab = %owner, error = %e.one_line(), "pending deck-pack link not cancelled");
        }
    }
}

impl Drop for PendingLink<'_> {
    /// Only a panic net: never blocks, never waits for the workspace.
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let session = match self.ws.try_lock() {
            Ok(w) => w.session(&self.owner).ok(),
            Err(TryLockError::Poisoned(p)) => p.into_inner().session(&self.owner).ok(),
            Err(TryLockError::WouldBlock) => None,
        };
        let unlinked = session.is_some_and(|session| match session.try_lock() {
            Ok(mut slot) => unlink_ready(&mut slot, &self.tab),
            Err(TryLockError::Poisoned(p)) => unlink_ready(&mut p.into_inner(), &self.tab),
            Err(TryLockError::WouldBlock) => false,
        });
        if !unlinked {
            tracing::warn!(owner = %self.owner, tab = %self.tab, "pending deck-pack link left: its owner is busy");
        }
    }
}

/// Unlinks `tab` from a ready owner; false when it is not ready.
fn unlink_ready(slot: &mut Slot, tab: &str) -> bool {
    match slot {
        Slot::Ready(owner) => {
            owner.unlink_deck_pack(tab);
            true
        }
        Slot::Dormant(_) | Slot::Closed(_) => false,
    }
}

impl Workspace {
    /// Flushes and forwards every tab linked to `deck_tab`, waking them.
    pub(super) fn flush_linked(ws: &Mutex<Self>, deck_tab: &str) {
        let linked = Self::lock(ws).linked_tabs(deck_tab);
        for tab in linked {
            Self::flush_deck_pack_tab(ws, &tab);
        }
    }

    /// Flushes deck-pack tab `tab` and forwards its save; `true`: the owner applied it.
    ///
    /// The forward path of a public call, run here so its answer is known; a
    /// deck-pack tab owns no links, so there is no pull to run. A failure is logged.
    fn flush_deck_pack_tab(ws: &Mutex<Self>, tab: &str) -> bool {
        let flushed = |s: &mut Session| s.flush_deck_pack().map(|()| s.take_deck_pack_send());
        match Self::acquire(ws, tab, Forward::No, flushed).0 {
            Ok(Ok(Some(send))) => Self::forward(ws, tab, &send),
            Ok(Ok(None)) => false,
            Ok(Err(e)) | Err(e) => {
                tracing::warn!(tab, error = %e.one_line(), "deck-pack tab not flushed");
                false
            }
        }
    }

    /// Flushes every awake deck-pack tab not discarded; no-op once closed.
    pub(super) fn flush_all_deck_packs(ws: &Mutex<Self>) {
        let tabs: Vec<String> = {
            let w = Self::lock(ws);
            if w.is_closed() {
                return;
            }
            #[cfg(test)]
            FLUSH_ALL_SCANS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let awake = |t: &Tab| match t.session.try_lock() {
                Ok(slot) => !slot.is_dormant(),
                Err(TryLockError::Poisoned(p)) => !p.into_inner().is_dormant(),
                Err(TryLockError::WouldBlock) => true,
            };
            w.tab_ids()
                .into_iter()
                .filter(|id| !w.discard_at_quit.contains(id))
                .filter(|id| w.tabs.get(id).is_some_and(|t| t.info.document.deck_tab().is_some() && awake(t)))
                .collect()
        };
        for tab in tabs {
            Self::flush_deck_pack_tab(ws, &tab);
        }
    }

    /// Closes `tab` with its linked tabs, or keeps all when they empty everything.
    ///
    /// Each closed tab passes `ensure_closable` (dirty or unsent refuses
    /// unless `discard`); the owner is detached first, handing the focus to
    /// its opener. A tab with no linked tab closes even when it is the last
    /// one. A refused last close with `discard` records the linked tabs as
    /// discarded at quit, until each one closes, and returns them.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, and `Unsaved` for a tab that is not closable.
    pub(super) fn close_with_links(&mut self, tab: &str, discard: bool) -> Result<Closed> {
        self.tabs.get(tab).context(UnknownTabSnafu { id: tab })?;
        let linked = self.linked_tabs(tab);
        let closing: Vec<&str> = std::iter::once(tab).chain(linked.iter().map(String::as_str)).collect();
        // A plain last tab closes as before (the host quits on the empty view).
        if !linked.is_empty() && self.tab_ids().iter().all(|id| closing.contains(&id.as_str())) {
            let discarded = if discard { linked } else { Vec::new() };
            self.discard_at_quit.extend(discarded.iter().cloned());
            return Ok(Closed::LastWindow(discarded));
        }
        for t in &closing {
            self.ensure_closable(t, discard)?;
        }
        self.detach_to_opener(tab)?;
        let mut gone: Vec<(String, Tab)> = self.remove_tab(tab).into_iter().collect();
        for t in linked {
            // The owner is already out: a failure here must not stop half-way.
            if let Err(e) = self.detach(&t) {
                tracing::warn!(tab = %t, error = %e.one_line(), "linked deck-pack tab not detached");
            }
            gone.extend(self.remove_tab(&t));
        }
        Ok(Closed::Gone(gone))
    }

    /// `apply` on `Close`: the pre-phase's flush, then the close under the lock.
    ///
    /// A deck-pack tab closed alone is flushed, removed, unlinked from its
    /// owner (lock released), and gives its owner the focus; the outcome is
    /// `DeckChanged` for the owner when the flush reached it.
    pub(super) fn close_tab(ws: &Mutex<Self>, tab: &str, discard: bool) -> Result<WorkspaceOutcome> {
        let owner = Self::lock(ws).tabs.get(tab).and_then(|t| t.info.document.deck_tab().map(str::to_owned));
        let reached = match (&owner, discard) {
            (_, true) => false,
            (Some(_), false) => Self::flush_deck_pack_tab(ws, tab),
            (None, false) => {
                Self::flush_linked(ws, tab);
                Self::save_pending_usages(ws, &[tab.to_owned()]);
                false
            }
        };
        let mut w = Self::lock(ws);
        let window = w.window_of(tab).ok().map(|i| w.layout.windows[i].label.clone());
        let was_active = w.tab_shown(tab);
        let gone = match w.close_with_links(tab, discard)? {
            Closed::LastWindow(discarded) => {
                let json = w.view_json();
                drop(w);
                Self::drop_unsent(ws, &discarded);
                return json.map(WorkspaceOutcome::LastWindow);
            }
            Closed::Gone(gone) => gone,
        };
        if let Some(owner) = owner.as_deref().filter(|_| was_active)
            && let Some(win) = w.layout.windows.iter_mut().find(|win| Some(&win.label) == window.as_ref())
            && win.tabs.iter().any(|t| t == owner)
        {
            owner.clone_into(&mut win.active);
        }
        w.save_layout();
        let (json, trash) = (w.view_json(), w.trash());
        drop(w);
        trash.empty(gone);
        let Some(owner) = owner else { return json.map(WorkspaceOutcome::Applied) };
        if let Err(e) = Self::with_quiet(ws, &owner, |s| s.unlink_deck_pack(tab)).0 {
            tracing::warn!(tab = %owner, error = %e.one_line(), "closed deck-pack tab not unlinked");
        }
        let json = json?;
        Ok(if reached { WorkspaceOutcome::DeckChanged { json, tab: owner } } else { WorkspaceOutcome::Applied(json) })
    }

    /// Every tab `CloseWindow` would close, and those of them discarded.
    ///
    /// The first list is the window's own tabs plus the tabs linked to its decks, in other
    /// windows too. The second is `discard` among them, plus each tab linked
    /// to a discarded owner; the caller flushes the others first.
    pub(super) fn window_closing(&self, window: &str, discard: &[String]) -> (Vec<String>, Vec<String>) {
        let own = self.layout.windows.iter().find(|w| w.label == window).map(|w| w.tabs.clone()).unwrap_or_default();
        let (mut closing, mut discarded) = (own.clone(), discard.to_vec());
        for owner in &own {
            for linked in self.linked_tabs(owner) {
                if discard.contains(owner) && !discarded.contains(&linked) {
                    discarded.push(linked.clone());
                }
                if !closing.contains(&linked) {
                    closing.push(linked);
                }
            }
        }
        discarded.retain(|t| closing.contains(t));
        (closing, discarded)
    }

    /// Flushes the deck-pack tabs among `closing` that are not `discarded`.
    pub(super) fn flush_closing(ws: &Mutex<Self>, closing: &[String], discarded: &[String]) {
        let flushed: Vec<String> = {
            let w = Self::lock(ws);
            let deck_pack = |id: &String| w.tabs.get(id).is_some_and(|t| t.info.document.deck_tab().is_some());
            closing.iter().filter(|id| !discarded.contains(id) && deck_pack(id)).cloned().collect()
        };
        for tab in flushed {
            Self::flush_deck_pack_tab(ws, &tab);
        }
    }

    /// Drops the unsent saves of tabs a refused last close discarded, lock released.
    ///
    /// Their state is written without them, so a resume never resends them.
    pub(super) fn drop_unsent(ws: &Mutex<Self>, discarded: &[String]) {
        for tab in discarded {
            if let Err(e) = Self::with_quiet(ws, tab, Session::discard_unsent).0 {
                tracing::warn!(tab = %tab, error = %e.one_line(), "discarded deck-pack save not dropped");
            }
        }
    }

    /// Unlinks closed deck-pack tabs from their surviving owners, lock released.
    pub(super) fn unlink_closed(ws: &Mutex<Self>, unlinked: &[(String, String)]) {
        for (owner, tab) in unlinked {
            if let Err(e) = Self::with_quiet(ws, owner, |s| s.unlink_deck_pack(tab)).0 {
                tracing::warn!(tab = %owner, error = %e.one_line(), "closed deck-pack tab not unlinked");
            }
        }
    }
}

#[cfg(test)]
#[path = "workspace_deck_packs_close_tests.rs"]
mod workspace_deck_packs_close_tests;
