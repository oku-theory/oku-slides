use std::path::PathBuf;

use oku_slides_platform::config::AgentPreset;
use speculoos::prelude::*;

use super::Launch;

/// The process folder, for presets that never launch here.
fn process() -> PathBuf {
    PathBuf::from("/nonexistent/agent-process")
}

#[test]
fn launch_for_preset_external_keeps_its_command() {
    let p = AgentPreset::new("Claude", "npx", vec!["-y".into()]);
    let l = Launch::for_preset(&p, process());
    assert_that!(l.command.as_str()).is_equal_to("npx");
    assert_that!(l.args).is_equal_to(vec!["-y".to_owned()]);
}

#[test]
fn launch_for_preset_keeps_its_process_dir() {
    let p = AgentPreset::new("Claude", "npx", vec!["-y".into()]);
    let l = Launch::for_preset(&p, process());
    assert_that!(l.process_dir).is_equal_to(process());
}
