use speculoos::prelude::*;

use super::*;
use crate::test_support::{fresh_dir, platform_in};

#[test]
fn configure_speech_saved_and_read_back_at_next_launch() {
    let dir = fresh_dir("speech-config");
    let platform = platform_in(dir.path(), "fr", |s| s.apple = false);
    let speech = SpeechConfig { voice: Some("v-fr".into()), rate: 1.5, in_presentation: true };
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
    let speech = SpeechConfig { voice: Some("v-fr".into()), rate: 1.5, in_presentation: true };
    stt.configure_speech(Some(true), speech.clone());
    let _applied = stt.configure(crate::Config { auto_stop: !stale.auto_stop, ..stale });
    assert_that!(stt.speech_settings()).is_equal_to((Some(true), speech));
}
