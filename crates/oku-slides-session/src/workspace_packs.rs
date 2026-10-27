//! Pack tabs in the workspace: opening a pack from a tab, and its opener.
//!
//! An open of pack P runs under P's opening lock (`with_opening`,
//! `workspace_opening.rs`): a second open of P waits, then finds the first
//! one's tab and focuses it; opens of other packs never wait on it. Under that lock, the workspace and each
//! session are taken in sequence, never nested: a pack tab of P already open
//! is focused (steps 1-2, `focus_existing`), else the requesting tab routes
//! the pack (`route_pack`): a deck-only workshop entered in place, or a new
//! pack tab right after it, leased before anything changes (steps 3-5,
//! `open_new`). An agent's gate is asked before any change, with only the
//! opening lock held; the user's open asks nothing and makes the requesting
//! deck the tab's opener, which a close hands the focus back to.

use std::sync::{Mutex, TryLockError};

use serde::Serialize;
use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::PackVersion;

use crate::SessionErrorKind;
use crate::agent::{PackGate, PackOpen};
use crate::deck_offer::DeckOffer;
use crate::pack_lease::PackLease;
use crate::pack_tab::{PackRoute, PackWrite};

use super::opening::with_opening;
#[cfg(test)]
use super::opening::{opening_entry, opening_waiters};
use super::{Opening, Result, Slot, TabPlace, UnknownTabSnafu, Workspace, WorkspaceOutcome};
use crate::TabDocument;

/// Dormant candidates woken at most by one open (O10): past them, the live re-scan decides.
const MAX_DORMANT_CANDIDATES: usize = 4;

/// Who opens a pack: the user (no limit, retargets) or an agent's gate.
pub(crate) enum Opener<'a> {
    /// The user, through `Workspace::apply`.
    Ui,
    /// An agent: its gate, asked before any change (one `Focus`, then one `NewTab`).
    Agent(&'a mut dyn FnMut(PackGate) -> bool),
}

impl std::fmt::Debug for Opener<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ui => "Ui",
            Self::Agent(_) => "Agent",
        })
    }
}

/// How opening a pack went, for `apply`.
#[derive(Debug)]
pub(crate) enum PackOpened {
    /// An open pack tab took focus.
    Focused(String),
    /// A new pack tab opened.
    Opened(String),
    /// The requesting tab edits it: workshop entered, or sub-tab opened.
    Here {
        /// The requesting tab.
        tab: String,
        /// True when a deck-only workshop was entered.
        entered: bool,
    },
    /// The gate refused.
    Refused,
}

/// The deck offered to take the version a pack tab wrote.
#[derive(Debug, Serialize)]
pub(crate) struct OpenerOffer {
    /// The opener deck's tab.
    pub tab: String,
    /// The opener deck's title and identity.
    #[serde(flatten)]
    pub deck: DeckOffer,
    /// The pack the tab edited.
    pub pack: Uuid,
    /// That pack's name, already a label.
    pub pack_name: String,
    /// The version it wrote.
    pub version: PackVersion,
}

/// One open of a pack, as the steps share it.
struct OpenCtx<'a> {
    /// The workspace.
    ws: &'a Mutex<Workspace>,
    /// The requesting tab.
    tab: &'a str,
    /// The pack to open.
    pack: Uuid,
    /// A widget whose sub-tab opens.
    widget: Option<&'a str>,
    /// The user's open: the requesting deck becomes the focused tab's opener.
    retarget: bool,
}

/// The gate of one open, asked at most once per kind.
type Ask<'a> = dyn FnMut(PackGate) -> bool + 'a;

impl Workspace {
    /// Opens `pack` from `tab` for an agent; says where it landed.
    ///
    /// # Errors
    ///
    /// As [`crate::agent::Sessions::open_pack`].
    ///
    /// # Blocking
    ///
    /// As [`crate::agent::Sessions::open_pack`].
    pub fn open_pack(
        ws: &Mutex<Self>,
        tab: &str,
        pack: Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(PackGate) -> bool,
    ) -> Result<PackOpen> {
        Ok(match Self::open_pack_with(ws, tab, pack, widget, Opener::Agent(gate))? {
            PackOpened::Here { .. } => PackOpen::Here,
            PackOpened::Focused(t) | PackOpened::Opened(t) => PackOpen::Tab(t),
            PackOpened::Refused => PackOpen::Refused,
        })
    }

    /// `apply` on `OpenPack`: the view, `DeckChanged` when a workshop was entered in place.
    pub(super) fn open_pack_ui(
        ws: &Mutex<Self>,
        tab: &str,
        pack: Uuid,
        widget: Option<&str>,
    ) -> Result<WorkspaceOutcome> {
        let opened = Self::open_pack_with(ws, tab, pack, widget, Opener::Ui)?;
        let json = Self::lock(ws).view_json()?;
        Ok(match opened {
            PackOpened::Here { tab, entered: true } => WorkspaceOutcome::DeckChanged { json, tab },
            PackOpened::Here { entered: false, .. }
            | PackOpened::Focused(_)
            | PackOpened::Opened(_)
            | PackOpened::Refused => WorkspaceOutcome::Applied(json),
        })
    }

