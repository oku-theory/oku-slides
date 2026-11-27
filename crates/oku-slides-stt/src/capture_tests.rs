use std::net::TcpListener;

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use speculoos::prelude::*;

use crate::test_support::{fresh_dir, platform_in};
use crate::{Config, Mode, Model, State, Stt};

#[test]
fn begin_recording_refused_once_state_moved_on() {
    let dir = fresh_dir("capture-race");
    let stt = Stt::new(&platform_in(dir.path(), "fr"));
    // Another `listen` won the race: listening already started from idle.
    let session = {
        let mut g = stt.state();
        g.state = State::Listening;
        g.session
    };
    stt.begin_recording(Mode::Listen, State::Idle);
    let g = stt.state();
    assert_that!((g.session, g.state)).is_equal_to((session, State::Listening));
}

#[test]
fn start_parakeet_not_installed_starts_download() {
    let dir = fresh_dir("parakeet-download");
    // A hub that takes the connection and never answers: the download stays in progress.
    let hub = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("hub: {e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.models = Some(dir.path().join("models"));
    settings.i18n.locale = Some("fr".into());
    settings.models.hub_url = format!("http://{}", hub.local_addr().unwrap_or_else(|e| panic!("hub: {e}")));
    settings.models.download_retries = 0;
    let stt = Stt::new(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}")));
    assert_that!(stt.status().config.model).is_equal_to(Model::Parakeet);
    stt.start(Vec::new());
    let s = stt.status();
    stt.cancel();
    assert_that!((s.state, s.error)).is_equal_to((State::Downloading, None));
}

#[test]
fn start_parakeet_in_japanese_refuses_locale_unsupported() {
    let dir = fresh_dir("parakeet-ja");
    let stt = Stt::new(&platform_in(dir.path(), "fr"));
    let installed = Model::Parakeet.path(&dir.path().join("models"));
    std::fs::create_dir_all(&installed).unwrap_or_else(|e| panic!("model: {e}"));
    std::fs::write(installed.join(oku_slides_platform::models::COMPLETE), b"").unwrap_or_else(|e| panic!("model: {e}"));
    // Dictation idle: the result of `configure` does not matter here.
    let _applied = stt.configure(Config { language: Some("ja".into()), ..stt.status().config });
    stt.start(Vec::new());
    let s = stt.status();
    assert_that!((s.state, s.error.as_deref()))
        .is_equal_to((State::Idle, Some("Le moteur de dictée ne prend pas en charge cette langue (ja)")));
    assert_that!(stt.state().started).is_none();
}
