//! A voice spoken through a Swift bridge, one speech at a time, the last one in wins.
//!
//! Shared by the two macOS voices: the synthesizer alone (`apple.rs`) and the duplex audio engine
//! (`crate::duplex`). One speech at a time, the last into the bridge wins (ruling R42), as in the
//! Swift side: a new speech, or [`stop`](SpeechEngine::stop) of the current id, cuts and releases
//! the previous speech outside the lock, so a DONE that runs inline cannot deadlock. Callers
//! serialize `speak` (the speaker does): two racing calls still give each speech one DONE, but
//! may lose or overlap audio.

use std::fmt;
use std::sync::{Mutex, PoisonError};

use super::{
    SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo, chosen, for_language,
    install,
};
use crate::error::{SttError, VoiceUnavailableSnafu};

/// The Swift side of a bridged voice: list, speak, cut.
pub(crate) trait VoiceBridge: Send + Sync + fmt::Debug {
    /// A speech the bridge accepted, until it is cut.
    type Speech: Send + fmt::Debug;

    /// Every voice of the system.
    fn voices(&self) -> Vec<VoiceInfo>;

    /// Starts `text` with `voice`, or `None` when the bridge refuses (`done` dropped uncalled).
    ///
    /// Calls `done` once, with `id`, when the speech ends or is cut.
    fn speak(&self, id: SpeechId, text: &str, voice: &str, rate: f32, done: SpeechDone) -> Option<Self::Speech>;

    /// Cuts and releases `speech`; its DONE, if still due, runs here.
    fn cut(&self, speech: Self::Speech);
}

/// A [`SpeechEngine`] over a [`VoiceBridge`]: one current speech, cut by the next.
#[derive(Debug)]
pub(crate) struct BridgedVoice<B: VoiceBridge> {
    bridge: B,
    /// The speech last started, until it is cut or replaced.
    current: Mutex<Option<(SpeechId, B::Speech)>>,
}

impl<B: VoiceBridge> BridgedVoice<B> {
    /// A voice speaking through `bridge`, silent so far.
    pub(crate) fn new(bridge: B) -> Self {
        Self { bridge, current: Mutex::new(None) }
    }

    /// Takes the current speech out if `keep` says so, under the lock only.
    fn take_current(&self, keep: impl FnOnce(SpeechId) -> bool) -> Option<B::Speech> {
        let mut g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if g.as_ref().is_some_and(|(id, _)| keep(*id)) { g.take().map(|(_, s)| s) } else { None }
    }
}

impl<B: VoiceBridge + Default> Default for BridgedVoice<B> {
    fn default() -> Self {
        Self::new(B::default())
    }
}

impl<B: VoiceBridge> SpeechEngine for BridgedVoice<B> {
    fn status(&self, lang: &str) -> SpeechStatus {
        let voices = self.bridge.voices();
        if voices.is_empty() {
            return SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine };
        }
        for_language(lang, voices)
    }

    fn speak(
        &self,
        id: SpeechId,
        text: &str,
        voice: Option<&str>,
        rate: SpeechRate,
        done: SpeechDone,
    ) -> Result<(), SttError> {
        let voice = chosen(voice)?;
        if let Some(previous) = self.take_current(|_| true) {
            self.bridge.cut(previous);
        }
        // The bridges refuse only an unknown voice (the text is always a valid C string).
        let speech = self
            .bridge
            .speak(id, text, voice, rate.get(), done)
            .ok_or_else(|| VoiceUnavailableSnafu { reason: UnavailableReason::NoVoiceForLanguage }.build())?;
        let loser = install(&mut self.current.lock().unwrap_or_else(PoisonError::into_inner), id, speech);
        if let Some(loser) = loser {
            self.bridge.cut(loser);
        }
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        if let Some(speech) = self.take_current(|current| current == id) {
            self.bridge.cut(speech);
        }
        Ok(())
    }
}

impl<B: VoiceBridge> Drop for BridgedVoice<B> {
    fn drop(&mut self) {
        if let Some(speech) = self.take_current(|_| true) {
            self.bridge.cut(speech);
        }
    }
}

#[cfg(test)]
#[path = "bridged_tests.rs"]
mod bridged_tests;
