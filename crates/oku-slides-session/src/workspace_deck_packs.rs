//! Deck-pack tabs in the workspace: links pushed, saves forwarded, tabs opened.
//!
//! A deck-only pack opens in a deck-pack tab right after its owner deck
//! (`open_deck_pack`); the layout (`TabDocument::DeckPack`) is the link's only
//! persisted form, and every session acquisition hands a deck tab its links
//! (`acquire`, `deck_links_of`). A public call on a deck-pack tab then forwards
//! its due save to the owner (`forward`): the tab's session, the owner's, then
//! the tab's again, one at a time, never under the workspace lock. Owners a
//! forward changed wait in `touched` for the hosts (`drain_touched`).

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::agent::PackGate;
use crate::deck_pack_link::LinkSpec;
use crate::deck_pack_receive::{Received, settle_answer};
use crate::deck_pack_tab::DeckPackSend;
use crate::error::{Result, UnknownTabSnafu};
use crate::pack::PackManifest;
use crate::{Session, TabDocument};

use super::deck_packs_close::PendingLink;
use super::packs::{Ask, OpenCtx, PackOpened};
use super::{Opening, Tab, TabPlace, Workspace, relock};

/// Whether an acquisition forwards a due deck-pack save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Forward {
    /// Public calls: forward, then pull.
    Yes,
    /// Workspace-internal calls and the forward's own steps.
    No,
}

/// A test hook run by `forward` right before it waits for the owner's session.
///
/// Nextest only: one process per test, so no other test sees it set.
#[cfg(test)]
pub(super) static BEFORE_OWNER_LOCK: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(None);

/// What one acquisition took from the session besides `f`'s result.
#[derive(Debug, Default)]
struct Taken {
    /// The deck-pack tab's due send.
    send: Option<Arc<DeckPackSend>>,
    /// The owner's pull of its links' parked sends.
    pull: bool,
    /// The tabs whose link the owner refused.
    refused: Vec<String>,
}

impl Workspace {
    /// The one session acquisition: links pushed, `f`, then the forward if asked.
    ///
    /// Under the workspace lock: the tab's session, its links and the layout
    /// generation; under the session's alone: the links handed over, `f`, and
    /// for `Forward::Yes` the due send and the owner's pull. Then, both locks
    /// released: refused links set aside, the info refreshed, the send
    /// forwarded and the pull run. `true`: the tab's info changed, through the
    /// forward's acknowledgement too.
    pub(super) fn acquire<R>(
        ws: &Mutex<Self>,
        tab: &str,
        forward: Forward,
        f: impl FnOnce(&mut Session) -> R,
    ) -> (Result<R>, bool) {
        let (session, closed, links, generation) = {
            let w = Self::lock(ws);
            match w.session(tab) {
                Ok(session) => (session, Arc::clone(&w.closed), w.deck_links_of(tab), w.layout_generation),
                Err(e) => return (Err(e), false),
            }
        };
        let (out, info, taken) = {
            let mut slot = relock(&session);
            // Read under the session's lock: `settle` sets `closed` before
            // flushing each session under this same lock; seen false here,
            // `f`'s change is flushed afterwards.
            if closed.load(Ordering::SeqCst) {
                return (Err(UnknownTabSnafu { id: tab }.build().into()), false);
            }
            match slot.ready_with_links(generation, &links) {
                Ok((s, refused)) => {
                    let out = f(s);
                    let (send, pull) = match forward {
                        Forward::Yes => (s.take_deck_pack_send(), s.take_link_pull()),
                        Forward::No => (None, false),
                    };
                    (Ok(out), Some(s.tab_info()), Taken { send, pull, refused })
                }
                Err(e) => (Err(e), None, Taken::default()),
            }
        };
        Self::set_aside(ws, &taken.refused);
        let mut changed = info.is_some_and(|info| Self::refresh(ws, tab, info));
        if let Some(send) = taken.send {
            let cached = |ws: &Mutex<Self>| Self::lock(ws).tabs.get(tab).map(|t| t.info.clone());
            let before = cached(ws);
            Self::forward(ws, tab, &send);
            changed |= cached(ws) != before;
        }
        if taken.pull {
            Self::pull_linked(ws, tab);
        }
        (out, changed)
    }

