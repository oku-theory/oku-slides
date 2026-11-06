//! Duplex audio on macOS: one voice-processing engine hears the user and speaks for Oku.
//!
//! One `AVAudioEngine` whose input node runs Apple's voice processing (echo cancellation) both
//! captures the microphone and plays Oku's synthesized speech (`OkuDuplex.swift`): since the
//! speech goes out through the same unit, the canceller has it as reference and removes it from
//! what the microphone hears. The input tap gives mono `Float32` frames at the device rate to the
//! [`Tap`], which feeds the [`Recorder`] made from the duplex, the way cpal feeds the microphone
//! one; the voice ([`Duplex::voice`]) keeps the [`SpeechEngine`] contract of `speak/apple.rs`.
//! Every `unsafe` block is in `duplex_ffi.rs`. The synthesizer delivers its buffers on the main
//! queue: the engine starts only in a host whose main thread runs a run loop (the app, not the CLI
//! dev server). Elsewhere than macOS [`Duplex::available`] is false and [`Duplex::start`] fails.

#[cfg(any(test, target_os = "macos"))]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(any(test, target_os = "macos"))]
use std::sync::{Arc, Mutex, PoisonError};

use crate::audio::Recorder;
#[cfg(any(test, target_os = "macos"))]
use crate::audio::{self, Captured};
#[cfg(any(test, target_os = "macos"))]
use crate::error::{DuplexStartSnafu, NoMicrophoneSnafu};
use crate::error::{EchoCancelSnafu, SttError};
use crate::speak::SpeechEngine;

/// The failure codes of `oku_duplex_start`, the same in `OkuDuplex.swift`.
///
/// Any other code is a `DuplexStart` error: 3, the engine does not start; 4, no main run loop
/// drains the main queue, where the synthesizer delivers its buffers (the CLI dev server).
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod raw {
    /// Voice processing (echo cancellation) could not be turned on.
    pub(crate) const ECHO_CANCEL: i32 = 1;
    /// The input node has no channel or no rate: no microphone.
    pub(crate) const NO_INPUT: i32 = 2;
}

/// The error for the bridge's failure `code`.
#[cfg(any(test, target_os = "macos"))]
pub(crate) fn failure(code: i32) -> SttError {
    match code {
        raw::ECHO_CANCEL => EchoCancelSnafu.build().into(),
        raw::NO_INPUT => NoMicrophoneSnafu.build().into(),
        _ => DuplexStartSnafu { code }.build().into(),
    }
}

/// Where the duplex's microphone frames go: the attached recorder's samples, or nowhere.
#[cfg(any(test, target_os = "macos"))]
#[derive(Debug, Default)]
pub(crate) struct Tap {
    sink: Mutex<Option<Arc<Mutex<Captured>>>>,
    /// The last frames' level (`f32` bits), attached or not.
    level: AtomicU32,
}

#[cfg(any(test, target_os = "macos"))]
impl Tap {
    /// Pushes `frames`, mono at `rate` hertz, to the attached recorder; dropped without one.
    pub(crate) fn push(&self, frames: &[f32], rate: u32) {
        if !frames.is_empty() {
            self.level.store(audio::level_of(frames).to_bits(), Ordering::Relaxed);
        }
        let sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(captured) = sink.as_ref() {
            audio::push_frames(captured, frames, rate);
        }
    }

    /// The level of the last frames, 0..1: what the microphone hears once echo-cancelled.
    pub(crate) fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// Attaches fresh samples, replacing any earlier ones, until the [`Attached`] guard drops.
    pub(crate) fn attach(self: &Arc<Self>) -> (Arc<Mutex<Captured>>, Attached) {
        let captured = Arc::new(Mutex::new(Captured::default()));
        *self.sink.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&captured));
        (Arc::clone(&captured), Attached { tap: Arc::clone(self), captured })
    }
}

/// Keeps a recorder's samples attached to the tap; detaches them when dropped.
#[cfg(any(test, target_os = "macos"))]
#[derive(Debug)]
pub(crate) struct Attached {
    tap: Arc<Tap>,
    captured: Arc<Mutex<Captured>>,
}

#[cfg(any(test, target_os = "macos"))]
impl Drop for Attached {
    fn drop(&mut self) {
        let mut sink = self.tap.sink.lock().unwrap_or_else(PoisonError::into_inner);
        // A later recorder may have replaced these samples: leave its own attached.
        if sink.as_ref().is_some_and(|s| Arc::ptr_eq(s, &self.captured)) {
            *sink = None;
        }
    }
}

