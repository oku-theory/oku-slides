//! t-1663: Parakeet replaced the former local models `turbo`, `large` and `base` with no alias. A
//! `config.json` that still names one no longer parses: it is kept aside byte for byte, never
//! overwritten, and dictation starts on the Parakeet defaults.

use std::path::Path;

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;
use oku_slides_stt::{Model, Stt};
use rstest::rstest;
use speculoos::prelude::*;

use crate::common::workdir;

fn stt(dir: &Path) -> Stt {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(dir.join("data"));
    settings.paths.decks = Some(dir.join("data").join("decks"));
    settings.paths.models = Some(dir.join("models"));
    settings.stt.apple = false;
    Stt::new(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}")))
}

/// The files of `dir` whose name starts with `config.json.invalid-`.
fn kept_aside(dir: &Path) -> Vec<String> {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("models: {e}"));
    entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("config.json.invalid-"))
        .collect()
}

#[rstest]
fn issue_1663_old_model_id_is_kept_aside_and_parakeet_starts(#[values("turbo", "large", "base")] id: &str) {
    let dir = workdir(&format!("t1663-{id}"));
    let models = dir.join("models");
    let original = format!(r#"{{"model": "{id}", "version": 1, "language": "fr"}}"#);
    let storage = Storage::local().unwrap_or_else(|e| panic!("storage: {e}"));
    storage.write(&models.join("config.json"), original.as_str()).unwrap_or_else(|e| panic!("config: {e}"));

    let stt = stt(&dir);

    asserting("dictation starts on Parakeet").that(&stt.status().config.model).is_equal_to(Model::Parakeet);
    let aside = kept_aside(&models);
    asserting("one copy kept aside").that(&aside).has_length(1);
    let name = aside.first().map_or("", String::as_str);
    let kept = std::fs::read_to_string(models.join(name)).unwrap_or_else(|e| panic!("kept: {e}"));
    asserting("the user's file, byte for byte").that(&kept).is_equal_to(original);
    asserting("no config.json written before a user change").that(&models.join("config.json").exists()).is_false();
}
