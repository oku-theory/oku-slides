use std::path::{Path, PathBuf};

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::{Parakeet, guarded, too_short};
use crate::audio::{parse_wav, resample};
use crate::phrase_engine::PhraseEngine;
use crate::test_support::fresh_dir;
use crate::{SAMPLE_RATE, SttErrorKind};

/// The real model, downloaded once by hand into `$TMPDIR/parakeet` (ignored tests only).
fn real_model() -> Parakeet {
    let dir = std::env::temp_dir().join("parakeet");
    Parakeet::load(&dir).unwrap_or_else(|e| panic!("model in {}: {e}", dir.display()))
}

/// `fixtures/stt/dictee-fr.wav`, mono 16 kHz.
fn french_speech() -> Vec<f32> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/stt/dictee-fr.wav");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let (rate, samples) = parse_wav(&bytes).unwrap_or_else(|e| panic!("wav: {e}"));
    resample(&samples, rate, SAMPLE_RATE)
}

/// A model folder holding the three pinned names, each with `contents`, the vocabulary `vocab`.
fn model_dir(contents: &[u8], vocab: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let dir = fresh_dir("parakeet-load");
    let model = dir.path().join("model");
    std::fs::create_dir_all(&model).unwrap_or_else(|e| panic!("dir: {e}"));
    for name in ["encoder-model.int8.onnx", "decoder_joint-model.int8.onnx"] {
        std::fs::write(model.join(name), contents).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    std::fs::write(model.join("vocab.txt"), vocab).unwrap_or_else(|e| panic!("vocab: {e}"));
    (dir, model)
}

#[test]
fn load_missing_dir_returns_model_load_error() {
    let dir = fresh_dir("parakeet-missing");
    let err = Parakeet::load(&dir.path().join("absent")).err().unwrap_or_else(|| panic!("loaded"));
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_equal_to(Localizer::new(Some("fr")).tr("stt-model"));
}

#[test]
fn load_garbage_files_returns_model_load_error() {
    let (_dir, model) = model_dir(b"x", b"x");
    let err = Parakeet::load(&model).err().unwrap_or_else(|| panic!("loaded"));
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}

#[test]
fn load_empty_vocab_returns_model_load_error() {
    let (_dir, model) = model_dir(&[0u8; 4096], b"");
    let err = Parakeet::load(&model).err().unwrap_or_else(|| panic!("loaded"));
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}

#[test]
fn transcribe_empty_audio_returns_empty_text() {
    assert_that!(too_short(&[])).is_true();
}

#[test]
fn transcribe_one_sample_returns_empty_text() {
    assert_that!(too_short(&[0.5])).is_true();
}

#[test]
fn too_short_tenth_of_a_second_returns_false() {
    assert_that!(too_short(&[0.0; 1600])).is_false();
}

#[test]
fn transcribe_panic_is_an_error() {
    let err = guarded(|| panic!("feature cache")).err().unwrap_or_else(|| panic!("no error"));
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Transcription);
    assert_that!(err.to_string()).contains("parakeet");
}

#[test]
fn guarded_library_error_is_a_transcribe_error() {
    let err = guarded(|| Err("onnx".into())).err().unwrap_or_else(|| panic!("no error"));
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Transcription);
}

#[test]
fn guarded_text_passes_through() {
    assert_that!(guarded(|| Ok("bonjour".to_owned())).ok()).is_equal_to(Some("bonjour".to_owned()));
}

#[test]
#[ignore = "needs the real model"]
fn transcribe_french_wav_returns_text() {
    let t = real_model().transcribe(&french_speech(), None, None, false).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(t.text.trim().is_empty()).is_false();
    assert_that!(t.text.as_str()).is_equal_to(t.text.trim());
    assert_that!(t.language).is_none();
}

#[test]
#[ignore = "needs the real model"]
fn transcribe_silence_returns_empty_text() {
    let silence = vec![0.0; 3200];
    let t = real_model().transcribe(&silence, None, None, false).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(t.text.as_str()).is_equal_to("");
}

#[test]
#[ignore = "needs the real model"]
fn transcribe_ignores_bogus_language() {
    let model = real_model();
    let audio = french_speech();
    let plain = model.transcribe(&audio, None, None, false).unwrap_or_else(|e| panic!("{e}"));
    let bogus = model.transcribe(&audio, Some("xx"), Some("contexte"), true).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(bogus.text).is_equal_to(plain.text);
}

#[test]
#[ignore = "needs the real model"]
fn transcribe_overlong_audio_returns_text() {
    let speech = french_speech();
    let mut audio = Vec::new();
    while audio.len() < 35 * SAMPLE_RATE as usize {
        audio.extend_from_slice(&speech);
    }
    let t = real_model().transcribe(&audio, None, None, false).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(t.text.trim().is_empty()).is_false();
}
