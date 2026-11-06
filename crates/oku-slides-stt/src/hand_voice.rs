//! A test voice and clock the test drives by hand: French voices, every language with `polyglot`.
//!
//! Also the listener states a speaker reads: listening through the duplex, the user speaking.

// A test helper: it panics on misuse, as an assertion would.
#![expect(clippy::panic, clippy::missing_panics_doc, reason = "test helper that panics on misuse")]

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::speak::{SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo};
use crate::speaker::SpeakerClock;
use crate::{State, Stt, SttError};

/// A clock the test moves by hand.
#[derive(Debug)]
pub struct HandClock(Mutex<Instant>);

impl HandClock {
    /// A clock that starts at the real now and then only moves by [`HandClock::advance`].
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(Instant::now())))
    }

    /// Moves the clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        let mut now = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *now = now.checked_add(by).unwrap_or_else(|| panic!("clock overflow"));
    }
}

impl SpeakerClock for HandClock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a [`HandEngine`] was asked, and the DONEs it holds for the test to call.
#[derive(Default)]
pub struct Calls {
    /// Each speech: its id, text and voice (empty when none was passed), in order.
    pub spoken: Vec<(SpeechId, String, String)>,
    /// DONEs not called yet.
    pub dones: Vec<(SpeechId, SpeechDone)>,
    /// Speeches stopped.
    pub stopped: Vec<SpeechId>,
    /// How many times the voices were listed.
    pub listed: usize,
}

impl fmt::Debug for Calls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Calls").field("spoken", &self.spoken).field("stopped", &self.stopped).finish_non_exhaustive()
    }
}

/// What the microphone hears of each speech.
pub type Echo = Box<dyn Fn(&str) + Send + Sync>;

/// A voice that speaks French (`v-fr` Amélie, `v-fr2` Thomas) and nothing else.
///
/// The test calls each DONE by hand, or the voice calls it inside `speak` (`inline`).
#[derive(Default)]
pub struct HandEngine {
    calls: Mutex<Calls>,
    /// Calls the DONE inside `speak`.
    inline: bool,
    /// Refuses every speech, as an OS voice that fails to start.
    refuse: bool,
    /// Called with each text it speaks, as a microphone would hear it.
    echo: Option<Echo>,
    /// Speaks every language with its French voices, not French alone.
    polyglot: bool,
}

impl fmt::Debug for HandEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HandEngine")
    }
}

impl HandEngine {
    /// A voice that calls each DONE inside `speak`.
    #[must_use]
    pub fn inline() -> Self {
        Self { inline: true, ..Self::default() }
    }

    /// A voice that refuses every speech, as an OS voice that fails to start.
    #[must_use]
    pub fn refusing() -> Self {
        Self { refuse: true, ..Self::default() }
    }

    /// A voice that hands each spoken text to `echo`, as a microphone would hear it.
    #[must_use]
    pub fn echoing(echo: Echo) -> Self {
        Self { echo: Some(echo), ..Self::default() }
    }

    /// A voice that speaks every language, with the same two voices.
    #[must_use]
    pub fn polyglot() -> Self {
        Self { polyglot: true, ..Self::default() }
    }

    /// What the voice was asked so far.
    pub fn calls(&self) -> MutexGuard<'_, Calls> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The texts spoken so far, in order.
    pub fn texts(&self) -> Vec<String> {
        self.calls().spoken.iter().map(|(_, text, _)| text.clone()).collect()
    }

    /// Calls the DONE of speech `id`, as the OS would, late or not.
    pub fn done(&self, id: u64) {
        let done = {
            let mut calls = self.calls();
            let at = calls.dones.iter().position(|(d, _)| d.get() == id).unwrap_or_else(|| panic!("no DONE {id}"));
            calls.dones.remove(at).1
        };
        done.call(SpeechId::new(id));
    }

    /// Calls every DONE still held, as the OS would at the end of each speech.
    pub fn finish_all(&self) {
        let dones = std::mem::take(&mut self.calls().dones);
        for (id, done) in dones {
            done.call(id);
        }
    }
}

impl SpeechEngine for HandEngine {
    fn status(&self, lang: &str) -> SpeechStatus {
        self.calls().listed += 1;
        if lang == "fr" || self.polyglot {
            let voice = |id: &str, name: &str| VoiceInfo { id: id.into(), name: name.into(), lang: "fr-FR".into() };
            SpeechStatus::Ready { voices: vec![voice("v-fr", "Amélie"), voice("v-fr2", "Thomas")] }
        } else {
            SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage }
        }
    }

    fn speak(
        &self,
        id: SpeechId,
        text: &str,
        voice: Option<&str>,
        _rate: SpeechRate,
        done: SpeechDone,
    ) -> Result<(), SttError> {
        if self.refuse {
            return Err(crate::error::VoiceFailedSnafu { step: "speak" }.build().into());
        }
        self.calls().spoken.push((id, text.to_owned(), voice.unwrap_or_default().to_owned()));
        if let Some(echo) = &self.echo {
            echo(text);
        }
        if self.inline {
            done.call(id);
        } else {
            self.calls().dones.push((id, done));
        }
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        self.calls().stopped.push(id);
        Ok(())
    }
}

/// Puts `stt` in hands-free listening through the duplex voice `voice`, barge-in on.
pub fn listen_through_duplex(stt: &Stt, voice: Arc<dyn SpeechEngine>) {
    let mut g = stt.state();
    g.state = State::Listening;
    g.barge = Some(voice);
    g.config.speech.barge_in = true;
}

/// Marks `stt` as listening, the user speaking (a phrase open) or not.
pub fn user_speaks(stt: &Stt, speaking: bool) {
    let mut g = stt.state();
    g.state = State::Listening;
    g.speaking = speaking;
    g.phrase_open = speaking;
}
