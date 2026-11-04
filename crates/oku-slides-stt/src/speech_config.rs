//! The read-aloud settings, saved in `config.json` with the dictation's.
//!
//! They change without touching the capture: listening goes on, unlike [`Stt::configure`].

use crate::{CONFIG_VERSION, SpeechConfig, Stt};

impl Stt {
    /// The read-aloud switch (`None` until set) and its voice, rate and presentation rule.
    #[must_use]
    pub fn speech_settings(&self) -> (Option<bool>, SpeechConfig) {
        let g = self.state();
        (g.config.read_aloud, g.config.speech.clone())
    }

    /// Applies and saves the read-aloud settings, in any dictation state; listening goes on.
    pub fn configure_speech(&self, read_aloud: Option<bool>, speech: SpeechConfig) {
        let mut g = self.state();
        let mut config = g.config.clone();
        config.read_aloud = read_aloud;
        config.speech = speech;
        config.version = CONFIG_VERSION;
        self.save(&config);
        g.config = config;
    }
}

#[cfg(test)]
#[path = "speech_config_tests.rs"]
mod speech_config_tests;
