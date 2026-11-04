//! OS voices that read Oku's answers aloud, one engine per OS.
//!
//! - macOS: `AVSpeechSynthesizer` through the Swift bridge (`apple.rs`, every `unsafe` block in
//!   `apple_ffi.rs`).
//! - Windows: the `WinRT` `SpeechSynthesizer` played by a `MediaPlayer` (`windows.rs`).
//! - Linux: `spd-say`, the Speech Dispatcher client (`linux.rs`).
//! - e2e: a voice that writes what it would say to a file (`fake.rs`, feature `e2e-fakes`).
//!
//! No voice is bundled, no cloud service called, no SSML passed: the text is spoken as plain
//! text. An engine with no voice for the language reports [`SpeechStatus::Unavailable`]; nothing
//! falls back to another language.

#[cfg(target_os = "macos")]
mod apple;
#[cfg(any(test, target_os = "macos"))]
mod apple_ffi;
#[cfg(feature = "e2e-fakes")]
pub(crate) mod fake;
#[cfg(any(test, target_os = "linux"))]
mod linux;
#[cfg(windows)]
mod windows;

use std::fmt;
use std::ops::RangeInclusive;
use std::sync::Arc;

use oku_slides_i18n::{Localize, Localizer};

use crate::error::{SttError, VoiceRateSnafu, VoiceUnavailableSnafu};

/// Implement this trait to give Oku a voice on one OS.
///
/// The caller allocates each [`SpeechId`] and records it as current *before* calling
/// [`speak`](SpeechEngine::speak): a `done` that fires before `speak` returns is still matched.
///
/// Every method blocks (it may ask the OS, run a program or wait for a callback): call them off
/// async workers. A `done` may run on the calling thread before [`speak`](SpeechEngine::speak)
/// or [`stop`](SpeechEngine::stop) returns, for the speech being cut or started (macOS releases
/// the cut speech inline): never hold a lock that a `done` takes while calling either.
pub trait SpeechEngine: Send + Sync + fmt::Debug {
    /// The voices that speak `lang`, or why none can.
    ///
    /// Blocking (it may ask the OS or run a program): call it off async workers.
    fn status(&self, lang: &str) -> SpeechStatus;

    /// Starts speaking `text` under `id`, cutting the speech in progress.
    ///
    /// `voice` must be a [`VoiceInfo::id`] from [`status`](SpeechEngine::status) for the
    /// language of `text`. `None` is refused with `NoVoiceForLanguage`: the OS default voice may
    /// speak another language. `done` is called once, with `id`, when the speech ends or is cut,
    /// from any thread, possibly before this returns. Cutting the previous speech may run its
    /// `done` here, on this thread. Blocking.
    ///
    /// # Errors
    ///
    /// [`SttErrorKind::VoiceUnavailable`](crate::SttErrorKind::VoiceUnavailable) when the
    /// engine or `voice` is missing, [`SttErrorKind::VoiceFailed`](crate::SttErrorKind::VoiceFailed)
    /// when the OS refuses. On `Err`, `done` is dropped uncalled.
    fn speak(
        &self,
        id: SpeechId,
        text: &str,
        voice: Option<&str>,
        rate: SpeechRate,
        done: SpeechDone,
    ) -> Result<(), SttError>;

    /// Cuts speech `id` if it is still playing; any other id does nothing.
    ///
    /// Its `done` may run here, on this thread, before this returns. Blocking.
    ///
    /// # Errors
    ///
    /// [`SttErrorKind::VoiceFailed`](crate::SttErrorKind::VoiceFailed) when the OS refuses.
    fn stop(&self, id: SpeechId) -> Result<(), SttError>;
}

/// Whether an engine can speak a language, and with which voices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechStatus {
    /// At least one voice speaks the language.
    Ready {
        /// The voices for the language, never empty.
        voices: Vec<VoiceInfo>,
    },
    /// No voice can speak the language.
    Unavailable {
        /// Why no voice can speak.
        reason: UnavailableReason,
    },
}

/// One OS voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceInfo {
    /// Stable identifier, passed back to [`SpeechEngine::speak`].
    pub id: String,
    /// Name shown to the user.
    pub name: String,
    /// BCP 47 language tag of the voice, as the OS gives it.
    pub lang: String,
}

/// Why no OS voice can speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    /// No speech engine on this system, or its program is missing.
    NoEngine,
    /// The engine has no voice for the language.
    NoVoiceForLanguage,
    /// The speech daemon does not answer.
    DaemonSilent,
}

impl Localize for UnavailableReason {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(match self {
            UnavailableReason::NoEngine => "stt-voice-no-engine",
            UnavailableReason::NoVoiceForLanguage => "stt-voice-no-voice",
            UnavailableReason::DaemonSilent => "stt-voice-daemon",
        })
    }
}

/// Speaking rate, finite, within 0.5..=2.0 of the OS default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechRate(f32);

