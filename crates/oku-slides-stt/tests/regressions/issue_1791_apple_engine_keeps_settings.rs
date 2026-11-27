//! t-1791: the Apple engine was removed while it was the default on macOS 26. A `config.json`
//! naming it switches to Parakeet and keeps every other dictation setting; nothing is kept aside.

use std::path::Path;

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;
use oku_slides_stt::{Model, Stt};
use speculoos::prelude::*;

use crate::common::workdir;

fn stt(dir: &Path) -> Stt {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(dir.join("data"));
    settings.paths.decks = Some(dir.join("data").join("decks"));
    settings.paths.models = Some(dir.join("models"));
    Stt::new(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}")))
}

#[test]
fn issue_1791_apple_engine_switches_to_parakeet_and_keeps_settings() {
    let dir = workdir("t1791-apple");
    let models = dir.join("models");
    let original = r#"{"model": "apple", "version": 1, "language": "fr", "silence": 3.0, "hands_free": true}"#;
    let storage = Storage::local().unwrap_or_else(|e| panic!("storage: {e}"));
    storage.write(&models.join("config.json"), original).unwrap_or_else(|e| panic!("config: {e}"));

    let config = stt(&dir).status().config;

    asserting("dictation starts on Parakeet").that(&config.model).is_equal_to(Model::Parakeet);
    asserting("language kept").that(&config.language).is_equal_to(Some("fr".to_owned()));
    asserting("silence kept").that(&config.silence).is_equal_to(3.0);
    asserting("hands-free kept").that(&config.hands_free).is_true();
    let names: Vec<String> = std::fs::read_dir(&models)
        .unwrap_or_else(|e| panic!("models: {e}"))
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    asserting("nothing kept aside").that(&names).is_equal_to(vec!["config.json".to_owned()]);
}
