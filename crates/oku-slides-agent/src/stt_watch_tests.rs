use speculoos::prelude::*;

use super::Fingerprint;
use crate::voice_port::voice_port_tests::voice_with_stt;

#[test]
fn fingerprint_ignores_the_mic_level() {
    let status = voice_with_stt().stt.unwrap().status();
    let mut louder = status.clone();
    louder.level = 0.9;
    assert_that!(Fingerprint::of(&louder)).is_equal_to(Fingerprint::of(&status));
}

#[test]
fn fingerprint_follows_the_state() {
    let status = voice_with_stt().stt.unwrap().status();
    let mut recording = status.clone();
    recording.state = oku_slides_stt::State::Recording;
    assert_that!(Fingerprint::of(&recording)).is_not_equal_to(Fingerprint::of(&status));
}
