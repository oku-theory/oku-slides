//! Barge-in: the user talks over Oku to cut its speech, where the duplex cancels Oku's echo.
//!
//! Hands-free listening opens the duplex ([`Duplex`], macOS) instead of the plain microphone
//! when the `speech.barge_in` setting is on: its voice then reads Oku's speeches
//! ([`Stt::barge_voice`]), so the echo canceller removes them from what the microphone hears,
//! and the [`Speaker`](crate::Speaker) keeps listening while it speaks. When the duplex does
//! not start (no main run loop in the CLI dev server, no echo cancellation), listening goes on
//! through the plain microphone, muted while Oku speaks, and [`Stt::barge_in_unavailable`] says
//! why. Elsewhere than macOS it is always unavailable.

use std::sync::Arc;

use oku_slides_i18n::{Localize, Localizer};

use crate::audio::Recorder;
use crate::{Duplex, Mode, SpeechEngine, State, Stt, SttError};

impl Stt {
    /// Why talking over Oku cannot cut its speech here; `None` when it can.
    ///
    /// Off macOS, always a reason; on macOS, the duplex's last start failure, if any.
    #[must_use]
    pub fn barge_in_unavailable(&self, l: &Localizer) -> Option<String> {
        if !Duplex::available() {
            return Some(l.tr("speech-barge-in-unavailable-os"));
        }
        self.state().barge_failure.as_ref().map(|e| e.localize(l))
    }

    /// The duplex voice, while listening goes through the duplex with barge-in on.
    pub(crate) fn barge_voice(&self) -> Option<Arc<dyn SpeechEngine>> {
        let g = self.state();
        if g.state == State::Listening && g.config.speech.barge_in { g.barge.clone() } else { None }
    }

    /// The microphone of a capture in `mode`, with the duplex voice when it is the duplex.
    ///
    /// A duplex that does not start is reported by [`Stt::barge_in_unavailable`]; the plain
    /// microphone serves instead.
    pub(crate) fn open_microphone(&self, mode: Mode) -> Result<(Recorder, Option<Arc<dyn SpeechEngine>>), SttError> {
        let barge_in = self.state().config.speech.barge_in;
        if wants_duplex(mode, barge_in, self.settings.fake_mic.is_some(), Duplex::available()) {
            match start_duplex() {
                Ok((recorder, voice)) => {
                    self.state().barge_failure = None;
                    return Ok((recorder, Some(voice)));
                }
                Err(e) => {
                    tracing::warn!(
                        error = %oku_slides_platform::errors::one_line(&e),
                        "barge-in unavailable: listening is muted while Oku speaks"
                    );
                    self.state().barge_failure = Some(Arc::new(e));
                }
            }
        }
        let recorder = Recorder::start(self.settings.fake_mic.as_deref(), &self.storage)?;
        Ok((recorder, None))
    }
}

/// Whether a capture in `mode` goes through the duplex: listening, barge-in on, a real microphone.
pub(crate) fn wants_duplex(mode: Mode, barge_in: bool, fake_mic: bool, available: bool) -> bool {
    mode == Mode::Listen && barge_in && !fake_mic && available
}

/// Starts the duplex: its recorder and its voice, the one made for this engine.
fn start_duplex() -> Result<(Recorder, Arc<dyn SpeechEngine>), SttError> {
    let duplex = Duplex::start()?;
    let voice: Arc<dyn SpeechEngine> = Arc::new(duplex.voice());
    Ok((Recorder::from(duplex), voice))
}

#[cfg(test)]
#[path = "barge_in_tests.rs"]
mod barge_in_tests;
