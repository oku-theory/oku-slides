//! Panneau des packs du deck : montré, ses actions appliquées, rafraîchi.
//!
//! La session donne une vue sans texte (`deck_packs`) ; le hub la traduit à
//! chaque construction, dans la langue courante. `prune` la relit et ne
//! renvoie le panneau que s'il a changé.

use oku_slides_session::agent::Surface;
use oku_slides_session::{DeckPacksView, HOST_PACKS, Session, deck_packs_panel, packs_event};
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::error::{HubError, HubInner, IncompleteSnafu, SessionSnafu};
use super::{McpEvent, PacksShown, ShowOutcome, Shown, SurfaceHub};

/// La vue du panneau ; `None` hors du mode deck.
fn view_in_deck_mode(s: &mut Session) -> oku_slides_session::Result<Option<DeckPacksView>> {
    if s.in_deck_mode() { s.deck_packs().map(Some) } else { Ok(None) }
}

impl SurfaceHub {
    /// Shows the deck's packs panel, alone, in `turn` (an agent's chip; `None`: opened by the user).
    ///
    /// `OutOfMode` outside deck mode, `TurnOver` when `turn` left the thread. Reads the library under the
    /// session lock: call it off the runtime threads.
    ///
    /// # Errors
    ///
    /// Fails if the session refuses: tab closed or packs library unreadable.
    pub fn show_packs(&self, turn: Option<u32>) -> Result<ShowOutcome, HubError> {
        let view = self.session.with(view_in_deck_mode).and_then(|r| r).context(SessionSnafu)?;
        let Some(view) = view else { return Ok(ShowOutcome::OutOfMode) };
        let surface = self.packs_surface(&view, turn);
        let shown = Shown::Packs(PacksShown { last: surface.clone(), turn });
        Ok(self.show_alone(surface, shown, |s| matches!(s, Shown::Packs(_))))
    }

    /// The panel's surface in `turn`, translated into the current language.
    fn packs_surface(&self, view: &DeckPacksView, turn: Option<u32>) -> Surface {
        Surface { turn, ..deck_packs_panel(view, &self.localizer.current()) }
    }

    /// Applies an action of the panel, then rebuilds it in its `turn`.
    ///
    /// A malformed context is `Incomplete`; the session checks everything else again.
    pub(super) fn apply_packs(
        &self,
        context: &Map<String, Value>,
        name: &str,
        turn: Option<u32>,
    ) -> Result<PacksShown, HubInner> {
        let Some(event) = packs_event(name, context) else {
            tracing::warn!(action = name, "action du panneau des packs mal formée");
            return IncompleteSnafu { key: name }.fail();
        };
        self.session.with(|s| s.packs_action(event)).and_then(|r| r).context(SessionSnafu)?;
        self.emit(McpEvent::Changed);
        let view = self.session.with(|s| s.deck_packs()).and_then(|r| r).context(SessionSnafu)?;
        let surface = self.packs_surface(&view, turn);
        self.emit(McpEvent::surface(surface.clone()));
        Ok(PacksShown { last: surface, turn })
    }

    /// Ferme le panneau hors du mode deck ; le refait s'il a changé.
    ///
    /// Appelée par `prune`. Relit `state` après la session : un panneau fermé
    /// entre-temps n'est jamais rouvert.
    pub(super) fn refresh_packs(&self) {
        let turn = match self.state().shows.get(HOST_PACKS) {
            Some(Shown::Packs(shown)) => shown.turn,
            _ => return,
        };
        let view = match self.session.with(view_in_deck_mode).and_then(|r| r) {
            Ok(view) => view,
            Err(e) => {
                tracing::warn!(error = %e, "panneau des packs non rafraîchi : session illisible");
                return;
            }
        };
        let surface = view.as_ref().map(|v| self.packs_surface(v, turn));
        let mut state = self.state();
        // Shown again meanwhile (another turn): that show sent its own panel.
        let Some(Shown::Packs(shown)) = state.shows.get(HOST_PACKS) else { return };
        if shown.turn != turn {
            return;
        }
        match surface {
            None => {
                state.shows.remove(HOST_PACKS);
                self.record_and_send(&mut state, McpEvent::CloseSurface { id: HOST_PACKS.to_owned() });
            }
            Some(surface) if surface == shown.last => {}
            Some(surface) => {
                state.shows.insert(HOST_PACKS.to_owned(), Shown::Packs(PacksShown { last: surface.clone(), turn }));
                self.record_and_send(&mut state, McpEvent::surface(surface));
            }
        }
    }
}
