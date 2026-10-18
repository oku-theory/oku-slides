use std::collections::BTreeMap;

use insta::assert_json_snapshot;
use oku_slides_i18n::Localizer;
use oku_slides_platform::config::{AgentPreset, Builtin};
use speculoos::prelude::*;

use super::{PICKER_ID, picker_surface};

fn presets() -> BTreeMap<String, AgentPreset> {
    BTreeMap::from([
        ("claude".to_owned(), AgentPreset::new("Claude", "npx", vec!["-y".into(), "claude-agent-acp".into()])),
        ("gemini".to_owned(), AgentPreset::new("Gemini", "npx", vec!["-y".into(), "gemini".into(), "--acp".into()])),
    ])
}

#[test]
fn picker_surface_idle_offers_every_preset_with_the_active_one_chosen() {
    let s = picker_surface(&presets(), "gemini", false, false, &Localizer::new(Some("fr")));
    assert_that!(s.id.as_str()).is_equal_to(PICKER_ID);
    assert_json_snapshot!(s);
}

#[test]
fn picker_surface_busy_warns_that_the_turn_will_stop() {
    assert_json_snapshot!(picker_surface(&presets(), "claude", true, false, &Localizer::new(Some("fr"))));
}

#[test]
fn picker_surface_locked_has_no_action() {
    let s = picker_surface(&presets(), "claude", false, true, &Localizer::new(Some("fr")));
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json.contains("\"select\"")).is_false();
    assert_json_snapshot!(s);
}

#[test]
fn picker_builtin_preset_has_no_command_hint() {
    let presets = BTreeMap::from([("local".to_owned(), AgentPreset::builtin("Local", Builtin::Local))]);
    let s = picker_surface(&presets, "local", false, false, &Localizer::new(Some("fr")));
    let json = serde_json::to_value(&s).unwrap();
    assert_that!(json.to_string().contains("hint")).is_false();
}
