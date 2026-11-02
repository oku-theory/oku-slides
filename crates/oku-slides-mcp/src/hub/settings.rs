//! Boîte des réglages de la dictée et de l'agent : montrée, ses clics appliqués, rafraîchie.
//!
//! Un seul chemin d'envoi, sous le verrou d'état : données seules quand la disposition ne
//! change pas, la boîte entière sinon et après un refus.

use oku_slides_session::agent::Surface;
use oku_slides_session::{HOST_VOICE, VoicePanel, voice_event};
use serde_json::{Map, Value};
use snafu::IntoError;

use super::error::{HubError, IncompleteSnafu, VoiceSnafu};
use super::{McpEvent, SettingsShown, ShowOutcome, Shown, SurfaceHub};
use crate::Origin;

impl SurfaceHub {
    /// Shows the settings box, alone, in `turn` (an agent's chip; `None`: opened by the user).
    ///
    /// `false`, and nothing shown, when `turn` left the thread (the box has no mode). May block:
    /// the port reads the disk.
    pub fn show_settings(&self, turn: Option<u32>) -> bool {
        let panel = VoicePanel::build(&self.voice.view(), &self.localizer.current());
        let surface = Surface { turn, ..panel.surface() };
        let shown = Shown::Settings(SettingsShown { last: panel, turn });
        self.show_alone(surface, shown, |s| matches!(s, Shown::Settings(_))) == ShowOutcome::Shown
    }

    /// Applique un clic de la boîte montrée, puis envoie ce qui a changé.
    pub(super) fn apply_settings(&self, context: &Map<String, Value>, name: &str) -> Result<(), HubError> {
        if !matches!(self.state().shows.get(HOST_VOICE), Some(Shown::Settings(_))) {
            return Err(HubError::unknown_surface(HOST_VOICE.to_owned()));
        }
        let result = if let Some(event) = voice_event(name, context) {
            self.voice.apply(event, Origin::User).map(|_| ()).map_err(|e| VoiceSnafu.into_error(e))
        } else {
            tracing::warn!(action = name, "action de la boîte des réglages mal formée");
            Err(IncompleteSnafu { key: name }.build())
        };
        let panel = VoicePanel::build(&self.voice.view(), &self.localizer.current());
        self.send_settings(&panel, result.is_err());
        result.map_err(Into::into)
    }

    /// Refait la boîte montrée avec `panel`, si elle a changé.
    pub fn refresh_settings_with(&self, panel: &VoicePanel) {
        self.send_settings(panel, false);
    }

    /// Envoie `panel` à la place de la boîte montrée ; `whole` : toute la boîte (après un refus).
    ///
    /// Relit `state` sous son verrou : une boîte fermée entre-temps n'est jamais rouverte.
    fn send_settings(&self, panel: &VoicePanel, whole: bool) {
        let mut state = self.state();
        let Some(Shown::Settings(shown)) = state.shows.get(HOST_VOICE) else { return };
        if !whole && shown.last == *panel {
            return;
        }
        let turn = shown.turn;
        let event = if !whole && shown.last.same_layout(panel) {
            McpEvent::SurfacePatch { id: HOST_VOICE.to_owned(), messages: panel.data_messages() }
        } else {
            McpEvent::Surface { surface: Surface { turn, ..panel.surface() } }
        };
        state.shows.insert(HOST_VOICE.to_owned(), Shown::Settings(SettingsShown { last: panel.clone(), turn }));
        self.record_and_send(&mut state, event);
    }
}
