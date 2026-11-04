use std::sync::Arc;

use oku_slides_i18n::Localizer;
use oku_slides_mcp::{Origin, VoiceApplied, VoiceErrorKind, VoicePort};
use oku_slides_session::{DictationPatch, SpeechPatch, SpeechStatusView, VoiceEvent};
use rstest::rstest;
use speculoos::prelude::*;

use super::super::voice_port_tests::{T, build_with};
use crate::hand_voice::HandVoice;

/// A French box with a hand voice (`with_voice`) or none.
fn box_fr(with_voice: bool) -> T {
    let engine: Option<Arc<dyn oku_slides_stt::SpeechEngine>> =
        with_voice.then(|| Arc::new(HandVoice::default()) as Arc<dyn oku_slides_stt::SpeechEngine>);
    build_with(true, engine, |_| {})
}

fn speech(patch: SpeechPatch) -> VoiceEvent {
    VoiceEvent::Speech(patch)
}

fn agent() -> Origin {
    Origin::Agent { tab: "t-1".into(), confirmed: false }
}

#[rstest]
#[case(0.4)]
#[case(2.5)]
#[case(f32::NAN)]
fn set_speech_rate_out_of_range_refused(#[case] rate: f32) {
    let t = box_fr(true);
    let e = t.voice.apply(speech(SpeechPatch { rate: Some(rate), ..Default::default() }), agent()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(VoiceErrorKind::BadRate);
}

#[test]
fn set_speech_saves_and_the_box_shows_it() {
    let t = box_fr(true);
    let patch = SpeechPatch {
        read_aloud: Some(true),
        voice: Some("v-fr2".into()),
        rate: Some(1.5),
        in_presentation: Some(true),
    };
    assert_that!(t.voice.apply(speech(patch.clone()), agent()).ok()).is_equal_to(Some(VoiceApplied::Now));
    let view = t.voice.view().speech.unwrap();
    assert_that!((view.read_aloud, view.voice.as_deref(), view.rate, view.in_presentation)).is_equal_to((
        true,
        Some("v-fr2"),
        1.5,
        true,
    ));
    assert_that!(t.voice.apply(speech(patch), agent()).ok()).is_equal_to(Some(VoiceApplied::Unchanged));
}

#[test]
fn set_speech_unknown_voice_refused() {
    let t = box_fr(true);
    let e = t.voice.check(&speech(SpeechPatch { voice: Some("v-ja".into()), ..Default::default() }), &agent());
    assert_that!(e.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::UnknownVoice);
}

#[test]
fn set_speech_on_without_voice_refused() {
    let t = box_fr(false);
    let e = t.voice.check(&speech(SpeechPatch { read_aloud: Some(true), ..Default::default() }), &agent());
    assert_that!(e.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::SpeechUnavailable);
}

#[test]
fn speech_view_without_voice_tells_why_localized() {
    let view = box_fr(false).voice.view().speech.unwrap();
    let fr = Localizer::new(Some("fr"));
    assert_that!(view.status).is_equal_to(SpeechStatusView::Unavailable { reason: fr.tr("stt-voice-no-engine") });
}

#[rstest]
#[case::never_set(None, Some(true))]
#[case::user_turned_it_off(Some(false), Some(false))]
fn hands_free_on_turns_read_aloud_on_the_first_time_only(#[case] before: Option<bool>, #[case] after: Option<bool>) {
    let t = box_fr(true);
    let stt = t.stt.clone().unwrap();
    if let Some(on) = before {
        t.voice.apply(speech(SpeechPatch { read_aloud: Some(on), ..Default::default() }), Origin::User).unwrap();
    }
    let hands_free = VoiceEvent::Dictation(DictationPatch { hands_free: Some(true), ..Default::default() });
    t.voice.apply(hands_free, Origin::User).unwrap();
    assert_that!(stt.speech_settings().0).is_equal_to(after);
}
