//! Le moteur d'Apple pour de vrai (macOS 26, français installé) : `just test-apple`.
#![cfg(target_os = "macos")]
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use std::time::{Duration, Instant};

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_stt::{Config, Model, Stt};
use speculoos::prelude::*;

mod common;
use common::workdir;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/stt/dictee-fr.wav");

#[test]
#[ignore = "macOS 26 et langue française installée : just test-apple"]
fn apple_transcribes_french_fixture_quickly() {
    let dir = workdir("apple");
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.join("data"));
    settings.paths.models = Some(dir.join("models"));
    settings.i18n.locale = Some("fr".into());
    let stt = Stt::new(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}")));
    // Dictée au repos : le réglage s'applique, son résultat est sans objet ici.
    let _applied = stt.configure(Config { model: Model::Apple, language: Some("fr".into()), ..stt.status().config });
    let apple = stt.status().models.into_iter().find(|m| m.id == Model::Apple);
    assert_that!(apple.map(|m| m.installed)).is_equal_to(Some(true));
    let mut reader = hound::WavReader::open(FIXTURE).unwrap_or_else(|e| panic!("fixture : {e}"));
    let audio: Vec<f32> = reader.samples::<i16>().map(|s| f32::from(s.unwrap_or(0)) / 32768.0).collect();

    let started = Instant::now();
    let (text, phrases) = stt.transcribe_samples(&audio).unwrap_or_else(|e| panic!("transcription : {e}"));

    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(2));
    assert_that!(phrases).is_not_empty();
    let lower = text.to_lowercase();
    assert_that!(lower.contains("diapositive") && lower.contains("trimestre")).is_true();
}
