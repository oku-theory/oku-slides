//! t-1202: `AgentSettings` had public `selected` and `presets` fields, so code (mostly tests)
//! could point `selected` at no preset, and `active()` then panicked. The invariant now
//! holds by construction: every way to build or change the settings checks it.

use std::collections::BTreeMap;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::config::{AgentPreset, AgentSettings, Settings};
use speculoos::prelude::*;

/// Fragment of the unknown-selected rule's English text.
const UNKNOWN_SELECTED: &str = "names no [agent.presets] entry";
/// Fragment of the one-launch rule's English text.
const PRESET_LAUNCH: &str = "needs exactly one of command or builtin";

fn faux() -> AgentPreset {
    AgentPreset::new("Faux", "/bin/faux", Vec::new())
}

fn defaults() -> AgentSettings {
    Settings::defaults().unwrap_or_else(|e| panic!("defaults: {e}")).agent
}

fn ids(agent: &AgentSettings) -> Vec<&str> {
    agent.presets().map(|(id, _)| id).collect()
}

#[test]
fn agent_settings_select_unknown_preset_is_refused_and_keeps_the_active_one() {
    let mut agent = defaults();
    let before = agent.active().clone();
    let err = agent.select("mistral").err().unwrap_or_else(|| panic!("select must fail"));
    assert_that!(err.to_string()).contains(UNKNOWN_SELECTED);
    assert_that!(agent.selected()).is_equal_to("claude");
    assert_that!(agent.active()).is_equal_to(&before);
}

#[test]
fn agent_settings_new_with_unknown_selected_is_refused_by_the_selected_rule() {
    let presets = BTreeMap::from([("faux".to_owned(), faux())]);
    let err = AgentSettings::new("mistral", presets).err().unwrap_or_else(|| panic!("new must fail"));
    assert_that!(err.to_string()).contains(UNKNOWN_SELECTED);
    let fr = Localizer::new(Some("fr"));
    let expected =
        fr.tr_with("agent-settings-unknown-selected", &[("selected", "mistral".into()), ("known", "faux".into())]);
    assert_that!(err.localize(&fr)).is_equal_to(expected);
}

#[test]
fn agent_settings_new_with_preset_without_launch_is_refused_by_the_launch_rule() {
    let presets = BTreeMap::from([("x".to_owned(), AgentPreset::new("X", "", Vec::new()))]);
    let err = AgentSettings::new("x", presets).err().unwrap_or_else(|| panic!("new must fail"));
    assert_that!(err.to_string()).contains(PRESET_LAUNCH);
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr_with("agent-settings-preset-launch", &[("id", "x".into())]));
}

#[test]
fn agent_settings_insert_then_select_makes_the_preset_active() {
    let mut agent = defaults();
    agent.insert_preset("faux", faux()).unwrap_or_else(|e| panic!("insert: {e}"));
    agent.select("faux").unwrap_or_else(|e| panic!("select: {e}"));
    assert_that!(agent.active()).is_equal_to(&faux());
    assert_that!(agent.preset("claude")).is_some();
    assert_that!(ids(&agent)).is_equal_to(vec!["claude", "codex", "faux", "gemini", "local"]);
}

#[test]
fn agent_settings_insert_preset_on_the_active_id_replaces_the_active_agent() {
    let mut agent = defaults();
    let count = agent.presets().count();
    agent.insert_preset("claude", faux()).unwrap_or_else(|e| panic!("insert: {e}"));
    assert_that!(agent.selected()).is_equal_to("claude");
    assert_that!(agent.active()).is_equal_to(&faux());
    assert_that!(agent.presets().count()).is_equal_to(count);
}

#[test]
fn agent_settings_invalid_preset_on_the_active_id_leaves_the_active_agent() {
    let mut agent = defaults();
    let before = agent.active().clone();
    let err = agent
        .insert_preset("claude", AgentPreset::new("X", "", Vec::new()))
        .err()
        .unwrap_or_else(|| panic!("insert must fail"));
    assert_that!(err.to_string()).contains(PRESET_LAUNCH);
    assert_that!(agent.active()).is_equal_to(&before);
}

#[test]
fn agent_settings_presets_keep_id_order_when_the_active_sorts_last() {
    let mut agent = defaults();
    agent.insert_preset("zed", faux()).unwrap_or_else(|e| panic!("insert: {e}"));
    agent.select("zed").unwrap_or_else(|e| panic!("select: {e}"));
    assert_that!(ids(&agent)).is_equal_to(vec!["claude", "codex", "gemini", "local", "zed"]);
}

#[test]
fn agent_settings_select_back_and_forth_keeps_every_preset() {
    let mut agent = defaults();
    let before = agent.clone();
    agent.select("codex").unwrap_or_else(|e| panic!("select codex: {e}"));
    agent.select("claude").unwrap_or_else(|e| panic!("select claude: {e}"));
    assert_that!(agent).is_equal_to(before);
}
