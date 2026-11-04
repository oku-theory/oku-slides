use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;
use crate::test_support::fresh_dir;

fn engine(dir: &std::path::Path) -> FakeSpeechEngine {
    FakeSpeechEngine::new(Storage::local().unwrap_or_else(|e| panic!("storage: {e}")), dir.join("spoken.log"))
}

#[test]
fn fake_engine_logs_text_and_calls_done_once() {
    let dir = fresh_dir("fake-voice");
    let voice = engine(dir.path());
    let ids = Arc::new(Mutex::new(Vec::new()));
    for (n, text) in [(1, "Slide added"), (2, "Two\nlines")] {
        let seen = Arc::clone(&ids);
        let done = SpeechDone::new(move |id| seen.lock().unwrap_or_else(PoisonError::into_inner).push(id));
        voice
            .speak(SpeechId::new(n), text, Some("fake-fr"), SpeechRate::default(), done)
            .unwrap_or_else(|e| panic!("speak: {e}"));
    }
    voice.stop(SpeechId::new(2)).unwrap_or_else(|e| panic!("stop: {e}"));
    let log = std::fs::read_to_string(dir.path().join("spoken.log")).unwrap_or_else(|e| panic!("log: {e}"));
    assert_that!(log).is_equal_to("Slide added\nTwo lines\n".to_owned());
    assert_that!(ids.lock().unwrap_or_else(PoisonError::into_inner).clone())
        .is_equal_to(vec![SpeechId::new(1), SpeechId::new(2)]);
}

#[test]
fn fake_engine_has_no_german_voice() {
    let dir = fresh_dir("fake-voice-de");
    let voice = engine(dir.path());
    assert_that!(voice.status("de"))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage });
    let refused = voice.speak(SpeechId::new(1), "x", Some("fake-de"), SpeechRate::default(), SpeechDone::new(|_| {}));
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoVoiceForLanguage }));
    assert_that!(dir.path().join("spoken.log").exists()).is_false();
}

#[test]
fn fake_engine_speak_without_voice_is_refused() {
    let dir = fresh_dir("fake-voice-none");
    let voice = engine(dir.path());
    let refused = voice.speak(SpeechId::new(1), "x", None, SpeechRate::default(), SpeechDone::new(|_| {}));
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoVoiceForLanguage }));
    assert_that!(dir.path().join("spoken.log").exists()).is_false();
}
