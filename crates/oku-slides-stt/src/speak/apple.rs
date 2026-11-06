//! The macOS voice: `AVSpeechSynthesizer` through the Swift bridge, in safe calls only.
//!
//! A [`BridgedVoice`]: one speech at a time, the last into the bridge wins (ruling R42), as in
//! the Swift router; the cut releases the previous handle outside the lock (`bridged.rs`).
//!
//! The bridge's delegate fires on the main queue: in the app, whose main thread runs a run loop,
//! DONE comes when the speech ends; in a host without one (the CLI dev server), only a cut or the
//! next speech sends it, and the caller's watchdog must not wait for it. There, each speech also
//! leaves its undelivered delegate calls on the main queue: about 6 KiB per speech (1200 speeches:
//! +7 MiB), against a plateau with a run loop. The bridge shares one synthesizer for the process,
//! since one per speech kept about 60 KiB each.

use super::apple_ffi::{self, VoiceHandle};
use super::bridged::{BridgedVoice, VoiceBridge};
use super::{SpeechDone, SpeechId, VoiceInfo};

/// The macOS OS voice.
pub(crate) type AppleSpeechEngine = BridgedVoice<Synthesizer>;

/// The process's speech synthesizer, behind the Swift voice bridge.
#[derive(Debug, Default)]
pub(crate) struct Synthesizer;

impl VoiceBridge for Synthesizer {
    type Speech = VoiceHandle;

    fn voices(&self) -> Vec<VoiceInfo> {
        apple_ffi::voices()
    }

    fn speak(&self, id: SpeechId, text: &str, voice: &str, rate: f32, done: SpeechDone) -> Option<VoiceHandle> {
        apple_ffi::speak(id, text, voice, rate, done)
    }

    fn cut(&self, speech: VoiceHandle) {
        speech.stop();
        speech.release();
    }
}

#[cfg(test)]
#[path = "apple_tests.rs"]
mod apple_tests;
