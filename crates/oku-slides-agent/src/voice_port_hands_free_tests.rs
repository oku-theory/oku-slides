use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::{Origin, VoiceApplied, VoiceErrorKind, VoicePort};
use oku_slides_session::{DictationPatch, VoiceEvent};
use oku_slides_stt::State;
use rstest::rstest;
use speculoos::prelude::*;

use super::super::check_patch;
use super::super::voice_port_tests::{T, build};

fn hands_free(on: bool) -> VoiceEvent {
    VoiceEvent::Dictation(DictationPatch { hands_free: Some(on), ..Default::default() })
}

fn agent() -> Origin {
    Origin::Agent { tab: "t-1".into(), confirmed: false }
}

/// A box whose saved dictation settings have hands-free on, as a previous launch left them.
fn saved_on() -> T {
    build(true, |s| {
        let models = s.paths.models();
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("config.json"), r#"{"model":"turbo","version":1,"hands_free":true}"#).unwrap();
    })
}

#[test]
fn hands_free_on_from_agent_is_refused_localized() {
    let t = build(true, |_| {});
    let e = t.voice.check(&hands_free(true), &agent()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(VoiceErrorKind::HandsFreeReservedToUser);
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr).as_str()).does_not_contain("voice-error-");
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("voice-error-hands-free-reserved"));
    let applied = t.voice.apply(hands_free(true), Origin::Agent { tab: "t-1".into(), confirmed: true });
    assert_that!(applied.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::HandsFreeReservedToUser);
    assert_that!(t.stt.as_ref().map(|s| s.status().config.hands_free)).is_equal_to(Some(false));
}

#[test]
fn hands_free_off_from_agent_applies() {
    let t = saved_on();
    let applied = t.voice.apply(hands_free(false), agent());
    assert_that!(applied.ok()).is_equal_to(Some(VoiceApplied::Now));
    assert_that!(t.stt.as_ref().map(|s| s.status().config.hands_free)).is_equal_to(Some(false));
    assert_that!(t.voice.view().dictation.map(|d| d.hands_free)).is_equal_to(Some(false));
}

#[rstest]
#[case::agent_off(agent(), false)]
#[case::user_on(Origin::User, true)]
#[case::user_off(Origin::User, false)]
fn hands_free_patch_while_listening_applies(#[case] origin: Origin, #[case] on: bool) {
    let mut status = build(true, |_| {}).stt.unwrap().status();
    status.state = State::Listening;
    status.config.hands_free = !on;
    let patch = DictationPatch { hands_free: Some(on), ..Default::default() };
    let (checked, config) = check_patch(&status, &patch, &origin).unwrap();
    assert_that!(checked.unchanged).is_false();
    assert_that!(config.hands_free).is_equal_to(on);
}

#[test]
fn hands_free_saved_on_starts_paused_at_launch() {
    let t = saved_on();
    let status = t.stt.as_ref().unwrap().status();
    assert_that!(status.config.hands_free).is_true();
    assert_that!(status.state).is_equal_to(State::Idle);
    assert_that!(t.voice.view().dictation.map(|d| d.hands_free)).is_equal_to(Some(true));
}
