use std::collections::BTreeMap;

use insta::assert_json_snapshot;
use oku_slides_i18n::Localizer;
use oku_slides_platform::config::{AgentPreset, Builtin};
use speculoos::prelude::*;

use rstest::rstest;

use super::{LocalPanel, PICKER_ID, picker_surface};
use oku_slides_session::agent::Surface;

fn presets() -> BTreeMap<String, AgentPreset> {
    BTreeMap::from([
        ("claude".to_owned(), AgentPreset::new("Claude", "npx", vec!["-y".into(), "claude-agent-acp".into()])),
        ("gemini".to_owned(), AgentPreset::new("Gemini", "npx", vec!["-y".into(), "gemini".into(), "--acp".into()])),
    ])
}

#[test]
fn picker_surface_idle_offers_every_preset_with_the_active_one_chosen() {
    let s = picker_surface(&presets(), "gemini", false, false, None, &Localizer::new(Some("fr")));
    assert_that!(s.id.as_str()).is_equal_to(PICKER_ID);
    assert_json_snapshot!(s);
}

#[test]
fn picker_surface_busy_warns_that_the_turn_will_stop() {
    assert_json_snapshot!(picker_surface(&presets(), "claude", true, false, None, &Localizer::new(Some("fr"))));
}

#[test]
fn picker_surface_locked_has_no_action() {
    let s = picker_surface(&presets(), "claude", false, true, None, &Localizer::new(Some("fr")));
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json.contains("\"select\"")).is_false();
    assert_json_snapshot!(s);
}

#[test]
fn picker_builtin_preset_has_no_command_hint() {
    let presets = BTreeMap::from([("local".to_owned(), AgentPreset::builtin("Local", Builtin::Local))]);
    let s = picker_surface(&presets, "local", false, false, None, &Localizer::new(Some("fr")));
    let json = serde_json::to_value(&s).unwrap();
    assert_that!(json.to_string().contains("hint")).is_false();
}

/// Les composants du panneau (message `updateComponents`), en JSON.
fn components(s: &Surface) -> Vec<serde_json::Value> {
    let json = serde_json::to_value(&s.messages).unwrap();
    let update = json.as_array().unwrap().iter().find(|m| m.get("updateComponents").is_some()).unwrap();
    update["updateComponents"]["components"].as_array().unwrap().clone()
}

fn local_presets() -> BTreeMap<String, AgentPreset> {
    BTreeMap::from([("local".to_owned(), AgentPreset::builtin("Local", Builtin::Local))])
}

#[rstest]
#[case::needs(LocalPanel::NeedsDownload { reason: "r".into() }, &["local-reason", "download"])]
#[case::running(LocalPanel::Running { percent: 42 }, &["progress", "cancel-download"])]
#[case::failed(
    LocalPanel::Failed { reason: "r".into(), error: "e".into() },
    &["local-reason", "download-error", "download"]
)]
fn picker_local_panel_shows_its_components(#[case] panel: LocalPanel, #[case] ids: &[&str]) {
    let s = picker_surface(&local_presets(), "local", false, false, Some(&panel), &Localizer::new(Some("fr")));
    let components = components(&s);
    let shown: Vec<&str> = components.iter().filter_map(|c| c["id"].as_str()).collect();
    for id in ids {
        assert_that!(shown).contains(*id);
    }
    let root = components.iter().find(|c| c["id"] == "root").unwrap();
    let children: Vec<&str> = root["children"].as_array().unwrap().iter().filter_map(|c| c.as_str()).collect();
    assert_that!(children[2..].to_vec()).is_equal_to(ids.to_vec());
}

#[test]
fn picker_download_button_sends_download_model() {
    let panel = LocalPanel::NeedsDownload { reason: "r".into() };
    let s = picker_surface(&local_presets(), "local", false, false, Some(&panel), &Localizer::new(Some("fr")));
    let components = components(&s);
    let button = components.iter().find(|c| c["id"] == "download").unwrap();
    assert_that!(button["action"]["event"]["name"].as_str()).is_equal_to(Some("download_model"));
    assert_that!(button["label"].as_str()).is_equal_to(Some("Télécharger le modèle"));
}

#[test]
fn picker_running_shows_percent_and_cancel_sends_cancel_download() {
    let panel = LocalPanel::Running { percent: 42 };
    let s = picker_surface(&local_presets(), "local", false, false, Some(&panel), &Localizer::new(Some("fr")));
    let components = components(&s);
    let progress = components.iter().find(|c| c["id"] == "progress").unwrap();
    assert_that!(progress["text"].as_str().unwrap_or_default()).contains("42");
    let cancel = components.iter().find(|c| c["id"] == "cancel-download").unwrap();
    assert_that!(cancel["action"]["event"]["name"].as_str()).is_equal_to(Some("cancel_download"));
}
