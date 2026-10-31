//! The workspace and agent usage: which deck tab counts a tab's turns.
//!
//! A deck tab counts its own; a deck-pack tab its owner deck; a pack or
//! theme tab the deck it was opened from (`Tab.opener`, by the user or an
//! agent; lost on restart), through any pack or theme tab between them; a
//! help tab none. A dormant deck queues the records until it wakes.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, TryLockError};

use oku_slides_core::UsageRecord;

use crate::DocumentKind;
use crate::agent::record_in;
use crate::error::{Result, UnknownTabSnafu};

use super::{Slot, Workspace, relock};

impl Workspace {
    /// The deck tab whose ledger takes `tab`'s agent usage; wakes nothing.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock alone.
    ///
    /// A pack or theme tab opened from another one leads on to that tab's
    /// deck; a chain longer than the tab count is a cycle: `None`.
    pub fn usage_owner(ws: &Mutex<Self>, tab: &str) -> Option<String> {
        let w = Self::lock(ws);
        let mut current = tab;
        for _ in 0..=w.tabs.len() {
            let t = w.tabs.get(current)?;
            match t.info.document.kind() {
                DocumentKind::Deck => return Some(current.to_owned()),
                DocumentKind::Pack | DocumentKind::Theme => {
                    current = t.info.document.deck_tab().or(t.opener.as_deref())?;
                }
                DocumentKind::Help => return None,
            }
        }
        None
    }

    /// Records a turn's usage in deck tab `owner`, waking nothing (t-1310).
    ///
    /// An awake deck takes it at once; a dormant one queues it until it
    /// wakes, or until [`Workspace::settle`] wakes it to keep it. Neither
    /// rendered nor handed its links: the ledger needs neither.
    ///
    /// # Errors
    ///
    /// `UnknownTab` for a tab unknown or closed, or after `settle`.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock, then the tab's session lock alone.
    pub fn record_usage(ws: &Mutex<Self>, owner: &str, record: UsageRecord) -> Result<()> {
        let (session, closed) = {
            let w = Self::lock(ws);
            (w.session(owner)?, Arc::clone(&w.closed))
        };
        let mut slot = relock(&session);
        // As `acquire`: once `settle` ran, no session writes again.
        if closed.load(Ordering::SeqCst) {
            return Err(UnknownTabSnafu { id: owner }.build().into());
        }
        match &mut *slot {
            Slot::Ready(s) => record_in(s, owner, record),
            Slot::Dormant(dormant) => dormant.usage.push(record),
            Slot::Closed(id) => return Err(UnknownTabSnafu { id: id.clone() }.build().into()),
        }
        Ok(())
    }

    /// Saves each awake clean deck of `tabs` that holds agent usage its file lacks.
    ///
    /// The close's pre-phase, lock released; never waits: a busy, poisoned
    /// or dormant session is skipped (the close's own check answers for it),
    /// and a failure is logged, never refuses the close.
    pub(super) fn save_pending_usages(ws: &Mutex<Self>, tabs: &[String]) {
        let (sessions, closed): (Vec<_>, _) = {
            let w = Self::lock(ws);
            let deck = |id: &String| w.tabs.get(id).is_some_and(|t| t.info.document.kind() == DocumentKind::Deck);
            let sessions = tabs.iter().filter(|id| deck(id)).filter_map(|id| Some((id.clone(), w.session(id).ok()?)));
            (sessions.collect(), Arc::clone(&w.closed))
        };
        for (tab, session) in sessions {
            match session.try_lock() {
                Ok(mut slot) => {
                    // As `acquire`: once `settle` ran, no session writes again.
                    if closed.load(Ordering::SeqCst) {
                        return;
                    }
                    if let Slot::Ready(s) = &mut *slot {
                        s.save_pending_usage();
                    }
                }
                Err(TryLockError::WouldBlock) => {
                    tracing::warn!(tab = %tab, "agent usage not saved on close: the session is busy");
                }
                Err(TryLockError::Poisoned(_)) => {
                    tracing::warn!(tab = %tab, "agent usage not saved on close: the session panicked");
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "workspace_usage_tests.rs"]
mod workspace_usage_tests;
