//! Thread turns: numbers, filing (in the call's turn), freezing, eviction, expiry of follow-ups.

use oku_slides_core::GuideKind;
use oku_slides_session::agent::receipt::{ReceiptSurfaces, fallback};
use oku_slides_session::agent::{KEPT_TURNS, Place, Surface, Trace, TurnOrigin};

use uuid::Uuid;

use super::{HubState, McpEvent, Shown, SurfaceHub};

/// What a tab-move claim answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reserve {
    /// The claim is held for this turn.
    Granted,
    /// The call's turn is not the open one, or the hub closed.
    NotThisTurn,
    /// This turn already claimed another tab.
    Taken,
}

/// The tab a turn's one move is about.
///
/// A pack and a theme never share a claim, even under one UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabTarget {
    /// A pack's tab.
    Pack(Uuid),
    /// A theme's tab.
    Theme(Uuid),
    /// The new tab of one `import` call: a fresh id per call, so two imports never share a claim.
    Import(Uuid),
    /// An open tab the agent activates, reorders or moves (`tab`): the number of its id `t-<n>`.
    Tab(u64),
    /// The help tab, opened or shown (`help`).
    Help,
}

impl std::fmt::Display for TabTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pack(id) => write!(f, "pack {id}"),
            Self::Theme(id) => write!(f, "theme {id}"),
            Self::Import(id) => write!(f, "import {id}"),
            Self::Tab(n) => write!(f, "tab t-{n}"),
            Self::Help => write!(f, "help"),
        }
    }
}

/// This turn's claim of its one tab move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TabClaim {
    /// The turn that claimed it.
    pub(super) turn: u32,
    /// The tab it moves to.
    pub(super) target: TabTarget,
    /// The claims granted on it and not given back.
    pub(super) claims: u32,
}

/// What a turn's tools ask to run once the turn has ended, never inside a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterTurn {
    /// The conversation starts over, once the turn's receipt is shown.
    NewConversation,
    /// The app quits, if no tab gained unsaved work meanwhile.
    Quit,
    /// The app installs its update and restarts.
    Install,
    /// The agent's own tab closes, alone or with its window, if no covered tab changed meanwhile.
    CloseOwn {
        /// The confirmed close decision (`SurfaceHub::store_close`), announced once the turn ended.
        decision: Uuid,
    },
}

impl AfterTurn {
    /// Quitting, installing and closing the agent's own tab end it: a turn records one at most.
    fn ends_app(self) -> bool {
        matches!(self, Self::Quit | Self::Install | Self::CloseOwn { .. })
    }
}

/// What replaces a thread question once answered (or cancelled).
pub type Freeze = Box<dyn FnOnce(&super::Answer) -> Surface + Send>;

/// Which turns may still receive a frozen surface.
#[derive(Debug, Clone, Copy)]
enum Admit {
    /// The open turn only: the tools' surfaces (`render`, `report`).
    Open,
    /// Any turn still in the thread: a frozen question, a fallback receipt (the turn just ended).
    InThread,
}

/// Surfaces figées gardées au plus par tour (`render`, questions répondues) : contre un agent qui en empile.
pub const STATIC_PER_TURN: usize = 8;

impl SurfaceHub {
    /// Opens a turn in the thread.
    ///
    /// It closes the oldest turns of the thread beyond `KEPT_TURNS`. Only this
    /// eviction counts (read-back turns included). The one of the agent's turn
    /// reducer (`TurnOut::Evict`) still calls `close_turn` on a turn already
    /// evicted here: redundant, but harmless (`close_turn` is idempotent).
    pub fn begin_turn(&self) -> u32 {
        self.begin_turn_from(TurnOrigin::Typed)
    }

    /// Opens a turn in the thread, as [`Self::begin_turn`], and keeps how it began.
    pub fn begin_turn_from(&self, origin: TurnOrigin) -> u32 {
        let (n, old) = {
            let mut state = self.state();
            state.turns = state.turns.saturating_add(1);
            let n = state.turns;
            state.turn = Some(n);
            state.origin = Some((n, origin));
            state.live_turns.insert(n);
            let excess = state.live_turns.len().saturating_sub(KEPT_TURNS);
            (n, state.live_turns.iter().copied().take(excess).collect::<Vec<_>>())
        };
        for t in old {
            self.close_turn(t);
        }
        self.expire_actions();
        n
    }

    /// How turn `turn` began, while it is the last turn begun.
    #[must_use]
    pub fn turn_origin(&self, turn: u32) -> Option<TurnOrigin> {
        self.state().origin.filter(|(n, _)| *n == turn).map(|(_, origin)| origin)
    }

