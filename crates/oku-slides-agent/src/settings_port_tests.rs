use std::sync::Arc;

use oku_slides_mcp::{Applied, SettingsPort};
use oku_slides_session::{Platform, Settings};
use serde_json::json;
use speculoos::prelude::*;

use super::AppSettings;

/// Une plateforme sur un dossier de données neuf (t-273), à garder vivant
/// autant qu'elle.
fn platform() -> (Platform, tempfile::TempDir) {
    let mut settings = Settings::defaults().unwrap();
    let dir = tempfile::Builder::new().prefix("oku-slides-settings-port-").tempdir().unwrap();
    settings.paths.data = Some(dir.path().to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    (Platform::with_settings(settings).unwrap(), dir)
}

#[test]
fn read_exposes_history_limit_and_vad_keys() {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    let v = port.read();
    assert_that!(v.get("history.limit").cloned()).is_equal_to(Some(json!(500)));
    assert_that!(v.contains_key("stt.vad.pause")).is_true();
}

#[test]
fn write_config_key_goes_to_config_toml_for_next_launch() {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write("history.limit", &json!(42))).is_equal_to(Ok(Applied::NextLaunch));
    let text = std::fs::read_to_string(p.settings.config_file()).unwrap();
    assert_that!(text.contains("limit = 42")).is_true();
}

#[test]
fn write_dictation_key_without_dictation_is_an_error() {
    let (p, _dir) = platform();
    let port = Arc::new(AppSettings::new(&p, None));
    assert_that!(port.write("dictation.model", &json!("base"))).is_err();
}
