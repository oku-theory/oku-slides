use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

fn env(vars: &[(&str, &str)]) -> config::Environment {
    let map: HashMap<String, String> = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    config::Environment::with_prefix("ECHO").prefix_separator("_").separator("__").try_parsing(true).source(Some(map))
}

#[test]
fn defaults_embedded_file_gives_documented_values() {
    let s = Settings::defaults().unwrap();
    assert_eq!(s.history.limit, 500);
    assert_eq!((s.serve.host.as_str(), s.serve.port), ("127.0.0.1", 4870));
    assert_eq!(s.stt.tick_ms, 50);
    assert_eq!(s.stt.vad.onset_frames, 3);
    assert_eq!(s.paths, PathSettings::default());
    assert_eq!(s.i18n.locale, None);
}

#[test]
fn from_sources_nested_env_var_overrides_default() {
    let s = Settings::from_sources(
        None,
        env(&[("ECHO_STT__VAD__PAUSE", "0.9"), ("ECHO_SERVE__PORT", "5000"), ("ECHO_PATHS__MODELS", "/tmp/m")]),
    )
    .unwrap();
    assert_eq!(s.stt.vad.pause, 0.9);
    assert_eq!(s.serve.port, 5000);
    assert_eq!(s.paths.models(), PathBuf::from("/tmp/m"));
}

#[test]
fn from_sources_invalid_env_value_returns_error() {
    assert!(Settings::from_sources(None, env(&[("ECHO_SERVE__PORT", "pas-un-port")])).is_err());
}

#[test]
fn from_sources_missing_file_is_ignored_and_present_file_overrides() {
    let dir = std::env::temp_dir().join(format!("echo-config-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("config.toml");
    assert_eq!(Settings::from_sources(Some(&file), env(&[])).unwrap().history.limit, 500);
    std::fs::write(&file, "[history]\nlimit = 20\n").unwrap();
    assert_eq!(Settings::from_sources(Some(&file), env(&[])).unwrap().history.limit, 20);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn paths_without_overrides_live_under_data_dir() {
    let paths = PathSettings { data: Some(PathBuf::from("/d")), ..PathSettings::default() };
    assert_eq!(paths.models(), PathBuf::from("/d/models"));
    assert_eq!(paths.templates(), PathBuf::from("/d/templates"));
}
