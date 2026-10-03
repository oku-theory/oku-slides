use std::collections::HashMap;
use std::path::PathBuf;

use speculoos::prelude::*;

use super::*;

fn env(vars: &[(&str, &str)]) -> config::Environment {
    let map: HashMap<String, String> = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    config::Environment::with_prefix("OKU_SLIDES")
        .prefix_separator("_")
        .separator("__")
        .try_parsing(true)
        .source(Some(map))
}

#[test]
fn defaults_embedded_file_gives_documented_values() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.history.limit).is_equal_to(500);
    assert_that!(s.resume.keep_failed).is_equal_to(3);
    assert_that!((s.serve.host.as_str(), s.serve.port)).is_equal_to(("127.0.0.1", 4870));
    assert_that!(s.stt.tick_ms).is_equal_to(50);
    assert_that!(s.stt.vad.onset_frames).is_equal_to(3);
    assert_that!(s.paths).is_equal_to(PathSettings::default());
    // Sans réglage, la langue du système.
    assert_that!(s.i18n.locale).is_none();
    // Sans réglage, le schéma du système.
    assert_that!(s.appearance.scheme).is_none();
}

#[test]
fn defaults_sources_max_replica_is_32_mib_in_bytes() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.sources.max_replica_mib).is_equal_to(32);
    assert_that!(s.sources.max_replica_bytes()).is_equal_to(32 * 1024 * 1024);
}

#[test]
fn defaults_sources_max_sources_is_256_mib_in_bytes() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.sources.max_sources_mib).is_equal_to(256);
    assert_that!(s.sources.max_sources_bytes()).is_equal_to(256 * 1024 * 1024);
}

#[test]
fn max_sources_bytes_saturates_instead_of_overflowing() {
    let mut s = Settings::defaults().unwrap();
    s.sources.max_sources_mib = usize::MAX;
    assert_that!(s.sources.max_sources_bytes()).is_equal_to(usize::MAX);
}

#[test]
fn from_sources_nested_env_var_overrides_default() {
    let s = Settings::from_sources(
        None,
        env(&[
            ("OKU_SLIDES_STT__VAD__PAUSE", "0.9"),
            ("OKU_SLIDES_SERVE__PORT", "5000"),
            ("OKU_SLIDES_PATHS__MODELS", "/tmp/m"),
        ]),
    )
    .unwrap();
    assert_that!(s.stt.vad.pause).is_equal_to(0.9);
    assert_that!(s.serve.port).is_equal_to(5000);
    assert_that!(s.paths.models()).is_equal_to(PathBuf::from("/tmp/m"));
}

#[test]
fn from_sources_invalid_env_value_returns_error() {
    assert_that!(Settings::from_sources(None, env(&[("OKU_SLIDES_SERVE__PORT", "pas-un-port")]))).is_err();
}

#[test]
fn from_sources_missing_file_is_ignored_and_present_file_overrides() {
    let dir = crate::test_support::fresh_dir("oku-slides-config-");
    let file = dir.path().join("config.toml");
    assert_that!(Settings::from_sources(Some(&file), env(&[])).unwrap().history.limit).is_equal_to(500);
    std::fs::write(&file, "[history]\nlimit = 20\n").unwrap();
    assert_that!(Settings::from_sources(Some(&file), env(&[])).unwrap().history.limit).is_equal_to(20);
}

#[test]
fn paths_without_overrides_live_under_data_dir() {
    let paths = PathSettings { data: Some(PathBuf::from("/d")), ..PathSettings::default() };
    assert_that!(paths.models()).is_equal_to(PathBuf::from("/d/models"));
    assert_that!(paths.templates()).is_equal_to(PathBuf::from("/d/templates"));
}

#[test]
fn decks_configured_path_wins() {
    let paths = PathSettings { decks: Some(PathBuf::from("/mes/decks")), ..PathSettings::default() };
    assert_that!(paths.decks()).is_equal_to(PathBuf::from("/mes/decks"));
}

#[test]
fn decks_default_is_oku_slides_in_documents() {
    let paths = PathSettings { data: Some(PathBuf::from("/d")), ..PathSettings::default() };
    assert_that!(paths.decks().ends_with("Documents/Oku Slides")).is_true();
}

#[test]
fn app_id_names_oku_slides() {
    assert_that!(APP_ID).is_equal_to("me.dartiguenave.oku-slides");
}

