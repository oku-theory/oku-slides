//! t-1216: a `config.json` that did not parse was silently replaced by the defaults at
//! launch, and the next change in the settings box (choosing the API model) wrote those
//! defaults over it: the user's address, headers and parameters were lost without a log.

use std::path::Path;

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;
use oku_slides_stt::{Config, Model, Stt};
use speculoos::prelude::*;

use crate::common::workdir;

/// A header written as a bare string instead of `{"env": …}`: the whole file is invalid.
const BROKEN: &str = r#"{"model": "api", "version": 1, "api": {"url": "https://example.test/v1",
  "headers": [{"name": "api-key", "value": "$AZURE_KEY"}]}}"#;

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

#[test]
fn issue_1216_unreadable_config_is_kept_aside_before_any_write() {
    let dir = workdir("t1216");
    let models = dir.join("models");
    let storage = Storage::local().unwrap_or_else(|e| panic!("storage: {e}"));
    storage.write(&models.join("config.json"), BROKEN).unwrap_or_else(|e| panic!("config: {e}"));

    let stt = stt(&dir);
    let applied = stt.configure(Config { model: Model::Api, ..stt.status().config });
    asserting("the API model is chosen").that(&applied).is_true();

    let aside = kept_aside(&models);
    asserting("one copy kept aside").that(&aside).has_length(1);
    let name = aside.first().map_or("", String::as_str);
    let kept = std::fs::read_to_string(models.join(name)).unwrap_or_else(|e| panic!("kept: {e}"));
    asserting("the user's file, byte for byte").that(&kept.as_str()).is_equal_to(BROKEN);
}
