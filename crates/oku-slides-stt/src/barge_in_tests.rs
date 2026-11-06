//! Barge-in: when listening goes through the duplex, and why it cannot.

use std::sync::Arc;

use oku_slides_i18n::Localizer;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::hand_voice::HandEngine;
use crate::test_support::{fresh_dir, platform_in};

/// A French dictation whose microphone is a silent WAV.
fn stt(dir: &std::path::Path) -> Stt {
    Stt::new(&platform_in(dir, "fr", |s| s.apple = false))
}

#[rstest]
#[case::listening_on(Mode::Listen, true, false, true, true)]
#[case::dictation(Mode::Dictate, true, false, true, false)]
#[case::setting_off(Mode::Listen, false, false, true, false)]
#[case::fake_microphone(Mode::Listen, true, true, true, false)]
#[case::no_duplex_here(Mode::Listen, true, false, false, false)]
fn wants_duplex_only_for_listening_with_barge_in(
    #[case] mode: Mode,
    #[case] barge_in: bool,
    #[case] fake_mic: bool,
    #[case] available: bool,
    #[case] wanted: bool,
) {
    assert_that!(wants_duplex(mode, barge_in, fake_mic, available)).is_equal_to(wanted);
}

#[rstest]
#[case::listening_on(State::Listening, true, true)]
#[case::listening_off(State::Listening, false, false)]
#[case::idle_on(State::Idle, true, false)]
fn barge_voice_only_while_listening_with_setting_on(#[case] state: State, #[case] on: bool, #[case] some: bool) {
    let dir = fresh_dir("barge-voice");
    let stt = stt(dir.path());
    {
        let mut g = stt.state();
        g.state = state;
        g.config.speech.barge_in = on;
        g.barge = Some(Arc::new(HandEngine::default()));
    }
    assert_that!(stt.barge_voice().is_some()).is_equal_to(some);
}

#[test]
fn open_microphone_with_fake_mic_never_starts_duplex() {
    let dir = fresh_dir("barge-fake-mic");
    let stt = stt(dir.path());
    let (_recorder, voice) = stt.open_microphone(Mode::Listen).unwrap_or_else(|e| panic!("microphone: {e}"));
    assert_that!((voice.is_none(), stt.state().barge_failure.is_none())).is_equal_to((true, true));
}

#[test]
fn listening_reset_drops_barge_voice() {
    let dir = fresh_dir("barge-reset");
    let stt = stt(dir.path());
    {
        let mut g = stt.state();
        g.state = State::Listening;
        g.barge = Some(Arc::new(HandEngine::default()));
    }
    stt.stop();
    assert_that!(stt.state().barge.is_none()).is_true();
}

#[cfg(not(target_os = "macos"))]
#[test]
fn barge_in_unavailable_off_macos_names_the_os() {
    let dir = fresh_dir("barge-os");
    let reason = stt(dir.path()).barge_in_unavailable(&Localizer::new(Some("fr")));
    assert_that!(reason).is_equal_to(Some(
        "Parler par-dessus Oku demande l'annulation d'écho, disponible sur macOS seulement.".to_owned(),
    ));
}

#[cfg(target_os = "macos")]
#[test]
fn barge_in_unavailable_on_macos_none_before_any_start() {
    let dir = fresh_dir("barge-macos");
    assert_that!(stt(dir.path()).barge_in_unavailable(&Localizer::new(Some("fr")))).is_none();
}

#[cfg(target_os = "macos")]
#[test]
fn barge_in_unavailable_after_failed_start_tells_why() {
    let dir = fresh_dir("barge-failed");
    let stt = stt(dir.path());
    stt.state().barge_failure = Some(Arc::new(crate::duplex::failure(4)));
    assert_that!(stt.barge_in_unavailable(&Localizer::new(Some("fr"))))
        .is_equal_to(Some("Le moteur audio avec annulation d'écho ne démarre pas".to_owned()));
}

#[test]
fn barge_in_unavailable_os_key_read_in_english_and_french() {
    let english = Localizer::new(Some("en")).tr("speech-barge-in-unavailable-os");
    let french = Localizer::new(Some("fr")).tr("speech-barge-in-unavailable-os");
    assert_that!((english.as_str(), french.as_str())).is_equal_to((
        "Talking over Oku needs echo cancellation, available on macOS only.",
        "Parler par-dessus Oku demande l'annulation d'écho, disponible sur macOS seulement.",
    ));
}

#[rstest]
#[case::turned_off(false, true)]
#[case::kept_on(true, false)]
fn barge_in_turned_off_forgets_failed_start(#[case] barge_in: bool, #[case] forgotten: bool) {
    let dir = fresh_dir("barge-off-forgets");
    let stt = stt(dir.path());
    stt.state().barge_failure = Some(Arc::new(crate::duplex::failure(4)));
    stt.configure_speech(Some(true), crate::SpeechConfig { barge_in, ..crate::SpeechConfig::default() });
    // Forgotten, the settings box shows the switch again, so the user can turn it back on.
    assert_that!(stt.state().barge_failure.is_none()).is_equal_to(forgotten);
}
