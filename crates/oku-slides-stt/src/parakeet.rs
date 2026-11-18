//! Parakeet TDT 0.6B v3 on ONNX Runtime, on the CPU, through parakeet-rs.
//!
//! parakeet-rs reads the model files by path, not through `Storage`: a documented deviation
//! (spec:2026-11-18-dictee-parakeet-design). Download and deletion still go through `Storage`.
//!
//! The model takes no language, no context and no quick mode: it detects the language itself,
//! and a preview costs the same as a final phrase.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Mutex, OnceLock, PoisonError};

use ort::environment::Environment;
use parakeet_rs::Transcriber as _;
use parakeet_rs::{ExecutionConfig, ParakeetTDT};
use snafu::ResultExt;

use crate::error::{BoxError, ModelLoadSnafu, TranscribeSnafu, boxed};
use crate::phrase_engine::PhraseEngine;
use crate::{SAMPLE_RATE, SttError, Transcript};

/// Shortest audio worth transcribing: 0.1 s at 16 kHz.
const MIN_SAMPLES: usize = 1600;

/// Parakeet loaded in memory, ready to transcribe.
pub(crate) struct Parakeet {
    /// The library transcribes through `&mut self`. The single transcription thread serializes
    /// final, flush and preview jobs, so this lock is never contended.
    inner: Mutex<ParakeetTDT>,
}

impl Parakeet {
    /// Loads the model from `dir` (`encoder-model.int8.onnx`, `decoder_joint-model.int8.onnx`,
    /// `vocab.txt`).
    pub(crate) fn load(dir: &Path) -> Result<Parakeet, SttError> {
        let config = ExecutionConfig::new()
            .with_intra_threads(threads())
            .with_environment(environment().context(ModelLoadSnafu { dir })?);
        let model = ParakeetTDT::from_pretrained(dir, Some(config)).map_err(boxed).context(ModelLoadSnafu { dir })?;
        Ok(Parakeet { inner: Mutex::new(model) })
    }
}

impl PhraseEngine for Parakeet {
    /// `language`, `context` and `quick` are ignored: see the module documentation.
    fn transcribe(
        &self,
        audio: &[f32],
        _language: Option<&str>,
        _context: Option<&str>,
        _quick: bool,
    ) -> Result<Transcript, SttError> {
        if too_short(audio) {
            return Ok(Transcript::default());
        }
        let text = guarded(|| {
            // A panic while transcribing may leave the model's feature cache stale; the
            // next phrase rebuilds it, so the poisoned lock is recovered.
            let mut model = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            let result = model.transcribe_samples(audio.to_vec(), SAMPLE_RATE, 1, None).map_err(boxed)?;
            Ok(result.text)
        })?;
        Ok(Transcript { text: text.trim().to_owned(), language: None, no_speech: 0.0 })
    }
}

/// True if `audio` is too short to hold a word: the model is not even asked.
pub(super) fn too_short(audio: &[f32]) -> bool {
    audio.len() < MIN_SAMPLES
}

/// Runs a transcription, turning a library error or a panic into a [`TranscribeSnafu`] error.
pub(super) fn guarded(f: impl FnOnce() -> Result<String, BoxError>) -> Result<String, SttError> {
    let outcome = catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|panic| {
        let message = panic
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic".to_owned());
        Err(format!("panicked: {message}").into())
    });
    Ok(outcome.context(TranscribeSnafu { step: "parakeet" })?)
}

/// The process's ONNX Runtime environment, created once: ONNX Runtime allows only one.
fn environment() -> Result<Environment, BoxError> {
    static ENV: OnceLock<Result<Environment, String>> = OnceLock::new();
    ENV.get_or_init(|| ort::init().with_name("oku-slides").build().map_err(|e| e.to_string())).clone().map_err(boxed)
}

/// Compute threads: the available cores, up to 8.
fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, std::num::NonZero::get).min(8)
}

#[cfg(test)]
#[path = "parakeet_tests.rs"]
mod parakeet_tests;
