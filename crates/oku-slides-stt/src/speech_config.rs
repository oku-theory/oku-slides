//! The read-aloud settings, saved in `config.json` with the dictation's.
//!
//! They change without touching the capture, unlike [`Stt::configure`], except the barge-in
//! switch: listening restarts on the duplex microphone or the plain one.

use serde::{Deserialize, Serialize};

use crate::{CONFIG_VERSION, State, Stt};

/// The read-aloud settings besides the switch, saved with the dictation's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechConfig {
    /// The chosen OS voice id; `None`: the first voice of the interface language.
    pub voice: Option<String>,
    /// Speaking rate, a factor of the OS default within [`SpeechRate::RANGE`].
    ///
    /// [`SpeechRate::RANGE`]: crate::SpeechRate::RANGE
    pub rate: f32,
    /// Read the answer to a wake-phrase request while the tab presents.
    pub in_presentation: bool,
    /// Talking over Oku cuts its speech, where echo cancellation allows it (macOS).
    ///
    /// Elsewhere, or off, listening is muted while Oku speaks.
    pub barge_in: bool,
}

impl Default for SpeechConfig {
    fn default() -> Self {
        Self { voice: None, rate: 1.0, in_presentation: false, barge_in: true }
    }
}

impl Stt {
    /// The read-aloud switch (`None` until set) and its voice, rate and presentation rule.
    #[must_use]
    pub fn speech_settings(&self) -> (Option<bool>, SpeechConfig) {
        let g = self.state();
        (g.config.read_aloud, g.config.speech.clone())
    }

    /// Applies and saves the read-aloud settings, in any dictation state.
    ///
    /// Listening goes on; a change of [`SpeechConfig::barge_in`] restarts its capture, which
    /// then opens the duplex microphone or the plain one. Turning it off forgets a failed
    /// duplex start, so the settings box offers the switch again.
    pub fn configure_speech(&self, read_aloud: Option<bool>, speech: SpeechConfig) {
        let restart = {
            let mut g = self.state();
            let mut config = g.config.clone();
            let barge_in_changed = config.speech.barge_in != speech.barge_in;
            config.read_aloud = read_aloud;
            config.speech = speech;
            config.version = CONFIG_VERSION;
            self.save(&config);
            if !config.speech.barge_in {
                // Off, a failed duplex start no longer matters: the box shows the switch again.
                g.barge_failure = None;
            }
            g.config = config;
            (barge_in_changed && g.state == State::Listening).then(|| {
                g.restart_listening();
                g.vocabulary.clone()
            })
        };
        if let Some(vocabulary) = restart {
            // Still `Listening` meanwhile, so a Start in between is refused.
            self.launch(vocabulary, crate::Mode::Listen, State::Listening);
        }
    }
}

#[cfg(test)]
#[path = "speech_config_tests.rs"]
mod speech_config_tests;