    /// Opens `pack` from `tab`, saying how; holds the pack's opening lock.
    ///
    /// # Errors
    ///
    /// As [`crate::agent::Sessions::open_pack`].
    ///
    /// # Blocking
    ///
    /// As [`crate::agent::Sessions::open_pack`].
    pub(crate) fn open_pack_with(
        ws: &Mutex<Self>,
        tab: &str,
        pack: Uuid,
        widget: Option<&str>,
        opener: Opener<'_>,
    ) -> Result<PackOpened> {
        with_opening(pack, || {
            let (retarget, mut gate) = match opener {
                Opener::Ui => (true, None),
                Opener::Agent(gate) => (false, Some(gate)),
            };
            let mut ask = |g: PackGate| gate.as_mut().is_none_or(|gate| gate(g));
            let ctx = OpenCtx { ws, tab, pack, widget, retarget };
            let (window, candidates) = Self::lock(ws).candidates(tab, &TabDocument::Pack { pack })?;
            if let Some(done) = Self::focus_existing(&ctx, &candidates, &mut ask)? {
                return Ok(done);
            }
            Self::open_new(&ctx, window, &mut ask)
        })
    }

    /// Step 1: `tab`'s window, and the tabs cached as editing `wanted` (a pack or a theme).
    ///
    /// Every awake one (ready or busy) first, then at most four dormant ones, in window order.
    pub(super) fn candidates(&self, tab: &str, wanted: &TabDocument) -> Result<(String, Vec<String>)> {
        let window = self.opening_window(tab)?;
        let (mut awake, mut dormant) = (Vec::new(), Vec::new());
        for id in self.layout.windows.iter().flat_map(|w| &w.tabs) {
            let Some(t) = self.tabs.get(id).filter(|t| t.info.document == *wanted) else { continue };
            let sleeps = match t.session.try_lock() {
                Ok(slot) => slot.is_dormant(),
                Err(TryLockError::Poisoned(p)) => p.into_inner().is_dormant(),
                Err(TryLockError::WouldBlock) => false,
            };
            if sleeps { &mut dormant } else { &mut awake }.push(id.clone());
        }
        dormant.truncate(MAX_DORMANT_CANDIDATES);
        awake.extend(dormant);
        Ok((window, awake))
    }

    /// Step 2: focuses the first candidate that is still `pack`'s pack tab, asking `Focus` once.
    fn focus_existing(ctx: &OpenCtx<'_>, candidates: &[String], ask: &mut Ask<'_>) -> Result<Option<PackOpened>> {
        let mut asked = false;
        for candidate in candidates {
            if candidate != ctx.tab && !asked {
                if !ask(PackGate::Focus) {
                    return Ok(Some(PackOpened::Refused));
                }
                asked = true;
            }
            if let Some(done) = Self::focus_candidate(ctx, candidate)? {
                return Ok(Some(done));
            }
        }
        Ok(None)
    }

