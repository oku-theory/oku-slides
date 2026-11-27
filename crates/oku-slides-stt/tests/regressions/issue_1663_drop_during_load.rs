//! t-1663: a dictation started with Parakeet loads ONNX Runtime on its transcription
//! thread. When the process exited during that load (a quit, or a test ending right after
//! `Start`), ONNX Runtime's static objects were destroyed under the thread: SIGSEGV.
//! Dropping the last [`Stt`], or `Stt::shutdown`, now stops the dictation and waits for that
//! thread: the process exits only once the load is over.

use std::path::{Path, PathBuf};

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;
use oku_slides_stt::{Model, State, Stt, SttRequest};
use speculoos::prelude::*;

use crate::common::workdir;

/// One second of a quiet tone: enough for the fake microphone to start.
fn wav_bytes() -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap_or_else(|e| panic!("wav: {e}"));
        for _ in 0..16_000 {
            writer.write_sample(100i16).unwrap_or_else(|e| panic!("wav: {e}"));
        }
        writer.finalize().unwrap_or_else(|e| panic!("wav: {e}"));
    }
    cursor.into_inner()
}

/// Parakeet marked installed without its files (the load starts ONNX Runtime, then fails),
/// a WAV as the microphone.
fn stt(dir: &Path) -> Stt {
    let storage = Storage::local().unwrap_or_else(|e| panic!("storage: {e}"));
    let models = dir.join("models");
    storage.write(&Model::Parakeet.path(&models).join(".complete"), Vec::new()).unwrap_or_else(|e| panic!("{e}"));
    let wav: PathBuf = dir.join("voice.wav");
    storage.write(&wav, wav_bytes()).unwrap_or_else(|e| panic!("{e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(dir.join("data"));
    settings.paths.decks = Some(dir.join("data").join("decks"));
    settings.paths.models = Some(models);
    settings.stt.fake_mic = Some(wav);
    settings.i18n.locale = Some("fr".into());
    Stt::new(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}")))
}

#[test]
fn stt_dropped_during_model_load_exits_without_crash() {
    let dir = workdir("drop-load");
    let stt = stt(&dir);
    let started = stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    assert_that!(started.state).is_equal_to(State::Recording);
    // Returns once the transcription thread has ended; the process then exits cleanly.
    drop(stt);
}

#[test]
fn stt_shutdown_during_model_load_returns_to_idle() {
    let dir = workdir("shutdown-load");
    let stt = stt(&dir);
    // A clone held elsewhere (the app's dictation watch) keeps the dictation alive.
    let watcher = stt.clone();
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    stt.shutdown();
    assert_that!(watcher.status().state).is_equal_to(State::Idle);
}
