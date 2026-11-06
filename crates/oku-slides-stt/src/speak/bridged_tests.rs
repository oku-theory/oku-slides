//! The bridged voice over a fake bridge: one DONE per id, whatever cuts it.

use std::sync::{Arc, Mutex, PoisonError};

use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

/// A speech's DONE, taken once by whichever of the end and the cut comes first.
type Pending = Arc<Mutex<Option<SpeechDone>>>;

/// A speech of the fake bridge: its DONE, taken once by the end or the cut.
#[derive(Debug)]
struct FakeSpeech {
    id: SpeechId,
    done: Pending,
}

impl FakeSpeech {
    /// Sends DONE if nothing sent it yet.
    fn close(&self) {
        let taken = self.done.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(done) = taken {
            done.call(self.id);
        }
    }
}

/// A bridge that refuses the voice `"unknown"` and keeps every accepted speech's DONE.
#[derive(Debug, Default)]
struct FakeBridge {
    /// Accepted speeches, oldest first, so a test can end one as the OS would.
    spoken: Mutex<Vec<(SpeechId, Pending)>>,
    cuts: Mutex<Vec<SpeechId>>,
}

impl VoiceBridge for FakeBridge {
    type Speech = FakeSpeech;

    fn voices(&self) -> Vec<VoiceInfo> {
        vec![VoiceInfo { id: "v".into(), name: "V".into(), lang: "fr-FR".into() }]
    }

    fn speak(&self, id: SpeechId, _: &str, voice: &str, _: f32, done: SpeechDone) -> Option<FakeSpeech> {
        if voice == "unknown" {
            return None;
        }
        let done = Arc::new(Mutex::new(Some(done)));
        self.spoken.lock().unwrap_or_else(PoisonError::into_inner).push((id, Arc::clone(&done)));
        Some(FakeSpeech { id, done })
    }

    fn cut(&self, speech: FakeSpeech) {
        self.cuts.lock().unwrap_or_else(PoisonError::into_inner).push(speech.id);
        speech.close();
    }
}

impl FakeBridge {
    /// Ends speech `n` by itself, as the player's last buffer would.
    fn end(&self, n: usize) {
        let spoken = self.spoken.lock().unwrap_or_else(PoisonError::into_inner);
        let (id, done) = &spoken[n];
        FakeSpeech { id: *id, done: Arc::clone(done) }.close();
    }
}

/// A voice over a fake bridge, and the ids its DONEs reported.
fn voice() -> (BridgedVoice<FakeBridge>, Arc<Mutex<Vec<SpeechId>>>) {
    (BridgedVoice::new(FakeBridge::default()), Arc::default())
}

fn speak(
    v: &BridgedVoice<FakeBridge>,
    n: u64,
    voice: &str,
    record: &Arc<Mutex<Vec<SpeechId>>>,
) -> Result<(), SttError> {
    let record = Arc::clone(record);
    let done = SpeechDone::new(move |id| record.lock().unwrap_or_else(PoisonError::into_inner).push(id));
    v.speak(SpeechId::new(n), "x", Some(voice), SpeechRate::default(), done)
}

fn ids(record: &Arc<Mutex<Vec<SpeechId>>>) -> Vec<SpeechId> {
    record.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

#[test]
fn bridged_second_speak_cuts_first_with_one_done() {
    let (v, record) = voice();
    speak(&v, 1, "v", &record).unwrap_or_else(|e| panic!("speak 1: {e}"));
    speak(&v, 2, "v", &record).unwrap_or_else(|e| panic!("speak 2: {e}"));
    assert_that!(ids(&record)).is_equal_to(vec![SpeechId::new(1)]);
}

#[test]
fn bridged_stop_of_older_id_cuts_nothing() {
    let (v, record) = voice();
    speak(&v, 1, "v", &record).unwrap_or_else(|e| panic!("speak 1: {e}"));
    speak(&v, 2, "v", &record).unwrap_or_else(|e| panic!("speak 2: {e}"));
    v.stop(SpeechId::new(1)).unwrap_or_else(|e| panic!("stop 1: {e}"));
    assert_that!(ids(&record)).is_equal_to(vec![SpeechId::new(1)]);
    assert_that!(v.bridge.cuts.lock().unwrap_or_else(PoisonError::into_inner).clone())
        .is_equal_to(vec![SpeechId::new(1)]);
}

#[test]
fn bridged_end_then_stop_gives_one_done() {
    let (v, record) = voice();
    speak(&v, 3, "v", &record).unwrap_or_else(|e| panic!("speak: {e}"));
    v.bridge.end(0);
    v.stop(SpeechId::new(3)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(ids(&record)).is_equal_to(vec![SpeechId::new(3)]);
}

#[test]
fn bridged_stop_of_current_id_sends_its_done() {
    let (v, record) = voice();
    speak(&v, 4, "v", &record).unwrap_or_else(|e| panic!("speak: {e}"));
    v.stop(SpeechId::new(4)).unwrap_or_else(|e| panic!("stop: {e}"));
    v.stop(SpeechId::new(4)).unwrap_or_else(|e| panic!("stop again: {e}"));
    assert_that!(ids(&record)).is_equal_to(vec![SpeechId::new(4)]);
}

#[test]
fn bridged_drop_cuts_the_current_speech() {
    let (v, record) = voice();
    speak(&v, 5, "v", &record).unwrap_or_else(|e| panic!("speak: {e}"));
    drop(v);
    assert_that!(ids(&record)).is_equal_to(vec![SpeechId::new(5)]);
}

#[test]
fn bridged_refused_voice_is_unavailable_without_done() {
    let (v, record) = voice();
    let refused = speak(&v, 6, "unknown", &record);
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoVoiceForLanguage }));
    assert_that!(ids(&record)).is_empty();
}

#[test]
fn bridged_status_keeps_the_voices_of_the_language() {
    let (v, _) = voice();
    assert_that!(v.status("en"))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage });
    assert_that!(matches!(v.status("fr"), SpeechStatus::Ready { .. })).is_true();
}