impl SpeechRate {
    /// The accepted rates, as factors of the OS default rate.
    pub const RANGE: RangeInclusive<f32> = 0.5..=2.0;

    /// A rate of `r` times the OS default.
    ///
    /// # Errors
    ///
    /// [`SttErrorKind::VoiceFailed`](crate::SttErrorKind::VoiceFailed) when `r` is NaN,
    /// infinite or outside [`SpeechRate::RANGE`].
    pub fn new(r: f32) -> Result<Self, SttError> {
        if r.is_finite() && Self::RANGE.contains(&r) {
            Ok(Self(r))
        } else {
            Err(VoiceRateSnafu { rate: r }.build().into())
        }
    }

    /// The factor applied to the OS default rate.
    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}

impl Default for SpeechRate {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Called once, with the speech's generation id.
///
/// A late call for an older id is ignored, by the caller: it compares the id with its current speech.
pub struct SpeechDone(Box<dyn FnOnce(SpeechId) + Send>);

impl SpeechDone {
    /// Wraps the closure an engine calls when the speech ends or is cut.
    pub fn new(f: impl FnOnce(SpeechId) + Send + 'static) -> Self {
        Self(Box::new(f))
    }

    /// Reports that speech `id` ended or was cut.
    pub fn call(self, id: SpeechId) {
        (self.0)(id);
    }
}

impl fmt::Debug for SpeechDone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SpeechDone(..)")
    }
}

/// The generation of one speech, allocated by the caller.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SpeechId(u64);

impl SpeechId {
    /// The speech of generation `n`.
    #[must_use]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    /// The generation number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The engine for a system with no OS voice: always unavailable.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSpeechEngine;

impl SpeechEngine for NoSpeechEngine {
    fn status(&self, _lang: &str) -> SpeechStatus {
        SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine }
    }

    fn speak(&self, _: SpeechId, _: &str, _: Option<&str>, _: SpeechRate, _: SpeechDone) -> Result<(), SttError> {
        Err(VoiceUnavailableSnafu { reason: UnavailableReason::NoEngine }.build().into())
    }

    fn stop(&self, _id: SpeechId) -> Result<(), SttError> {
        Ok(())
    }
}

/// The OS voice of this system, or [`NoSpeechEngine`] where Oku has none.
#[must_use]
pub fn system_speech_engine() -> Arc<dyn SpeechEngine> {
    system()
}

#[cfg(target_os = "macos")]
fn system() -> Arc<dyn SpeechEngine> {
    Arc::new(apple::AppleSpeechEngine::default())
}

#[cfg(windows)]
fn system() -> Arc<dyn SpeechEngine> {
    Arc::new(windows::WindowsSpeechEngine::default())
}

#[cfg(target_os = "linux")]
fn system() -> Arc<dyn SpeechEngine> {
    // Looked up on `PATH`: a sandboxed build (AppImage, Flatpak) that cannot see it is unavailable.
    Arc::new(linux::SpdSay::new("spd-say".into()))
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
fn system() -> Arc<dyn SpeechEngine> {
    Arc::new(NoSpeechEngine)
}

/// Makes speech `id` current and returns the one it replaces, which the caller must cut.
///
/// The last speech installed wins, whatever the ids: it was the last into the bridge, which
/// speaks one at a time. Never drops a handle: the replaced one always comes back.
#[cfg(any(test, target_os = "macos", windows))]
pub(crate) fn install<H>(current: &mut Option<(SpeechId, H)>, id: SpeechId, handle: H) -> Option<H> {
    current.replace((id, handle)).map(|(_, replaced)| replaced)
}

/// The voice to speak with: `None` is refused, never the OS default voice.
pub(crate) fn chosen(voice: Option<&str>) -> Result<&str, SttError> {
    voice.ok_or_else(|| VoiceUnavailableSnafu { reason: UnavailableReason::NoVoiceForLanguage }.build().into())
}

/// Whether a voice tagged `voice_lang` speaks `lang`: the same primary language subtag.
///
/// `fr` matches `fr-FR` and `fr_CA`; `en` never matches `fr`. Region and script are ignored.
pub(crate) fn same_language(voice_lang: &str, lang: &str) -> bool {
    let primary = |tag: &str| tag.split(['-', '_']).next().unwrap_or_default().to_ascii_lowercase();
    let wanted = primary(lang);
    !wanted.is_empty() && primary(voice_lang) == wanted
}

/// The status for `lang` among an engine's `voices`: its voices, or no voice for the language.
pub(crate) fn for_language(lang: &str, voices: Vec<VoiceInfo>) -> SpeechStatus {
    let voices: Vec<VoiceInfo> = voices.into_iter().filter(|v| same_language(&v.lang, lang)).collect();
    if voices.is_empty() {
        SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage }
    } else {
        SpeechStatus::Ready { voices }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