    /// The open turn, `None` between turns: the token a tool call takes when it arrives.
    ///
    /// A thread surface a tool creates is filed in its call's turn, never in the one open
    /// when it shows: `Tools::call` reads this turn before any work and passes it to
    /// `show_static`, `show_report`, `ask_then` or `show_params`.
    pub fn turn(&self) -> Option<u32> {
        self.state().turn
    }

    /// True while `turn` is the open turn of a live hub.
    #[must_use]
    pub(crate) fn turn_is_open(&self, turn: u32) -> bool {
        Self::admits(&self.state(), turn, Admit::Open)
    }

    /// Claims this turn's one tab move, to `target`.
    ///
    /// The decision and the claim are one step under the state lock, so of many concurrent
    /// claims on distinct targets in one turn exactly one is granted. A claim of a later turn is
    /// never overwritten; claims on the same target are counted, so each failed open gives back
    /// its own.
    #[must_use]
    pub(crate) fn reserve_tab(&self, turn: u32, target: TabTarget) -> Reserve {
        let mut state = self.state();
        if !Self::admits(&state, turn, Admit::Open) {
            return Reserve::NotThisTurn;
        }
        match &mut state.tab_claim {
            Some(claim) if claim.turn >= turn => {
                if claim.turn != turn || claim.target != target {
                    return Reserve::Taken;
                }
                match claim.claims.checked_add(1) {
                    Some(claims) => {
                        claim.claims = claims;
                        Reserve::Granted
                    }
                    None => Reserve::Taken,
                }
            }
            slot => {
                *slot = Some(TabClaim { turn, target, claims: 1 });
                Reserve::Granted
            }
        }
    }

    /// Gives back one claim of `target` in `turn`, if still held.
    pub(crate) fn release_tab(&self, turn: u32, target: TabTarget) {
        let mut state = self.state();
        let Some(claim) = state.tab_claim.as_mut().filter(|c| c.turn == turn && c.target == target) else {
            return;
        };
        match claim.claims.checked_sub(1) {
            Some(left) if left > 0 => claim.claims = left,
            Some(_) => state.tab_claim = None,
            None => {
                tracing::warn!(turn, %target, "tab claim released more often than granted");
                state.tab_claim = None;
            }
        }
    }

    /// Records that `turn` moved to another tab.
    ///
    /// An older turn never replaces a newer one.
    pub(crate) fn note_elsewhere(&self, turn: u32) {
        let mut state = self.state();
        if state.elsewhere.is_none_or(|noted| noted < turn) {
            state.elsewhere = Some(turn);
        }
    }

    /// True while `turn` must leave its own tab's widgets alone.
    ///
    /// That is, for the open turn, once it holds a tab claim or moved to another tab.
    #[must_use]
    pub(crate) fn writes_elsewhere(&self, turn: u32) -> bool {
        let state = self.state();
        Self::admits(&state, turn, Admit::Open)
            && (state.tab_claim.is_some_and(|claim| claim.turn == turn) || state.elsewhere == Some(turn))
    }

    /// Whether turn `turn` may still receive a surface (the lock already taken).
    fn admits(state: &HubState, turn: u32, admit: Admit) -> bool {
        let open = match admit {
            Admit::Open => state.turn == Some(turn),
            Admit::InThread => true,
        };
        open && !state.closed && state.live_turns.contains(&turn)
    }

    /// Records `action` to run once turn `turn` has ended; `false` when refused.
    ///
    /// Refused outside the open turn, and for a second quit, install or own close in one turn: the first
    /// one recorded wins. A new conversation is kept once.
    #[must_use]
    pub fn after_turn(&self, turn: u32, action: AfterTurn) -> bool {
        let mut state = self.state();
        if !Self::admits(&state, turn, Admit::Open) {
            return false;
        }
        let slot = &mut state.after_turn;
        if slot.as_ref().is_none_or(|(t, _)| *t != turn) {
            *slot = Some((turn, Vec::new()));
        }
        let Some((_, actions)) = slot.as_mut() else { return false };
        if action.ends_app() && actions.iter().any(|a| a.ends_app()) {
            return false;
        }
        if !actions.contains(&action) {
            actions.push(action);
        }
        true
    }

    /// True once turn `turn` recorded a quit, an install or its own tab's close.
    #[must_use]
    pub fn ends_app_after(&self, turn: u32) -> bool {
        self.state().after_turn.as_ref().is_some_and(|(t, a)| *t == turn && a.iter().any(|a| a.ends_app()))
    }

