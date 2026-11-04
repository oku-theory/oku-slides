//! The macOS voice: `AVSpeechSynthesizer` through the Swift bridge, in safe calls only.
//!
//! One speech at a time, the last into the bridge wins (ruling R42), as in the Swift router: a new
//! speech, or [`stop`](SpeechEngine::stop) of the current id, cuts and releases the previous handle
//! outside the lock, so a DONE that runs inline cannot deadlock. Callers serialize `speak` (the
//! speaker does): two racing calls still give each speech one DONE, but may lose or overlap audio.
//!
//! The bridge's delegate fires on the main queue: in the app, whose main thread runs a run loop,
//! DONE comes when the speech ends; in a host without one (the CLI dev server), only a cut or the
//! next speech sends it, and the caller's watchdog must not wait for it. There, each speech also
//! leaves its undelivered delegate calls on the main queue: about 6 KiB per speech (1200 speeches:
//! +7 MiB), against a plateau with a run loop. The bridge shares one synthesizer for the process,
//! since one per speech kept about 60 KiB each.

use std::sync::{Mutex, PoisonError};

use super::apple_ffi::{self, VoiceHandle};
use super::{
    SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, chosen, for_language, install,
};
use crate::error::{SttError, VoiceUnavailableSnafu};

/// The macOS OS voice.
#[derive(Debug, Default)]
pub(crate) struct AppleSpeechEngine {
    /// The speech last started, until it is cut or replaced.
    current: Mutex<Option<(SpeechId, VoiceHandle)>>,
}

impl AppleSpeechEngine {
    /// Takes the current speech out if `keep` says so, under the lock only.
    fn take_current(&self, keep: impl FnOnce(SpeechId) -> bool) -> Option<VoiceHandle> {
        let mut g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if g.as_ref().is_some_and(|(id, _)| keep(*id)) { g.take().map(|(_, h)| h) } else { None }
    }
}

/// Cuts and releases `handle`; its DONE, if still due, runs here.
fn cut(handle: VoiceHandle) {
    handle.stop();
    handle.release();
}

impl SpeechEngine for AppleSpeechEngine {
    fn status(&self, lang: &str) -> SpeechStatus {
        let voices = apple_ffi::voices();
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
            cut(previous);
        }
        // The bridge refuses only an unknown voice (the text is always a valid C string).
        let handle = apple_ffi::speak(id, text, voice, rate.get(), done)
            .ok_or_else(|| VoiceUnavailableSnafu { reason: UnavailableReason::NoVoiceForLanguage }.build())?;
        let loser = install(&mut self.current.lock().unwrap_or_else(PoisonError::into_inner), id, handle);
        if let Some(loser) = loser {
            cut(loser);
        }
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        if let Some(handle) = self.take_current(|current| current == id) {
            cut(handle);
        }
        Ok(())
    }
}

impl Drop for AppleSpeechEngine {
    fn drop(&mut self) {
        if let Some(handle) = self.take_current(|_| true) {
            cut(handle);
        }
    }
}

#[cfg(test)]
#[path = "apple_tests.rs"]
mod apple_tests;
