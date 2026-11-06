use speculoos::prelude::*;

use super::*;
use crate::test_support::{Part, fresh_dir, listening_stt, platform_in};

#[test]
fn configure_speech_saved_and_read_back_at_next_launch() {
    let dir = fresh_dir("speech-config");
    let platform = platform_in(dir.path(), "fr", |s| s.apple = false);
    let speech = SpeechConfig { voice: Some("v-fr".into()), rate: 1.5, in_presentation: true, barge_in: false };
    Stt::new(&platform).configure_speech(Some(false), speech.clone());
    let again = Stt::new(&platform);
    assert_that!(again.speech_settings()).is_equal_to((Some(false), speech));
}

#[test]
fn speech_settings_never_set_by_default() {
    let dir = fresh_dir("speech-config-default");
    let stt = Stt::new(&platform_in(dir.path(), "fr", |s| s.apple = false));
    assert_that!(stt.speech_settings()).is_equal_to((None, SpeechConfig::default()));
}

#[test]
fn configure_with_a_stale_snapshot_keeps_the_live_speech_settings() {
    let dir = fresh_dir("speech-config-stale");
    let stt = Stt::new(&platform_in(dir.path(), "fr", |s| s.apple = false));
    let stale = stt.status().config;
    let speech = SpeechConfig { voice: Some("v-fr".into()), rate: 1.5, in_presentation: true, barge_in: false };
    stt.configure_speech(Some(true), speech.clone());
    let _applied = stt.configure(crate::Config { auto_stop: !stale.auto_stop, ..stale });
    assert_that!(stt.speech_settings()).is_equal_to((Some(true), speech));
}

#[test]
fn speech_config_without_barge_in_defaults_true() {
    let read: SpeechConfig = serde_json::from_str(r#"{"voice": null, "rate": 1.0, "in_presentation": false}"#)
        .unwrap_or_else(|e| panic!("speech config: {e}"));
    assert_that!(read.barge_in).is_true();
}

#[test]
fn configure_speech_barge_in_change_restarts_listening() {
    let dir = fresh_dir("speech-config-barge-restart");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(0.5)], 1.0);
    stt.listen(Vec::new());
    let before = crate::test_support::wait_until(&stt, "listening", |s| s.state == crate::State::Listening);
    let session = stt.state().session;
    let (read_aloud, speech) = stt.speech_settings();
    stt.configure_speech(read_aloud, SpeechConfig { barge_in: !speech.barge_in, ..speech });
    let g = stt.state();
    assert_that!((before.state, g.state, g.session > session)).is_equal_to((
        crate::State::Listening,
        crate::State::Listening,
        true,
    ));
}

#[test]
fn configure_speech_rate_change_keeps_listening_capture() {
    let dir = fresh_dir("speech-config-rate-keeps");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(0.5)], 1.0);
    stt.listen(Vec::new());
    crate::test_support::wait_until(&stt, "listening", |s| s.state == crate::State::Listening);
    let session = stt.state().session;
    let (read_aloud, speech) = stt.speech_settings();
    stt.configure_speech(read_aloud, SpeechConfig { rate: 1.5, ..speech });
    assert_that!(stt.state().session).is_equal_to(session);
}
