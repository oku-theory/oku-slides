use oku_slides_platform::Platform;
use oku_slides_platform::config::{LocalEngine, Settings};
use speculoos::prelude::*;

use super::build_engine;
use crate::error::Inner;

/// Une plateforme dans un dossier neuf, réglages modifiés par `edit`.
fn platform(edit: impl FnOnce(&mut Settings, &std::path::Path)) -> (tempfile::TempDir, Platform) {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(dir.path().to_owned());
    edit(&mut settings, dir.path());
    (dir, Platform::with_settings(settings).unwrap())
}

#[cfg(debug_assertions)]
#[test]
fn build_engine_fake_without_script_is_no_script() {
    let (_d, p) = platform(|s, _| s.local_agent.engine = LocalEngine::Fake);
    let err = build_engine(&p).err().unwrap();
    assert_that!(matches!(err.0, Inner::NoScript { .. })).is_true();
}

#[cfg(debug_assertions)]
#[test]
fn build_engine_fake_with_script_builds() {
    let (_d, p) = platform(|s, dir| {
        let script = dir.join("s.json");
        std::fs::write(&script, r#"{"turns":[]}"#).unwrap();
        s.local_agent.engine = LocalEngine::Fake;
        s.local_agent.fake_script = Some(script);
    });
    assert_that!(build_engine(&p).is_ok()).is_true();
}

#[test]
fn build_engine_gguf_without_model_is_unavailable() {
    let (_d, p) = platform(|s, _| s.local_agent.engine = LocalEngine::Gguf);
    let err = build_engine(&p).err().unwrap();
    assert_that!(matches!(err.0, Inner::Unavailable { .. })).is_true();
}
