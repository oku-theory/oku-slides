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
    assert_that!((s.serve.host.as_str(), s.serve.port)).is_equal_to(("127.0.0.1", 4870));
    assert_that!(s.stt.tick_ms).is_equal_to(50);
    assert_that!(s.stt.vad.onset_frames).is_equal_to(3);
    assert_that!(s.paths).is_equal_to(PathSettings::default());
    // Sans réglage, la langue du système.
    assert_that!(s.i18n.locale).is_none();
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
    let dir = std::env::temp_dir().join(format!("oku-slides-config-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("config.toml");
    assert_that!(Settings::from_sources(Some(&file), env(&[])).unwrap().history.limit).is_equal_to(500);
    std::fs::write(&file, "[history]\nlimit = 20\n").unwrap();
    assert_that!(Settings::from_sources(Some(&file), env(&[])).unwrap().history.limit).is_equal_to(20);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn paths_without_overrides_live_under_data_dir() {
    let paths = PathSettings { data: Some(PathBuf::from("/d")), ..PathSettings::default() };
    assert_that!(paths.models()).is_equal_to(PathBuf::from("/d/models"));
    assert_that!(paths.templates()).is_equal_to(PathBuf::from("/d/templates"));
}

#[test]
fn app_id_names_oku_slides() {
    assert_that!(APP_ID).is_equal_to("me.dartiguenave.oku-slides");
}
