use std::path::{Path, PathBuf};

use oku_slides_platform::config::{AgentPreset, Builtin};
use speculoos::prelude::*;

use super::Launch;

/// The process folder, for presets that never launch here.
fn process() -> PathBuf {
    PathBuf::from("/nonexistent/agent-process")
}

#[test]
fn launch_for_preset_external_keeps_its_command() {
    let p = AgentPreset::new("Claude", "npx", vec!["-y".into()]);
    let l = Launch::for_preset(&p, None, process()).unwrap();
    assert_that!(l.command.as_str()).is_equal_to("npx");
    assert_that!(l.args).is_equal_to(vec!["-y".to_owned()]);
}

#[test]
fn launch_for_preset_builtin_runs_this_program_with_the_flag() {
    let p = AgentPreset::builtin("Local", Builtin::Local);
    let l = Launch::for_preset(&p, Some(Path::new("/Apps/Oku")), process()).unwrap();
    assert_that!(l.command.as_str()).is_equal_to("/Apps/Oku");
    assert_that!(l.args).is_equal_to(vec![oku_slides_platform::LOCAL_AGENT_FLAG.to_owned()]);
}

#[test]
fn launch_for_preset_builtin_without_program_is_none() {
    let p = AgentPreset::builtin("Local", Builtin::Local);
    assert_that!(Launch::for_preset(&p, None, process())).is_none();
}

#[test]
fn launch_for_preset_keeps_its_process_dir() {
    let p = AgentPreset::new("Claude", "npx", vec!["-y".into()]);
    let l = Launch::for_preset(&p, None, process()).unwrap();
    assert_that!(l.process_dir).is_equal_to(process());
}
