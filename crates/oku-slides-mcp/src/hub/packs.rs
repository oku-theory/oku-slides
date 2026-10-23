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
use super::{McpEvent, PacksShown, Shown, SurfaceHub};

/// La vue du panneau ; `None` hors du mode deck.
fn view_in_deck_mode(s: &mut Session) -> oku_slides_session::Result<Option<DeckPacksView>> {
    if s.in_deck_mode() { s.deck_packs().map(Some) } else { Ok(None) }
}

impl SurfaceHub {
    /// Montre le panneau des packs du deck, seul ; `false` hors du mode deck.
    ///
    /// Lit la bibliothèque sous le verrou de session : hors des fils du runtime.
    pub fn show_packs(&self) -> Result<bool, HubError> {
        let view = self.session.with(view_in_deck_mode).and_then(|r| r).context(SessionSnafu)?;
        let Some(view) = view else { return Ok(false) };
        let surface = self.packs_surface(&view);
        let shown = Shown::Packs(PacksShown { last: surface.clone() });
        self.show_alone(surface, shown, |s| matches!(s, Shown::Packs(_)));
        Ok(true)
    }

    /// La surface du panneau, traduite dans la langue courante.
    fn packs_surface(&self, view: &DeckPacksView) -> Surface {
        deck_packs_panel(view, &self.localizer.current())
    }

    /// Applique une action du panneau, puis le refait.
    ///
    /// Un contexte mal formé est `Incomplete` ; la session revérifie tout le reste.
    pub(super) fn apply_packs(&self, context: &Map<String, Value>, name: &str) -> Result<PacksShown, HubInner> {
        let Some(event) = packs_event(name, context) else {
            tracing::warn!(action = name, "action du panneau des packs mal formée");
            return IncompleteSnafu { key: name }.fail();
        };
        self.session.with(|s| s.packs_action(event)).and_then(|r| r).context(SessionSnafu)?;
        self.emit(McpEvent::Changed);
        let view = self.session.with(|s| s.deck_packs()).and_then(|r| r).context(SessionSnafu)?;
        let surface = self.packs_surface(&view);
        self.emit(McpEvent::Surface { surface: surface.clone() });
        Ok(PacksShown { last: surface })
    }

    /// Ferme le panneau hors du mode deck ; le refait s'il a changé.
    ///
    /// Appelée par `prune`. Relit `state` après la session : un panneau fermé
    /// entre-temps n'est jamais rouvert.
    pub(super) fn refresh_packs(&self) {
        if !matches!(self.state().shows.get(HOST_PACKS), Some(Shown::Packs(_))) {
            return;
        }
        let view = match self.session.with(view_in_deck_mode).and_then(|r| r) {
            Ok(view) => view,
            Err(e) => {
                tracing::warn!(error = %e, "panneau des packs non rafraîchi : session illisible");
                return;
            }
        };
        let surface = view.as_ref().map(|v| self.packs_surface(v));
        let mut state = self.state();
        let Some(Shown::Packs(shown)) = state.shows.get(HOST_PACKS) else { return };
        match surface {
            None => {
                state.shows.remove(HOST_PACKS);
                self.record_and_send(&mut state, McpEvent::CloseSurface { id: HOST_PACKS.to_owned() });
            }
            Some(surface) if surface == shown.last => {}
            Some(surface) => {
                state.shows.insert(HOST_PACKS.to_owned(), Shown::Packs(PacksShown { last: surface.clone() }));
                self.record_and_send(&mut state, McpEvent::Surface { surface });
            }
        }
    }
}
