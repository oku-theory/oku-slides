use std::path::{Path, PathBuf};

use oku_slides_platform::config::{AgentPreset, Builtin};
use speculoos::prelude::*;

use super::Launch;

#[test]
fn launch_for_preset_external_keeps_its_command() {
    let p = AgentPreset::new("Claude", "npx", vec!["-y".into()]);
    let l = Launch::for_preset(&p, None, PathBuf::from("/tmp")).unwrap();
    assert_that!(l.command.as_str()).is_equal_to("npx");
    assert_that!(l.args).is_equal_to(vec!["-y".to_owned()]);
}

#[test]
fn launch_for_preset_builtin_runs_this_program_with_the_flag() {
    let p = AgentPreset::builtin("Local", Builtin::Local);
    let l = Launch::for_preset(&p, Some(Path::new("/Apps/Oku")), PathBuf::from("/tmp")).unwrap();
    assert_that!(l.command.as_str()).is_equal_to("/Apps/Oku");
    assert_that!(l.args).is_equal_to(vec![oku_slides_platform::LOCAL_AGENT_FLAG.to_owned()]);
}

#[test]
fn launch_for_preset_builtin_without_program_is_none() {
    let p = AgentPreset::builtin("Local", Builtin::Local);
    assert_that!(Launch::for_preset(&p, None, PathBuf::from("/tmp"))).is_none();
}