    /// True once turn `turn` recorded anything to run after it, a new conversation included.
    #[must_use]
    pub fn records_after(&self, turn: u32) -> bool {
        self.state().after_turn.as_ref().is_some_and(|(t, a)| *t == turn && !a.is_empty())
    }

    /// Takes what turn `turn` recorded, in recording order; empty for any other turn.
    pub fn take_after_turn(&self, turn: u32) -> Vec<AfterTurn> {
        let mut state = self.state();
        match state.after_turn.take() {
            Some((t, actions)) if t == turn => actions,
            other => {
                state.after_turn = other;
                Vec::new()
            }
        }
    }

    /// Forgets everything recorded: the turn was stopped, cancelled or failed.
    pub fn drop_after_turn(&self) {
        self.state().after_turn = None;
    }

    /// Shows `text` in a card of turn `turn`, while that turn is in the thread.
    ///
    /// For what happened once the turn ended: an after-turn action dropped or refused.
    pub fn after_turn_notice(&self, turn: u32, text: &str) {
        let id = self.next_id("note");
        let shown = fallback(&id, text, &[], &[], &self.localizer.current())
            .is_some_and(|card| self.put_static(turn, card, Admit::InThread));
        if !shown {
            tracing::debug!(turn, "after-turn notice not shown: the turn left the thread");
        }
    }

    /// Une composition `render` dans le tour `turn` de son appel.
    ///
    /// Refusée (`false`) si l'onglet est fermé ou si `turn` n'est plus le tour ouvert (appel
    /// tardif d'un tour annulé ou clos) : la décision et l'insertion se font sous le même verrou.
    pub fn show_static(&self, turn: u32, surface: Surface) -> bool {
        self.put_static(turn, surface, Admit::Open)
    }

    /// Une question répondue (ou annulée), figée dans son tour, tant que ce tour est dans le fil.
    pub(super) fn show_frozen(&self, turn: u32, surface: Surface) -> bool {
        self.put_static(turn, surface, Admit::InThread)
    }

    fn put_static(&self, turn: u32, mut surface: Surface, admit: Admit) -> bool {
        let oldest = {
            let mut state = self.state();
            if !Self::admits(&state, turn, admit) {
                return false;
            }
            surface.turn = Some(turn);
            surface.live = false;
            let plain = surface.has_actions().then(|| surface.without_actions());
            state
                .shows
                .insert(surface.id.clone(), Shown::Static { turn: Some(turn), plain, expired: false, report: false });
            // Au plus `STATIC_PER_TURN` surfaces figées par tour : la plus ancienne cède la place.
            let same: Vec<String> = state
                .open
                .iter()
                .filter(|s| s.turn == Some(turn) && s.id != surface.id && !s.live)
                // Le reçu du tour ne compte pas : il est unique et ne doit jamais céder sa place.
                .filter(|s| matches!(state.shows.get(&s.id), Some(Shown::Static { report: false, .. })))
                .map(|s| s.id.clone())
                .collect();
            let excess = (same.len() + 1).saturating_sub(STATIC_PER_TURN);
            same.into_iter().take(excess).collect::<Vec<_>>()
        };
        self.close_all(oldest);
        self.emit(McpEvent::surface(surface));
        true
    }

    /// Freezes surface `id`: read-only in its turn of the thread, with its last values.
    ///
    /// An inspector offer goes back to the thread too, where its turn shows it.
    pub fn freeze(&self, id: &str) {
        let frozen = {
            let mut state = self.state();
            let Some(open) = state.open.iter_mut().find(|s| s.id == id) else {
                tracing::debug!(surface = id, "surface à figer introuvable");
                return;
            };
            // En lecture seule : ses actions n'auraient plus de destinataire.
            let mut frozen = open.without_actions();
            frozen.live = false;
            frozen.place = Place::Thread;
            *open = frozen.clone();
            let turn = frozen.turn;
            state.shows.insert(id.to_owned(), Shown::Static { turn, plain: None, expired: false, report: false });
            frozen
        };
        self.emit(McpEvent::surface(frozen));
    }

    /// Le reçu du tour `turn` de son appel ; remplace un reçu déjà montré pour ce tour.
    ///
    /// Refusé (`false`) comme `show_static` : onglet fermé, ou `turn` n'est plus le tour ouvert.
    /// Un appel tardif ne touche donc jamais au reçu (de repli ou non) d'un autre tour.
    pub fn show_report(&self, turn: u32, report: ReceiptSurfaces) -> bool {
        self.put_report(turn, report, Admit::Open)
    }

