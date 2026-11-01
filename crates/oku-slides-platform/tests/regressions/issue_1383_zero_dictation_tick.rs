//! t-1383: `stt.tick_ms = 0` was accepted, and the dictation loop then spun without a pause.

use oku_slides_platform::config::Settings;
use speculoos::prelude::*;

fn env(vars: &[(&str, &str)]) -> config::Environment {
    let vars = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    config::Environment::with_prefix("OKU_SLIDES")
        .prefix_separator("_")
        .separator("__")
        .try_parsing(true)
        .source(Some(vars))
}

#[test]
fn issue_1383_zero_dictation_tick_is_refused_when_settings_load() {
    assert_that!(Settings::from_sources(None, env(&[("OKU_SLIDES_STT__TICK_MS", "0")]))).is_err();
}

#[test]
fn issue_1383_positive_dictation_tick_loads() {
    let s = Settings::from_sources(None, env(&[("OKU_SLIDES_STT__TICK_MS", "20")]))
        .unwrap_or_else(|e| panic!("settings: {e}"));
    assert_that!(s.stt.tick_ms.get()).is_equal_to(20);
}
