//! Sélecteur de thème de l'hôte : montré, choisi, rafraîchi.
//!
//! Le seul `show_*` que le hub construit lui-même : il le refait après un choix ou
//! un changement du deck (`prune`), à partir des cartes gardées, sans rien rendre.

use oku_slides_session::{HOST_THEME_PICKER, picked_theme_id, theme_picker};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use super::error::{HubError, HubInner, IncompleteSnafu, SessionSnafu};
use super::{McpEvent, Shown, SurfaceHub, ThemePicker};

impl SurfaceHub {
    /// Montre le sélecteur de thème sur `slide` ; `false` hors du mode deck.
    ///
    /// Rend les miniatures sous le verrou de session : hors des fils du runtime.
    pub fn show_themes(&self, slide: Option<String>) -> Result<bool, HubError> {
        let choices = self.session.with(|s| s.theme_choices(slide.as_deref())).and_then(|r| r).context(SessionSnafu)?;
        let Some(choices) = choices else { return Ok(false) };
        let surface = theme_picker(&choices.cards, choices.active, &self.localizer.current());
        let picker = ThemePicker { cards: choices.cards, active: choices.active };
        self.show_alone(surface, Shown::Themes(picker), |s| matches!(s, Shown::Themes(_)));
        Ok(true)
    }

    /// Applique le thème choisi, puis refait le sélecteur avec lui actif.
    pub(super) fn apply_theme(
        &self,
        picker: &ThemePicker,
        context: &Map<String, Value>,
    ) -> Result<ThemePicker, HubInner> {
        let id = picked_theme_id(context).context(IncompleteSnafu { key: oku_slides_session::THEME_EVENT })?;
        self.session.with(|s| s.pick_theme(id)).and_then(|r| r).context(SessionSnafu)?;
        self.emit(McpEvent::Changed);
        let next = ThemePicker { cards: picker.cards.clone(), active: id };
        self.emit(McpEvent::Surface { surface: theme_picker(&next.cards, id, &self.localizer.current()) });
        Ok(next)
    }

    /// Ferme le sélecteur hors du mode deck ou si sa slide a disparu ; le refait si l'actif a changé.
    ///
    /// Appelée par `prune`. Relit `state` après la session : un sélecteur fermé
    /// entre-temps n'est jamais rouvert.
    pub(super) fn refresh_themes(&self) {
        let shown = matches!(self.state().shows.get(HOST_THEME_PICKER), Some(Shown::Themes(_)));
        if !shown {
            return;
        }
        // Les cartes montrent toutes une même slide. Elle a disparu (autre deck ouvert, slide
        // retirée) si l'id n'existe plus, ou si l'id est vide (deck vide) et que le deck a des slides.
        let seen = self.state().shows.get(HOST_THEME_PICKER).and_then(|s| match s {
            Shown::Themes(p) => p.cards.first().map(|c| c.slide.clone()),
            _ => None,
        });
        let read = self.session.with(|s| {
            let deck = s.deck();
            let gone =
                seen.is_some_and(|id| if id.is_empty() { !deck.slides.is_empty() } else { deck.slide(&id).is_none() });
            (s.in_deck_mode(), s.active_theme(), gone)
        });
        let (deck_mode, active, gone) = match read {
            Ok(read) => read,
            Err(e) => {
                tracing::warn!(error = %e, "sélecteur de thème non rafraîchi : session illisible");
                return;
            }
        };
        let mut state = self.state();
        let Some(Shown::Themes(picker)) = state.shows.get(HOST_THEME_PICKER) else { return };
        let picker = picker.clone();
        if !deck_mode || gone {
            state.shows.remove(HOST_THEME_PICKER);
            self.record_and_send(&mut state, McpEvent::CloseSurface { id: HOST_THEME_PICKER.to_owned() });
            return;
        }
        if active == picker.active {
            return;
        }
        let surface = theme_picker(&picker.cards, active, &self.localizer.current());
        state.shows.insert(HOST_THEME_PICKER.to_owned(), Shown::Themes(ThemePicker { active, ..picker }));
        self.record_and_send(&mut state, McpEvent::Surface { surface });
    }
}