    fn put_report(&self, turn: u32, report: ReceiptSurfaces, admit: Admit) -> bool {
        let ReceiptSurfaces { mut surface, plain, spoken } = report;
        let old: Vec<String> = {
            let mut state = self.state();
            if !Self::admits(&state, turn, admit) {
                return false;
            }
            surface.turn = Some(turn);
            surface.live = false;
            let old: Vec<String> = state
                .shows
                .iter()
                .filter(|(_, s)| matches!(s, Shown::Static { turn: t, report: true, .. } if *t == Some(turn)))
                .map(|(id, _)| id.clone())
                .collect();
            for id in &old {
                state.shows.remove(id);
            }
            state.reported.insert(turn);
            let plain = plain.map(|mut p| {
                p.turn = Some(turn);
                p.live = false;
                p
            });
            state
                .shows
                .insert(surface.id.clone(), Shown::Static { turn: Some(turn), plain, expired: false, report: true });
            old
        };
        for id in old {
            self.emit(McpEvent::CloseSurface { id });
        }
        self.emit(McpEvent::surface_aloud(surface, spoken));
        true
    }

    /// Records a guide write of turn `turn`, which a receipt or a card of that turn lists.
    ///
    /// A later write of the same guide replaces one no receipt lists yet. A write that comes
    /// once the turn has its receipt, or has ended, is shown at once in a card of its own.
    pub fn note_guide_write(&self, turn: u32, which: GuideKind, fact: String) {
        let late = {
            let mut state = self.state();
            let listed = state.guide_listed.get(&turn).copied().unwrap_or(0);
            let writes = state.guide_writes.entry(turn).or_default();
            match writes.iter_mut().skip(listed).find(|(k, _)| *k == which) {
                Some(unlisted) => unlisted.1 = fact,
                None => writes.push((which, fact)),
            }
            state.turn != Some(turn) || state.reported.contains(&turn)
        };
        if late {
            self.show_unlisted_guide_writes(turn);
        }
    }

    /// The lines of turn `turn`'s guide writes no receipt lists yet, in write order.
    pub fn guide_writes(&self, turn: u32) -> Vec<String> {
        let state = self.state();
        let listed = state.guide_listed.get(&turn).copied().unwrap_or(0);
        let writes = state.guide_writes.get(&turn).map_or(&[][..], Vec::as_slice);
        writes.iter().skip(listed).map(|(_, fact)| fact.clone()).collect()
    }

    /// Counts the first `count` unlisted guide writes of turn `turn` as listed by its receipt.
    pub fn mark_guide_writes_listed(&self, turn: u32, count: usize) {
        let mut state = self.state();
        let total = state.guide_writes.get(&turn).map_or(0, Vec::len);
        let listed = state.guide_listed.entry(turn).or_insert(0);
        *listed = listed.saturating_add(count).min(total);
    }

    /// Shows the guide writes of turn `turn` that no receipt lists, in a card of that turn.
    fn show_unlisted_guide_writes(&self, turn: u32) {
        let facts = self.guide_writes(turn);
        if facts.is_empty() {
            return;
        }
        self.mark_guide_writes_listed(turn, facts.len());
        let id = self.next_id("report");
        let shown = fallback(&id, "", &facts, &[], &self.localizer.current())
            .is_some_and(|card| self.put_static(turn, card, Admit::InThread));
        if !shown {
            tracing::debug!(turn, "late guide writes not shown: the turn left the thread");
        }
    }

    /// Le tour `turn` a-t-il son reçu ?
    pub fn has_report(&self, turn: u32) -> bool {
        self.state().reported.contains(&turn)
    }

    /// The traces of turn `turn`, as its tab closed it, for its fallback receipt (`end_turn`).
    pub fn note_turn_traces(&self, turn: u32, traces: Vec<Trace>) {
        let mut state = self.state();
        // Only a turn of the thread gets a receipt: an evicted or cleared one keeps nothing.
        if state.live_turns.contains(&turn) && !traces.is_empty() {
            state.traces.insert(turn, traces);
        }
    }

    /// Fin du tour `turn` : plus de tour en cours, et reçu de repli si l'agent n'en a pas fait.
    ///
    /// Once done, [`SurfaceHub::turn_ended`] answers for `turn`. The fallback lists the turn's
    /// traces (`note_turn_traces`), dropped here either way.
    pub fn end_turn(&self, turn: u32, narration: &str) {
        self.close_open_turn(turn, narration);
        self.note_ended(turn);
    }