    /// `f` on a session with its links, without forwarding; `true`: its info changed.
    ///
    /// For workspace-internal calls: a forward never runs under `OPENING`
    /// and never loses its owner. Refused after `settle`, like `with_session`.
    pub(super) fn with_quiet<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool) {
        Self::acquire(ws, tab, Forward::No, f)
    }

    /// The deck tabs a forward changed since the last call; drains them.
    #[must_use]
    pub fn take_touched(ws: &Mutex<Self>) -> Vec<String> {
        std::mem::take(&mut Self::lock(ws).touched)
    }

    /// Drains touched owners: `emit` once if any, then `after` per owner.
    ///
    /// The one host step after a call: `emit` redraws the views, `after`
    /// tells the agent the owner's deck changed. Runs with no lock held.
    pub fn drain_touched(ws: &Mutex<Self>, emit: impl FnOnce(), mut after: impl FnMut(&str)) {
        let owners = Self::take_touched(ws);
        if owners.is_empty() {
            return;
        }
        emit();
        for owner in &owners {
            after(owner);
        }
    }

    /// The deck-pack tabs whose owner is `deck_tab`, in window order.
    pub(super) fn linked_tabs(&self, deck_tab: &str) -> Vec<String> {
        let linked = |id: &&String| self.tabs.get(*id).is_some_and(|t| t.info.document.deck_tab() == Some(deck_tab));
        self.layout.windows.iter().flat_map(|w| &w.tabs).filter(linked).cloned().collect()
    }

    /// The links of `tab` as the layout names them.
    pub(super) fn deck_links_of(&self, tab: &str) -> Vec<LinkSpec> {
        self.linked_tabs(tab)
            .into_iter()
            .filter_map(|id| {
                let t = self.tabs.get(&id)?;
                let TabDocument::DeckPack { pack, .. } = t.info.document else { return None };
                Some(LinkSpec { title: t.info.title.clone(), tab: id, pack })
            })
            .collect()
    }

    /// Hands `tab`'s taken send to its owner, then settles the answer.
    ///
    /// The owner's session is awaited like any request; an owner that is
    /// gone answers `UnknownTab`, which the tab notices. An applied or
    /// delivered copy retargets the tab's cached document in this same call,
    /// and an applied send puts the owner in `touched`. `true`: the owner applied it.
    pub(super) fn forward(ws: &Mutex<Self>, tab: &str, send: &DeckPackSend) -> bool {
        let owner = Self::lock(ws).tabs.get(tab).and_then(|t| t.info.document.deck_tab().map(str::to_owned));
        #[cfg(test)]
        {
            // Cloned out first: the hook never runs with the static's mutex held.
            let hook = BEFORE_OWNER_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
            if let Some(hook) = hook {
                hook();
            }
        }
        let answer = match &owner {
            Some(owner) => Self::with_quiet(ws, owner, |o| o.receive_deck_pack(tab, send)).0.and_then(|r| r),
            None => Err(UnknownTabSnafu { id: tab }.build().into()),
        };
        if let Err(e) = Self::with_quiet(ws, tab, |s| settle_answer(s, send.seq, &answer)).0 {
            tracing::warn!(tab, error = %e.one_line(), "deck-pack answer not settled: the tab stays unsent");
        }
        let applied = matches!(answer, Ok(Received::Applied));
        // `Delivered` too: an applied copy whose acknowledgement was lost is retargeted on its resend.
        if !matches!(answer, Ok(Received::Applied | Received::Delivered)) {
            return false;
        }
        let manifest = PackManifest::from_files(&send.files).ok().map(|m| m.id);
        let mut w = Self::lock(ws);
        if let Some(copy) = manifest.filter(|id| *id != send.base.pack) {
            w.retarget_deck_pack(tab, copy);
        }
        if let Some(owner) = owner.filter(|o| applied && !w.touched.contains(o)) {
            w.touched.push(owner);
        }
        applied
    }

    /// Forwards the parked sends of `owner`'s linked tabs.
    pub(super) fn pull_linked(ws: &Mutex<Self>, owner: &str) {
        let linked = Self::lock(ws).linked_tabs(owner);
        for tab in linked {
            if let Err(e) = Self::acquire(ws, &tab, Forward::Yes, Session::unpark_deck_pack).0 {
                tracing::warn!(tab = %tab, error = %e.one_line(), "parked deck-pack send not pulled");
            }
        }
    }

    /// Opens or focuses `pack`'s deck-pack tab beside its owner `ctx.tab`.
    ///
    /// The agent's gate is asked `NewTab` first. The owner records a pending
    /// link and gives the seed; the tab is built outside the workspace lock,
    /// then inserted right after its owner, the owner as its opener. A
    /// failed build or insert unlinks it again, the lock released.
    ///
    /// # Errors
    ///
    /// The `link_deck_pack` refusals, the build's and the insert's errors.
    ///
    /// # Blocking
    ///
    /// Builds and renders a session.
    pub(super) fn open_deck_pack(
        ctx: &OpenCtx<'_>,
        window: String,
        name: &str,
        ask: &mut Ask<'_>,
    ) -> Result<PackOpened> {
        if !ask(PackGate::NewTab) {
            return Ok(PackOpened::Refused);
        }
        let pending = {
            let mut w = Self::lock(ctx.ws);
            let opening = Opening::DeckPack {
                deck_tab: ctx.tab.to_owned(),
                pack: ctx.pack,
                widget: ctx.widget.map(str::to_owned),
            };
            let place =
                if w.tab_window(ctx.tab).is_some() { TabPlace::After(ctx.tab.to_owned()) } else { TabPlace::End };
            let window = w.tab_window(ctx.tab).map_or(window, str::to_owned);
            w.pending(Some(window), opening, place)
        };
        let (seed, _changed) = Self::with_quiet(ctx.ws, ctx.tab, |s| s.link_deck_pack(&pending.id, name, ctx.pack));
        let seed = seed??;
        // Armed before any workspace guard: every error path below cancels it, unlocked.
        let link = PendingLink::new(ctx.ws, ctx.tab, &pending.id);
        let session = match pending.build_deck_pack(seed) {
            Ok(session) => session,
            Err(e) => {
                link.cancel();
                return Err(e);
            }
        };
        let id = pending.id.clone();
        let finished = Self::finish(ctx.ws, pending, session);
        let inserted = Self::lock(ctx.ws).tabs.contains_key(&id);
        match finished {
            Ok((_, id)) => {
                link.disarm();
                Ok(PackOpened::Opened(id))
            }
            // Once inserted, the tab exists: its link stays.
            Err(e) if inserted => {
                link.disarm();
                tracing::warn!(tab = %id, error = %e.one_line(), "deck-pack tab opened, its view not built");
                Ok(PackOpened::Opened(id))
            }
            Err(e) => {
                link.cancel();
                Err(e)
            }
        }
    }

    /// Points a deck-pack tab's cached document at the accepted copy, then saves the layout.
    pub(super) fn retarget_deck_pack(&mut self, tab: &str, pack: Uuid) {
        let Some(t) = self.tabs.get_mut(tab) else { return };
        let TabDocument::DeckPack { pack: cached, .. } = &mut t.info.document else { return };
        *cached = pack;
        self.save_layout();
    }

    /// Closes the tabs whose link the owner refused, keeping their folders.
    ///
    /// Each one leaves its window (whose `active` stays valid) and stays
    /// cited in `kept`, its folder never deleted; returns their slots, whose
    /// resume the caller stops once the lock is released.
    pub(super) fn set_aside_unlinked(&mut self, tabs: &[String]) -> Vec<(String, Tab)> {
        let mut gone = Vec::new();
        for id in tabs {
            if let Err(e) = self.detach(id) {
                tracing::warn!(tab = %id, error = %e.one_line(), "deck-pack tab to set aside not found");
                continue;
            }
            gone.extend(self.remove_tab(id));
            if !self.layout.kept.contains(id) {
                self.layout.kept.push(id.clone());
            }
            tracing::warn!(tab = %id, "deck-pack tab set aside: its owner deck refused its link");
        }
        if !gone.is_empty() {
            self.save_layout();
        }
        gone
    }

    /// Whether `info` shows that deck-pack tab `tab` lost its document (resume failed).
    pub(super) fn lost_link(&self, tab: &str, info: &crate::TabInfo) -> bool {
        let cached = self.tabs.get(tab).map(|t| &t.info.document);
        matches!(cached, Some(TabDocument::DeckPack { .. })) && !matches!(info.document, TabDocument::DeckPack { .. })
    }
}

#[cfg(test)]
#[path = "workspace_deck_packs_tests.rs"]
mod workspace_deck_packs_tests;

#[cfg(test)]
#[path = "workspace_deck_packs_resume_tests.rs"]
mod workspace_deck_packs_resume_tests;
