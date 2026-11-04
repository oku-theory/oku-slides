//! e2e fake voice: each speech is one line appended to a log file, then DONE at once.
//!
//! Compiled only with the `e2e-fakes` feature, which `just e2e` alone turns on (Q16). It
//! speaks `en` and `fr` (voices `fake-en` and `fake-fr`), nothing else, so the specs also see
//! an unavailable language. The log is written through [`Storage`]; DONE is called inside
//! [`speak`](SpeechEngine::speak), before it returns.

#[cfg(not(debug_assertions))]
compile_error!("the e2e-fakes feature is for debug e2e builds only; never build a release with it");

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use oku_slides_platform::storage::Storage;
use snafu::ResultExt;

use super::{
    SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo, chosen, for_language,
};
use crate::error::{SttError, VoiceLogSnafu, VoiceUnavailableSnafu};

/// A voice that writes each speech to a log instead of the speakers.
#[derive(Debug)]
pub struct FakeSpeechEngine {
    storage: Storage,
    log: PathBuf,
    /// Keeps each line whole when two speeches append at once.
    writing: Mutex<()>,
}

impl FakeSpeechEngine {
    /// A fake voice that appends what it says to `log`.
    #[must_use]
    pub fn new(storage: Storage, log: PathBuf) -> Self {
        Self { storage, log, writing: Mutex::new(()) }
    }

    fn voices() -> Vec<VoiceInfo> {
        ["en", "fr"]
            .into_iter()
            .map(|lang| VoiceInfo { id: format!("fake-{lang}"), name: format!("Fake {lang}"), lang: lang.to_owned() })
            .collect()
    }

    fn append(&self, line: &str) -> Result<(), SttError> {
        let _writing = self.writing.lock().unwrap_or_else(PoisonError::into_inner);
        let mut writer = self.storage.appender(&self.log).context(VoiceLogSnafu)?;
        writer.write_all(line.as_bytes()).context(VoiceLogSnafu)?;
        writer.close().context(VoiceLogSnafu)?;
        Ok(())
    }
}

impl SpeechEngine for FakeSpeechEngine {
    fn status(&self, lang: &str) -> SpeechStatus {
        for_language(lang, Self::voices())
    }

    fn speak(
        &self,
        id: SpeechId,
        text: &str,
        voice: Option<&str>,
        _rate: SpeechRate,
        done: SpeechDone,
    ) -> Result<(), SttError> {
        let voice = chosen(voice)?;
        if !Self::voices().iter().any(|known| known.id == voice) {
            return Err(VoiceUnavailableSnafu { reason: UnavailableReason::NoVoiceForLanguage }.build().into());
        }
        // One speech per line, whatever line breaks the text holds.
        let line: String = text.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
        self.append(&format!("{line}\n"))?;
        done.call(id);
        Ok(())
    }

    fn stop(&self, _id: SpeechId) -> Result<(), SttError> {
        // Each speech already ended inside `speak`.
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_tests.rs"]
mod fake_tests;
