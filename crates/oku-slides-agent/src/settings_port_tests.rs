use std::sync::Arc;

use oku_slides_mcp::{Applied, SettingsPort};
use oku_slides_session::{Platform, Settings};
use serde_json::json;
use speculoos::prelude::*;

use super::AppSettings;

fn platform() -> Platform {
    let mut settings = Settings::defaults().unwrap();
    let dir = std::env::temp_dir().join(format!("oku-slides-settings-port-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::remove_file(dir.join("config.toml")).ok();
    settings.paths.data = Some(dir);
    Platform::with_settings(settings).unwrap()
}

#[test]
fn read_exposes_history_limit_and_vad_keys() {
    let p = platform();
    let port = AppSettings::new(&p, None);
    let v = port.read();
    assert_that!(v.get("history.limit").cloned()).is_equal_to(Some(json!(500)));
    assert_that!(v.contains_key("stt.vad.pause")).is_true();
}

#[test]
fn write_config_key_goes_to_config_toml_for_next_launch() {
    let p = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write("history.limit", &json!(42))).is_equal_to(Ok(Applied::NextLaunch));
    let text = std::fs::read_to_string(p.settings.config_file()).unwrap();
    assert_that!(text.contains("limit = 42")).is_true();
}

#[test]
fn write_dictation_key_without_dictation_is_an_error() {
    let p = platform();
    let port = Arc::new(AppSettings::new(&p, None));
    assert_that!(port.write("dictation.model", &json!("base"))).is_err();
}
