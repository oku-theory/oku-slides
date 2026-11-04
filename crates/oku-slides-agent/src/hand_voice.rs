//! A test voice whose DONE the test calls by hand: French voices only.

use std::sync::{Mutex, MutexGuard, PoisonError};

use oku_slides_stt::{
    SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, SttError, UnavailableReason, VoiceInfo,
};

/// What the voice was asked, and the DONEs the test calls.
#[derive(Default)]
pub(crate) struct Calls {
    /// Each speech: its text and voice, in order.
    pub spoken: Vec<(String, String)>,
    /// DONEs not called yet.
    pub dones: Vec<(SpeechId, SpeechDone)>,
    /// Speeches stopped.
    pub stopped: Vec<SpeechId>,
}

/// A voice that speaks French (`v-fr` Amélie, `v-fr2` Thomas) and nothing else.
#[derive(Default)]
pub(crate) struct HandVoice {
    calls: Mutex<Calls>,
}

impl std::fmt::Debug for HandVoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HandVoice")
    }
}

impl HandVoice {
    pub(crate) fn calls(&self) -> MutexGuard<'_, Calls> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The texts spoken so far, in order.
    pub(crate) fn texts(&self) -> Vec<String> {
        self.calls().spoken.iter().map(|(t, _)| t.clone()).collect()
    }

    /// Calls the DONE of speech `id`, as the OS would, late or not.
    pub(crate) fn done(&self, id: u64) {
        let done = {
            let mut calls = self.calls();
            let at = calls.dones.iter().position(|(d, _)| d.get() == id).unwrap_or_else(|| panic!("no DONE {id}"));
            calls.dones.remove(at)
        };
        done.1.call(done.0);
    }

    /// Calls every DONE still held, as the OS would at the end of each speech.
    pub(crate) fn finish_all(&self) {
        let dones = std::mem::take(&mut self.calls().dones);
        for (id, done) in dones {
            done.call(id);
        }
    }
}

impl SpeechEngine for HandVoice {
    fn status(&self, lang: &str) -> SpeechStatus {
        if lang == "fr" {
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
        let mut calls = self.calls();
        calls.spoken.push((text.to_owned(), voice.unwrap_or_default().to_owned()));
        calls.dones.push((id, done));
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        self.calls().stopped.push(id);
        Ok(())
    }
}

/// A clock the test moves by hand.
#[derive(Debug)]
pub(crate) struct HandClock(Mutex<std::time::Instant>);

impl HandClock {
    pub(crate) fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self(Mutex::new(std::time::Instant::now())))
    }

    pub(crate) fn advance(&self, by: std::time::Duration) {
        let mut now = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *now = now.checked_add(by).unwrap_or_else(|| panic!("clock overflow"));
    }
}

impl oku_slides_stt::SpeakerClock for HandClock {
    fn now(&self) -> std::time::Instant {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