#[test]
fn defaults_agent_runs_claude_agent_acp_through_npx() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.agent.command.as_str()).is_equal_to("npx");
    assert_that!(s.agent.args.clone())
        .is_equal_to(vec!["-y".to_owned(), "@agentclientprotocol/claude-agent-acp@0.81.2".to_owned()]);
}

#[test]
fn env_agent_args_split_on_spaces() {
    let env = Settings::environment().source(Some(std::collections::HashMap::from([
        ("OKU_SLIDES_AGENT__COMMAND".to_owned(), "/bin/faux".to_owned()),
        ("OKU_SLIDES_AGENT__ARGS".to_owned(), "--scenario e2e.json".to_owned()),
    ])));
    let s = Settings::from_sources(None, env).unwrap();
    assert_that!(s.agent.command.as_str()).is_equal_to("/bin/faux");
    assert_that!(s.agent.args.clone()).is_equal_to(vec!["--scenario".to_owned(), "e2e.json".to_owned()]);
}

#[test]
fn author_name_setting_wins_over_system_user() {
    let a = AuthorSettings { name: Some("Julie".into()) };
    assert_that!(a.resolved()).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn blank_author_name_falls_back_to_system_user() {
    let a = AuthorSettings { name: Some("  ".into()) };
    let expected = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).ok().filter(|u| !u.trim().is_empty());
    assert_that!(a.resolved()).is_equal_to(expected);
}

#[test]
fn default_settings_have_no_author_name() {
    assert_that!(Settings::defaults().unwrap().author.name).is_none();
}

#[test]
fn settings_default_render_limits_match_config_defaults() {
    let s = Settings::defaults().unwrap();
    assert_that!((s.render.timeout_ms, s.render.memory_mib, s.render.max_frame_mib)).is_equal_to((10_000, 2048, 512));
}

#[test]
fn render_settings_memory_bytes_converts_mib() {
    let r = RenderSettings { timeout_ms: 1, memory_mib: 3, max_frame_mib: 1 };
    assert_that!(r.memory_bytes()).is_equal_to(3 * 1024 * 1024);
}

#[test]
fn render_settings_timeout_is_in_milliseconds() {
    let r = RenderSettings { timeout_ms: 1500, memory_mib: 1, max_frame_mib: 1 };
    assert_that!(r.timeout()).is_equal_to(std::time::Duration::from_millis(1500));
}

#[test]
fn render_settings_max_frame_bytes_converts_mib() {
    let r = RenderSettings { timeout_ms: 1, memory_mib: 1, max_frame_mib: 512 };
    assert_that!(r.max_frame_bytes()).is_equal_to(512 * 1024 * 1024);
}

#[test]
fn platform_with_settings_renders_in_process() {
    let p = crate::Platform::with_settings(Settings::defaults().unwrap()).unwrap();
    assert_that!(p.render_launch).is_equal_to(crate::RenderLaunch::InProcess);
}

#[test]
fn platform_with_render_launch_replaces_the_launch() {
    let program = std::path::PathBuf::from("/bin/x");
    let p = crate::Platform::with_settings(Settings::defaults().unwrap())
        .unwrap()
        .with_render_launch(crate::RenderLaunch::Process { program: program.clone() });
    assert_that!(p.render_launch).is_equal_to(crate::RenderLaunch::Process { program });
}

#[test]
fn render_timeout_env_override_changes_the_setting() {
    let s = Settings::from_sources(None, env(&[("OKU_SLIDES_RENDER__TIMEOUT_MS", "5000")])).unwrap();
    assert_that!(s.render.timeout_ms).is_equal_to(5000);
}

#[test]
fn render_launch_for_a_worker_process_is_in_process() {
    let program = PathBuf::from("/bin/x");
    assert_that!(crate::render_launch_for(program.clone(), false))
        .is_equal_to(crate::RenderLaunch::Process { program: program.clone() });
    // Un travailleur qui n'a pas appelé `exit_if_worker` ne relance jamais de fils.
    assert_that!(crate::render_launch_for(program, true)).is_equal_to(crate::RenderLaunch::InProcess);
}

#[test]
fn render_worker_env_variable_does_not_break_settings() {
    let s = Settings::from_sources(None, env(&[(crate::RENDER_WORKER_ENV, "1")]));
    assert_that!(s.is_ok()).is_true();
}