    /// Focuses `candidate` if its live session is `pack`'s pack tab; `None`: skipped.
    fn focus_candidate(ctx: &OpenCtx<'_>, candidate: &str) -> Result<Option<PackOpened>> {
        let wanted = TabDocument::Pack { pack: ctx.pack };
        let (woken, _changed) = Self::with_session(ctx.ws, candidate, |s| {
            if s.tab_info().document != wanted {
                return Ok(false);
            }
            if let Some(w) = ctx.widget {
                s.open_widget_tab(w)?;
            }
            Ok(true)
        });
        match woken {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => return Ok(None),
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                tracing::warn!(tab = candidate, error = %e.one_line(), "pack tab candidate not woken: skipped");
                return Ok(None);
            }
        }
        let mut w = Self::lock(ctx.ws);
        if w.window_of(candidate).is_err() {
            return Ok(None);
        }
        w.activate(candidate)?;
        let from_deck = w.tabs.get(ctx.tab).is_some_and(|t| matches!(t.info.document, TabDocument::Deck { .. }));
        if ctx.retarget
            && from_deck
            && candidate != ctx.tab
            && let Some(t) = w.tabs.get_mut(candidate)
        {
            t.opener = Some(ctx.tab.to_owned());
        }
        w.save_layout();
        Ok(Some(if candidate == ctx.tab {
            PackOpened::Here { tab: ctx.tab.to_owned(), entered: false }
        } else {
            PackOpened::Focused(candidate.to_owned())
        }))
    }

    /// Steps 3-5: routes the pack from `tab`; a library pack opens a new pack tab after it.
    fn open_new(ctx: &OpenCtx<'_>, window: String, ask: &mut Ask<'_>) -> Result<PackOpened> {
        let (route, _changed) = Self::with_session(ctx.ws, ctx.tab, |s| s.route_pack(ctx.pack, ctx.widget));
        let (scope, name) = match route?? {
            PackRoute::Here { entered } => return Ok(PackOpened::Here { tab: ctx.tab.to_owned(), entered }),
            PackRoute::Tab { scope, name } => (scope, name),
        };
        if !ask(PackGate::NewTab) {
            return Ok(PackOpened::Refused);
        }
        // Before any change to the opener (J7): a refusal leaves its widget session as it was.
        let lease = match PackLease::acquire(&scope, ctx.pack, &name, &name) {
            Ok(lease) => lease,
            Err(refused) => {
                let lagging = Self::lock(ctx.ws).live_tab(&TabDocument::Pack { pack: ctx.pack });
                let focused = match lagging {
                    Some(tab) => Self::focus_candidate(ctx, &tab)?,
                    None => None,
                };
                return focused.ok_or(refused);
            }
        };
        let (seed, _changed) = Self::with_session(ctx.ws, ctx.tab, |s| s.take_preview(ctx.pack));
        let seed = seed??;
        let pending = {
            let mut w = Self::lock(ctx.ws);
            let opening = Opening::Pack { pack: ctx.pack, widget: ctx.widget.map(str::to_owned) };
            // The requesting tab closed meanwhile: its window, at the end and without opener.
            let place =
                if w.tab_window(ctx.tab).is_some() { TabPlace::After(ctx.tab.to_owned()) } else { TabPlace::End };
            let window = w.tab_window(ctx.tab).map_or(window, str::to_owned);
            w.pending(Some(window), opening, place)
        };
        let session = pending.build_pack(*seed, lease)?;
        let id = pending.id.clone();
        match Self::finish(ctx.ws, pending, session) {
            Ok((_, id)) => Ok(PackOpened::Opened(id)),
            // Once inserted, the tab exists: an agent's claim is never released for it.
            Err(e) if Self::lock(ctx.ws).tabs.contains_key(&id) => {
                tracing::warn!(tab = %id, error = %e.one_line(), "pack tab opened, its view not built");
                Ok(PackOpened::Opened(id))
            }
            Err(e) => Err(e),
        }
    }

    /// The first tab, in window order, whose live session edits `wanted` (a pack or a theme).
    ///
    /// Read without waiting: an awake slot through `try_lock`, a busy one by
    /// its cached document; a dormant one holds no lease.
    pub(super) fn live_tab(&self, wanted: &TabDocument) -> Option<String> {
        let live = |slot: &Slot| slot.info().map(|i| i.document);
        self.layout.windows.iter().flat_map(|w| &w.tabs).find_map(|id| {
            let t = self.tabs.get(id)?;
            let document = match t.session.try_lock() {
                Ok(slot) => live(&slot),
                Err(TryLockError::Poisoned(p)) => live(&p.into_inner()),
                Err(TryLockError::WouldBlock) => Some(t.info.document.clone()),
            };
            (document.as_ref() == Some(wanted)).then(|| id.clone())
        })
    }

    /// The opener of pack tab `tab`, with the version it would take.
    ///
    /// # Errors
    ///
    /// `UnknownTab` for `tab`.
    ///
    /// # Blocking
    ///
    /// Reads two sessions and the library.
    pub(crate) fn pack_opener(ws: &Mutex<Self>, tab: &str) -> Result<Option<OpenerOffer>> {
        let opener = {
            let w = Self::lock(ws);
            w.tabs.get(tab).context(UnknownTabSnafu { id: tab })?.opener.clone()
        };
        let Some(opener) = opener else { return Ok(None) };
        let (wrote, _changed) = Self::with_session(ws, tab, |s| s.wrote_in_pack_tab());
        let Some(PackWrite { pack, name, version }) = wrote? else { return Ok(None) };
        let (offer, _changed) = Self::with_session(ws, &opener, |s| s.opener_offer(pack, &version));
        let deck = match offer {
            Ok(Some(deck)) => deck,
            Ok(None) => return Ok(None),
            Err(e) if e.kind() == SessionErrorKind::UnknownTab => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(Some(OpenerOffer { tab: opener, deck, pack, pack_name: name, version }))
    }

    /// Detaches `tab`; an active pack or theme tab hands the focus back to its opener.
    ///
    /// Only when the opener is still a tab of the same window; otherwise the neighbour rule.
    pub(super) fn detach_to_opener(&mut self, tab: &str) -> Result<()> {
        let window = self.window_of(tab)?;
        let was_active = self.layout.windows[window].active == tab;
        let (label, opener) =
            (self.layout.windows[window].label.clone(), self.tabs.get(tab).and_then(|t| t.opener.clone()));
        self.detach(tab)?;
        if let Some(opener) = opener.filter(|_| was_active)
            && let Some(win) = self.layout.windows.iter_mut().find(|w| w.label == label)
            && win.tabs.contains(&opener)
        {
            win.active = opener;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "workspace_packs_fixtures.rs"]
pub(super) mod workspace_packs_fixtures;

#[cfg(test)]
#[path = "workspace_packs_tests.rs"]
mod workspace_packs_tests;

#[cfg(test)]
#[path = "workspace_packs_race_tests.rs"]
mod workspace_packs_race_tests;

#[cfg(test)]
#[path = "workspace_packs_close_tests.rs"]
mod workspace_packs_close_tests;
