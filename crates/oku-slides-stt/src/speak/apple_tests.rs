//! The real bridge, on this Mac: listing voices is quick and silent.

use speculoos::prelude::*;

use super::*;
use crate::speak::{SpeechEngine, SpeechRate, SpeechStatus, UnavailableReason};

#[test]
fn apple_lists_at_least_one_voice() {
    let all = apple_ffi::voices();
    assert_that!(all).is_not_empty();
    let lang = all.first().map(|v| v.lang.clone()).unwrap_or_default();
    match AppleSpeechEngine::default().status(&lang) {
        SpeechStatus::Ready { voices } => {
            assert_that!(voices.iter().all(|v| crate::speak::same_language(&v.lang, &lang))).is_true();
        }
        SpeechStatus::Unavailable { reason } => panic!("a listed voice's language is unavailable: {reason:?}"),
    }
}

#[test]
fn apple_unknown_voice_is_unavailable() {
    let refused = AppleSpeechEngine::default().speak(
        SpeechId::new(1),
        "x",
        Some("org.oku.no-such-voice"),
        SpeechRate::default(),
        SpeechDone::new(|_| {}),
    );
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(crate::SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoVoiceForLanguage }));
}

/// The first installed voice: a test speech needs a voice, never the default one.
fn any_voice() -> String {
    apple_ffi::voices().first().map_or_else(|| panic!("no voice on this Mac"), |v| v.id.clone())
}

#[test]
fn apple_stop_runs_done_before_returning() {
    // No main run loop in a test: the delegate never fires, so DONE comes from the cut itself,
    // on this thread, inside `stop`. Callers must not hold a lock their DONE takes (trait doc).
    let engine = AppleSpeechEngine::default();
    let (tx, rx) = std::sync::mpsc::channel();
    let done = SpeechDone::new(move |id| tx.send(id).unwrap_or_else(|e| panic!("{e}")));
    engine
        .speak(SpeechId::new(4), ".", Some(&any_voice()), SpeechRate::default(), done)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    engine.stop(SpeechId::new(4)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(rx.try_recv()).is_equal_to(Ok(SpeechId::new(4)));
}

#[test]
fn apple_speak_without_voice_is_refused() {
    let refused =
        AppleSpeechEngine::default().speak(SpeechId::new(1), "x", None, SpeechRate::default(), SpeechDone::new(|_| {}));
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(crate::SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoVoiceForLanguage }));
}

#[test]
fn apple_second_speak_cuts_first_with_one_done() {
    // Two speaks in a row: the first is cut by the second, with exactly one DONE, inline (no main
    // run loop here); the second is the current speech, which `stop` then ends with its own DONE.
    let engine = AppleSpeechEngine::default();
    let voice = any_voice();
    let (tx, rx) = std::sync::mpsc::channel();
    let reporting = |tx: std::sync::mpsc::Sender<SpeechId>| {
        SpeechDone::new(move |id| tx.send(id).unwrap_or_else(|e| panic!("{e}")))
    };
    for n in [1, 2] {
        engine
            .speak(SpeechId::new(n), ".", Some(&voice), SpeechRate::default(), reporting(tx.clone()))
            .unwrap_or_else(|e| panic!("speak {n}: {e}"));
    }
    assert_that!(rx.try_iter().collect::<Vec<_>>()).is_equal_to(vec![SpeechId::new(1)]);
    engine.stop(SpeechId::new(1)).unwrap_or_else(|e| panic!("stop 1: {e}"));
    assert_that!(rx.try_iter().collect::<Vec<_>>()).is_empty();
    engine.stop(SpeechId::new(2)).unwrap_or_else(|e| panic!("stop 2: {e}"));
    assert_that!(rx.try_iter().collect::<Vec<_>>()).is_equal_to(vec![SpeechId::new(2)]);
}
