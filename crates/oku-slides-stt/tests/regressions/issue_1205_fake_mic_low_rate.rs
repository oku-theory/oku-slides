//! t-1205: the fake microphone replays its WAV in 20 ms packets of `rate / 50`
//! samples. Below 50 Hz that size was 0, and `chunks(0)` panicked in the capture
//! thread: dictation stayed "recording" with no sound at all. A rate of 0 (or no
//! `fmt ` chunk before the data) was accepted as a valid file.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use oku_slides_i18n::Localizer;
use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;
use oku_slides_stt::{ApiConfig, Config, Model, State, Status, Stt, SttRequest};
use speculoos::prelude::*;

use crate::common::workdir;

/// A 16-bit mono WAV whose header says `rate` (0 included), holding `samples` loud samples.
fn wav_at(dir: &Path, rate: u32, samples: usize) -> PathBuf {
    let data: Vec<u8> = std::iter::repeat_n(8192i16.to_le_bytes(), samples).flatten().collect();
    let data_len = u32::try_from(data.len()).unwrap_or_else(|e| panic!("wav: {e}"));
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes()); // bytes per second
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(&data);
    let path = dir.join("voix.wav");
    let storage = Storage::local().unwrap_or_else(|e| panic!("storage: {e}"));
    storage.write(&path, bytes).unwrap_or_else(|e| panic!("wav: {e}"));
    path
}

/// Dictation through the API engine (no model to load), the WAV as microphone.
fn stt(dir: &Path, wav: PathBuf) -> Stt {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(dir.join("data"));
    settings.paths.decks = Some(dir.join("data").join("decks"));
    settings.paths.models = Some(dir.join("models"));
    settings.stt.fake_mic = Some(wav);
    settings.stt.apple = false;
    settings.i18n.locale = Some("fr".into());
    let stt = Stt::new(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}")));
    // `PATH` is set on every OS: the key counts as present. No request is sent while nothing is said.
    let api = ApiConfig {
        url: "http://127.0.0.1:1/v1".into(),
        model: "whisper-1".into(),
        key_env: "PATH".into(),
        ..ApiConfig::default()
    };
    let applied = stt.configure(Config { model: Model::Api, auto_stop: false, api, ..Config::default() });
    asserting("the API engine is chosen").that(&applied).is_true();
    stt
}

fn wait_until(stt: &Stt, what: &str, done: impl Fn(&Status) -> bool) -> Status {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = stt.status();
        if done(&status) {
            return status;
        }
        asserting(what).that(&(Instant::now() < deadline)).is_true();
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn fake_mic_rate_below_50_hz_delivers_the_samples() {
    let dir = workdir("low-rate");
    let stt = stt(&dir, wav_at(&dir, 8, 500));
    let started = stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    assert_that!(started.state).is_equal_to(State::Recording);
    let heard = wait_until(&stt, "the file's sound reaches the level meter", |s| s.level > 0.0);
    assert_that!(heard.error).is_none();
    assert_that!(stt.handle(SttRequest::Cancel).state).is_equal_to(State::Idle);
}

#[test]
fn fake_mic_zero_rate_is_refused_as_an_unreadable_file() {
    let dir = workdir("zero-rate");
    let stt = stt(&dir, wav_at(&dir, 0, 10));
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    let failed = wait_until(&stt, "the zero-rate file is refused", |s| s.error.is_some());
    let unreadable = Localizer::new(Some("fr")).tr("stt-audio-file");
    assert_that!(failed.error).is_equal_to(Some(unreadable));
    assert_that!(failed.state).is_equal_to(State::Idle);
}