    fn close_open_turn(&self, turn: u32, narration: &str) {
        let traces = {
            // No open turn any more: a late call of this turn is refused (see `turn`).
            let mut state = self.state();
            if state.turn == Some(turn) {
                state.turn = None;
            }
            state.traces.remove(&turn).unwrap_or_default()
        };
        if self.has_report(turn) {
            // Writes the report could not list (made after it, or while it was built).
            self.show_unlisted_guide_writes(turn);
            return;
        }
        let written = self.guide_writes(turn);
        if narration.trim().is_empty() && written.is_empty() && traces.is_empty() {
            return;
        }
        self.mark_guide_writes_listed(turn, written.len());
        let id = self.next_id("report");
        if let Some(surface) = fallback(&id, narration, &written, &traces, &self.localizer.current()) {
            // The turn is closed, but its fallback receipt goes to it while it is in the thread.
            // Built from the agent's narration: never read aloud.
            self.put_report(turn, ReceiptSurfaces { surface, plain: None, spoken: None }, Admit::InThread);
        }
    }

    /// Ferme les surfaces du tour `n` (tour évincé du fil).
    ///
    /// Toutes, vivantes comprises : des réglages (`show_params`) encore au dock quand leur tour
    /// est évincé sont **fermés, pas figés** (leur tour n'est plus dans le fil pour les recevoir),
    /// et une action en retard sur eux est refusée (`unknown surface`). Cas rare : il faut
    /// `KEPT_TURNS` tours sans nouveaux réglages ni fermeture. Une question en attente n'y survit
    /// pas non plus, mais elle est déjà relâchée par le tour suivant (`cancel_asks`).
    pub fn close_turn(&self, n: u32) {
        let ids = {
            let mut state = self.state();
            state.live_turns.remove(&n);
            state.reported.remove(&n);
            state.guide_writes.remove(&n);
            state.guide_listed.remove(&n);
            state.traces.remove(&n);
            Self::thread_ids(&state, |t| t == n)
        };
        self.close_all(ids);
    }

    /// Nouvelle conversation : les questions du fil relâchées, ses tours et surfaces fermés.
    ///
    /// Appelée **par le gardien de l'onglet** (ordre `Forget`, dans l'ordre de sa
    /// file), **avant la consigne suivante** : l'acteur attend la réponse de l'oubli,
    /// donc jamais en concurrence avec `begin_turn`. Tout est décidé sous un seul verrou.
    pub fn clear_thread(&self) {
        let ids = {
            let mut state = self.state();
            let turns = std::mem::take(&mut state.live_turns);
            let ids = Self::thread_ids(&state, |t| turns.contains(&t));
            if state.turn.is_some_and(|t| turns.contains(&t)) {
                state.turn = None;
            }
            for t in &turns {
                state.reported.remove(t);
                state.guide_writes.remove(t);
                state.guide_listed.remove(t);
                state.traces.remove(t);
            }
            ids
        };
        // Après l'effacement : une question relâchée ici ne trouve plus son tour et se ferme au lieu de se figer.
        self.cancel_asks_where(|id| ids.iter().any(|x| x == id));
        self.close_all(ids);
    }

    /// Les surfaces rangées dans un tour que `which` désigne.
    fn thread_ids(state: &HubState, which: impl Fn(u32) -> bool) -> Vec<String> {
        state.open.iter().filter(|s| s.turn.is_some_and(&which)).map(|s| s.id.clone()).collect()
    }

    /// Ferme `ids` (sans verrou tenu par l'appelant).
    pub(super) fn close_all(&self, ids: Vec<String>) {
        for id in ids {
            self.state().shows.remove(&id);
            self.emit(McpEvent::CloseSurface { id });
        }
    }

    /// Les suites des tours passés expirent : leur surface est remplacée par sa version sans actions.
    fn expire_actions(&self) {
        let plains: Vec<Surface> = {
            let mut state = self.state();
            let current = state.turn;
            state
                .shows
                .values_mut()
                .filter_map(|shown| match shown {
                    Shown::Static { turn, plain, expired, .. } if !*expired && *turn != current => {
                        *expired = true;
                        plain.take()
                    }
                    _ => None,
                })
                .collect()
        };
        for surface in plains {
            self.emit(McpEvent::surface(surface));
        }
    }
}

#[cfg(test)]
#[path = "hub_turns_tests.rs"]
mod hub_turns_tests;
