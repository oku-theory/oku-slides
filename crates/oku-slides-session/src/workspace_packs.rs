//! Pack tabs in the workspace: opening a pack from a tab, and its opener.
//!
//! An open of pack P runs under P's opening lock (`with_opening`,
//! `workspace_opening.rs`): a second open of P waits, then finds the first
//! one's tab and focuses it; opens of other packs never wait on it. Under that lock, the workspace and each
//! session are taken in sequence, never nested: a pack tab of P already open
//! is focused (steps 1-2, `focus_existing`), else the requesting tab routes
//! the pack (`route_pack`): a library pack opens a new pack tab right after
//! it, leased before anything changes (steps 3-5, `open_new`); a deck-only
//! pack of a deck opens its deck-pack tab, linked to that deck
//! (`open_deck_pack`, `workspace_deck_packs.rs`). An agent's gate is asked before any change, with only the
//! opening lock held; the user's open asks nothing and makes the requesting
//! deck the tab's opener, which a close hands the focus back to. A new pack
//! (`new_pack`) is created empty through the requesting tab's library, then
//! opened like any other; its draft goes if that open fails.

use std::sync::{Mutex, TryLockError};

use serde::Serialize;
use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::{PackKind, PackVersion};

use crate::SessionErrorKind;
use crate::agent::{TabGate, TabOpen};
use crate::deck_offer::DeckOffer;
use crate::error::SingleDocumentSnafu;
use crate::library_packs::{DraftKind, PackLibrary};
use crate::pack_lease::PackLease;
use crate::pack_tab::{PackRoute, PackWrite};

use super::opening::with_opening;
#[cfg(test)]
use super::opening::{opening_entry, opening_waiters};
#[cfg(test)]
use super::tabs::open_failpoint;
use super::{
    Activate, Created, Opening, Pending, Result, Slot, TabPlace, UnknownTabSnafu, Workspace, WorkspaceOutcome,
};
use crate::TabDocument;

/// Dormant candidates woken at most by one open (O10): past them, the live re-scan decides.
const MAX_DORMANT_CANDIDATES: usize = 4;

/// Who opens a pack or theme tab: the user (no limit, retargets, always shown) or an agent's gate.
pub(crate) enum Opener<'a> {
    /// The user, through `Workspace::apply`.
    Ui,
    /// An agent: its gate, asked before any change (one `Focus`, then one
    /// `NewTab`, then one `Focus` after a lease refusal), never under the workspace guard.
    Agent(&'a mut dyn FnMut(TabGate) -> bool),
}

impl std::fmt::Debug for Opener<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ui => "Ui",
            Self::Agent(_) => "Agent",
        })
    }
}

/// How opening a pack or theme tab went, for `apply`.
///
/// `shown`: the tab became its window's active tab (always for the user).
#[derive(Debug)]
pub(crate) enum PackOpened {
    /// An open pack or theme tab was focused.
    Focused { tab: String, shown: bool },
    /// A new pack or theme tab opened.
    Opened { tab: String, shown: bool },
    /// The requesting tab edits it; its item selected, if one was named.
    Here,
    /// The gate refused.
    Refused,
}

impl From<PackOpened> for TabOpen {
    fn from(opened: PackOpened) -> Self {
        match opened {
            PackOpened::Here => Self::Here,
            PackOpened::Focused { tab, shown } => Self::Focused { tab, shown },
            PackOpened::Opened { tab, shown } => Self::Opened { tab, shown },
            PackOpened::Refused => Self::Refused,
        }
    }
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
pub(super) struct OpenCtx<'a> {
    /// The workspace.
    pub(super) ws: &'a Mutex<Workspace>,
    /// The requesting tab.
    pub(super) tab: &'a str,
    /// The pack to open.
    pub(super) pack: Uuid,
    /// An item selected in the pack tab.
    pub(super) widget: Option<&'a str>,
    /// The user's open: always shown, and the requesting deck becomes the focused tab's opener.
    pub(super) retarget: bool,
    /// The pack was created empty for this open.
    pub(super) created: Created,
}

impl OpenCtx<'_> {
    /// How the tab this open builds is activated: an agent's only while its requester is shown.
    pub(super) fn activation(&self) -> Activate {
        if self.retarget { Activate::Always } else { Activate::IfShown(self.tab.to_owned()) }
    }
}