/// The macOS duplex audio engine: echo-cancelled microphone and Oku's voice on one unit.
///
/// Cheap to clone: every clone names the same engine, which stops when the last one drops.
/// Make one recorder at a time from it (`Recorder::from`) and one voice ([`Duplex::voice`]).
#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct Duplex {
    tap: Arc<Tap>,
    engine: Arc<crate::duplex_ffi::EngineHandle>,
}

/// The duplex audio engine, macOS only: never built on this system.
#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone)]
pub struct Duplex {
    never: std::convert::Infallible,
}

impl Duplex {
    /// Whether this system has the duplex audio engine: macOS only.
    #[must_use]
    pub fn available() -> bool {
        cfg!(target_os = "macos")
    }

    /// Starts the engine: microphone with echo cancellation, and a player for the voice.
    ///
    /// The microphone permission is asked by the system on first use, as for cpal.
    ///
    /// # Errors
    ///
    /// [`SttErrorKind::Microphone`](crate::SttErrorKind::Microphone) when echo cancellation
    /// cannot be turned on, when there is no microphone, when the engine does not start, or when
    /// no main run loop would deliver the speech.
    #[cfg(target_os = "macos")]
    pub fn start() -> Result<Self, SttError> {
        let tap = Arc::new(Tap::default());
        let engine = crate::duplex_ffi::start(Arc::clone(&tap)).map_err(failure)?;
        Ok(Self { tap, engine: Arc::new(engine) })
    }

    /// Starts the engine: off macOS, always refused.
    ///
    /// # Errors
    ///
    /// Always [`SttErrorKind::Microphone`](crate::SttErrorKind::Microphone): no echo
    /// cancellation on this system.
    #[cfg(not(target_os = "macos"))]
    pub fn start() -> Result<Self, SttError> {
        Err(EchoCancelSnafu.build().into())
    }

    /// The microphone's recent level, 0..1, once echo-cancelled; 0 before the first frames.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn level(&self) -> f32 {
        self.tap.level()
    }

    /// The microphone's recent level: never reached off macOS, where no engine exists.
    #[cfg(not(target_os = "macos"))]
    #[must_use]
    pub fn level(&self) -> f32 {
        match self.never {}
    }

    /// Oku's voice played by this engine, so the canceller hears it; make one per engine.
    ///
    /// Same contract as the macOS OS voice: one speech at a time, one DONE per id, the last
    /// buffer played (or the cut) sending it.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn voice(&self) -> impl SpeechEngine + use<> {
        DuplexVoice::new(DuplexBridge { engine: Arc::clone(&self.engine) })
    }

    /// Oku's voice played by this engine: never reached off macOS, where no engine exists.
    #[cfg(not(target_os = "macos"))]
    #[must_use]
    pub fn voice(&self) -> impl SpeechEngine + use<> {
        crate::speak::NoSpeechEngine
    }
}

#[cfg(target_os = "macos")]
impl From<Duplex> for Recorder {
    /// A recorder fed by the duplex's echo-cancelled microphone, until it finishes.
    fn from(duplex: Duplex) -> Self {
        let (captured, attached) = duplex.tap.attach();
        Recorder::fed(captured, (attached, duplex))
    }
}

#[cfg(not(target_os = "macos"))]
impl From<Duplex> for Recorder {
    fn from(duplex: Duplex) -> Self {
        match duplex.never {}
    }
}

/// The duplex engine's voice.
#[cfg(target_os = "macos")]
pub(crate) type DuplexVoice = crate::speak::bridged::BridgedVoice<DuplexBridge>;

/// The duplex engine seen as a voice bridge: its player speaks, the system lists the voices.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct DuplexBridge {
    engine: Arc<crate::duplex_ffi::EngineHandle>,
}

#[cfg(target_os = "macos")]
impl crate::speak::bridged::VoiceBridge for DuplexBridge {
    type Speech = crate::duplex_ffi::SpeechHandle;

    fn voices(&self) -> Vec<crate::speak::VoiceInfo> {
        crate::speak::apple_ffi::voices()
    }

    fn speak(
        &self,
        id: crate::speak::SpeechId,
        text: &str,
        voice: &str,
        rate: f32,
        done: crate::speak::SpeechDone,
    ) -> Option<Self::Speech> {
        crate::duplex_ffi::speak(&self.engine, id, text, voice, rate, done)
    }

    fn cut(&self, speech: Self::Speech) {
        speech.stop();
        speech.release();
    }
}

#[cfg(test)]
#[path = "duplex_tests.rs"]
mod duplex_tests;