/// The gate of one open: one `Focus`, one `NewTab`, one `Focus` after a lease refusal.
pub(super) type Ask<'a> = dyn FnMut(TabGate) -> bool + 'a;

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
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen> {
        Ok(Self::open_pack_with(ws, tab, pack, widget, Opener::Agent(gate), Created::No)?.into())
    }

    /// `apply` on `OpenPack`: the view.
    pub(super) fn open_pack_ui(
        ws: &Mutex<Self>,
        tab: &str,
        pack: Uuid,
        widget: Option<&str>,
    ) -> Result<WorkspaceOutcome> {
        Self::open_pack_with(ws, tab, pack, widget, Opener::Ui, Created::No)?;
        Ok(WorkspaceOutcome::Applied(Self::lock(ws).view_json()?))
    }

    /// `apply` on `NewPack`: an empty library pack of `kind`, opened in its own tab after `tab`.
    ///
    /// The requesting tab checks it may open a pack and has a lease scope,
    /// then creates the pack through its library, in its language; the open
    /// runs outside any lock. An open that fails deletes the draft again.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `ReadOnly`, `InHistory`, `WrongDocument` (a theme tab),
    /// `SingleDocument` without a lease scope, the creation's and the open's errors.
    ///
    /// # Blocking
    ///
    /// Writes the library, then builds and renders the new tab's session.
    pub(super) fn new_pack(ws: &Mutex<Self>, tab: &str, kind: PackKind) -> Result<WorkspaceOutcome> {
        let (created, _changed) = Self::with_quiet(ws, tab, |s| {
            s.can_open_pack()?;
            snafu::ensure!(s.library.lease_scope().is_some(), SingleDocumentSnafu);
            let name = s.localizer.tr("pack-default-name");
            Ok::<_, crate::SessionError>((s.library.create_empty_pack(kind, &name)?, s.library.clone()))
        });
        let (id, library) = created??;
        let opened = Self::open_pack_with(ws, tab, id, None, Opener::Ui, Created::Yes);
        if !matches!(opened, Ok(PackOpened::Opened { .. })) {
            Self::roll_back(&library, DraftKind::Pack, id);
        }
        opened?;
        Ok(WorkspaceOutcome::Applied(Self::lock(ws).view_json()?))
    }

    /// Deletes draft `id` again after its open failed, logging a failure.
    pub(super) fn roll_back(library: &PackLibrary, kind: DraftKind, id: Uuid) {
        if let Err(e) = library.discard_untouched(kind, id) {
            tracing::warn!(draft = %id, error = %e.one_line(), "draft of a failed open not deleted");
        }
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
    pub(super) fn open_pack_with(
        ws: &Mutex<Self>,
        tab: &str,
        pack: Uuid,
        widget: Option<&str>,
        opener: Opener<'_>,
        created: Created,
    ) -> Result<PackOpened> {
        #[cfg(test)]
        open_failpoint()?;
        with_opening(pack, || {
            let (retarget, mut gate) = match opener {
                Opener::Ui => (true, None),
                Opener::Agent(gate) => (false, Some(gate)),
            };
            let (window, candidates) = {
                let w = Self::lock(ws);
                let wanted = w.pack_documents(tab, pack);
                w.candidates(tab, &wanted)?
            };
            let mut ask = |g: TabGate| gate.as_mut().is_none_or(|gate| gate(g));
            let ctx = OpenCtx { ws, tab, pack, widget, retarget, created };
            if let Some(done) = Self::focus_existing(&ctx, &candidates, &mut ask)? {
                return Ok(done);
            }
            Self::open_new(&ctx, window, &mut ask)
        })
    }

    /// The documents of `pack`'s tab as `tab` may find it: its pack tab, or its deck-pack tab.
    ///
    /// A deck-pack tab's is the one linked to `tab`, or `tab` itself when it edits `pack`.
    fn pack_documents(&self, tab: &str, pack: Uuid) -> Vec<TabDocument> {
        let mut wanted = vec![TabDocument::Pack { pack }, TabDocument::DeckPack { deck_tab: tab.to_owned(), pack }];
        if let Some(TabDocument::DeckPack { deck_tab, pack: own }) = self.tabs.get(tab).map(|t| &t.info.document)
            && *own == pack
        {
            wanted.push(TabDocument::DeckPack { deck_tab: deck_tab.clone(), pack });
        }
        wanted
    }

    /// Step 1: `tab`'s window, and the tabs cached as editing one of `wanted` (a pack or a theme).
    ///
    /// Every awake one (ready or busy) first, then at most four dormant ones, in window order.
    pub(super) fn candidates(&self, tab: &str, wanted: &[TabDocument]) -> Result<(String, Vec<String>)> {
        let window = self.opening_window(tab)?;
        let (mut awake, mut dormant) = (Vec::new(), Vec::new());
        for id in self.layout.windows.iter().flat_map(|w| &w.tabs) {
            let Some(t) = self.tabs.get(id).filter(|t| wanted.contains(&t.info.document)) else { continue };
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
                if !ask(TabGate::Focus) {
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

    /// Focuses `candidate` if its live session is `pack`'s pack or deck-pack tab; `None`: skipped.
    ///
    /// A deck-pack tab counts when linked to the requesting tab, or when it is
    /// that tab. It becomes its window's active tab for the user, or when the
    /// requesting tab is its window's active tab under the activating guard.
    fn focus_candidate(ctx: &OpenCtx<'_>, candidate: &str) -> Result<Option<PackOpened>> {
        let edits = |document: TabDocument| match document {
            TabDocument::Pack { pack } => pack == ctx.pack,
            TabDocument::DeckPack { deck_tab, pack } => {
                pack == ctx.pack && (deck_tab == ctx.tab || candidate == ctx.tab)
            }
            TabDocument::Deck { .. } | TabDocument::Help { .. } | TabDocument::Theme { .. } => false,
        };
        let (woken, _changed) = Self::with_quiet(ctx.ws, candidate, |s| {
            if !edits(s.tab_info().document) {
                return Ok(false);
            }
            if let Some(w) = ctx.widget {
                s.select_item(w)?;
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
        let shown = ctx.retarget || w.tab_shown(ctx.tab);
        if shown {
            w.activate(candidate)?;
        }
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
            PackOpened::Here
        } else {
            PackOpened::Focused { tab: candidate.to_owned(), shown }
        }))
    }

    /// Steps 3-5: routes the pack from `tab`; a library pack opens a new pack tab after it.
    ///
    /// A deck-only pack of `tab`'s deck opens its deck-pack tab (`open_deck_pack`).
    fn open_new(ctx: &OpenCtx<'_>, window: String, ask: &mut Ask<'_>) -> Result<PackOpened> {
        let (route, _changed) = Self::with_quiet(ctx.ws, ctx.tab, |s| s.route_pack(ctx.pack, ctx.widget));
        let (scope, name) = match route?? {
            PackRoute::Here => return Ok(PackOpened::Here),
            PackRoute::Tab { scope, name } => (scope, name),
            PackRoute::DeckTab { name } => return Self::open_deck_pack(ctx, window, &name, ask),
        };
        if !ask(TabGate::NewTab) {
            return Ok(PackOpened::Refused);
        }
        // Before any change to the opener (J7): a refusal leaves its widget session as it was.
        let lease = match PackLease::acquire(&scope, ctx.pack, &name, &name) {
            Ok(lease) => lease,
            Err(refused) => {
                // Read in its own statement: the gate is never asked under the workspace guard.
                let lagging = Self::lock(ctx.ws).live_tab(&TabDocument::Pack { pack: ctx.pack });
                let Some(holder) = lagging else { return Err(refused) };
                if !ask(TabGate::Focus) {
                    return Ok(PackOpened::Refused);
                }
                return Self::focus_candidate(ctx, &holder)?.ok_or(refused);
            }
        };
        let (seed, _changed) = Self::with_quiet(ctx.ws, ctx.tab, |s| s.take_preview(ctx.pack));
        let seed = seed??;
        let pending = {
            let mut w = Self::lock(ctx.ws);
            let opening = Opening::Pack { pack: ctx.pack, widget: ctx.widget.map(str::to_owned), created: ctx.created };
            // The requesting tab closed meanwhile: its window, at the end and without opener.
            let place =
                if w.tab_window(ctx.tab).is_some() { TabPlace::After(ctx.tab.to_owned()) } else { TabPlace::End };
            let window = w.tab_window(ctx.tab).map_or(window, str::to_owned);
            let mut pending = w.pending(Some(window), opening, place);
            pending.activate = ctx.activation();
            pending
        };
        let session = pending.build_pack(*seed, lease)?;
        Self::finish_opened(ctx.ws, pending, session)
    }

    /// Inserts a built pack, deck-pack or theme tab: `Opened` once it is in, even if its view failed.
    ///
    /// Once inserted, the tab exists: an agent's claim is never released for
    /// it. `shown` is the activation `finish` decided, or, after a failed
    /// view, whether the tab is its window's active tab.
    pub(super) fn finish_opened(ws: &Mutex<Self>, pending: Pending, session: crate::Session) -> Result<PackOpened> {
        let id = pending.id.clone();
        let e = match Self::finish_shown(ws, pending, session) {
            Ok((_, tab, shown)) => return Ok(PackOpened::Opened { tab, shown }),
            Err(e) => e,
        };
        let inserted = {
            let w = Self::lock(ws);
            w.tabs.contains_key(&id).then(|| w.tab_shown(&id))
        };
        let Some(shown) = inserted else { return Err(e) };
        tracing::warn!(tab = %id, error = %e.one_line(), "tab opened, its view not built");
        Ok(PackOpened::Opened { tab: id, shown })
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
    /// `None` for a deck-pack tab: its saves go to its owner deck.
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
            let t = w.tabs.get(tab).context(UnknownTabSnafu { id: tab })?;
            t.opener.clone().filter(|_| t.info.document.deck_tab().is_none())
        };
        let Some(opener) = opener else { return Ok(None) };
        let (wrote, _changed) = Self::with_quiet(ws, tab, |s| s.wrote_in_pack_tab());
        let Some(PackWrite { pack, name, version }) = wrote? else { return Ok(None) };
        let (offer, _changed) = Self::with_quiet(ws, &opener, |s| s.opener_offer(pack, &version));
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

#[cfg(test)]
#[path = "workspace_packs_new_tests.rs"]
mod workspace_packs_new_tests;
