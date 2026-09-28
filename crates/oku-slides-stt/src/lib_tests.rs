use speculoos::prelude::*;

use super::*;

/// Une plateforme isolée : ses modèles dans `dir`.
fn platform(dir: &Path) -> Platform {
    let mut settings = oku_slides_platform::config::Settings::defaults().unwrap();
    settings.paths.models = Some(dir.to_path_buf());
    Platform::with_settings(settings).unwrap()
}

#[test]
fn clean_silence_marker_returns_empty() {
    assert_that!(clean("  [BLANK_AUDIO] ")).is_equal_to(String::new());
}

#[test]
fn clean_extra_spaces_collapses_them() {
    assert_that!(clean(" Bonjour   à tous. ")).is_equal_to("Bonjour à tous.".to_owned());
}

#[test]
fn resample_48k_to_16k_keeps_duration_and_level() {
    let one_second = vec![0.5f32; 48_000];
    let out = resample(&one_second, 48_000, SAMPLE_RATE);
    assert_that!(out).has_length(16_000);
    assert_that!(out.iter().all(|&s| (s - 0.5).abs() < 1e-6)).is_true();
}

#[test]
fn resampler_chunked_input_matches_whole_resample() {
    let input: Vec<f32> = (0..48_000).map(|i| (i as f32 / 100.0).sin()).collect();
    let mut r = audio::Resampler::new(48_000);
    let mut out = Vec::new();
    for chunk in input.chunks(997) {
        r.push(chunk, &mut out);
    }
    let whole = resample(&input, 48_000, SAMPLE_RATE);
    assert_that!((out.len() as i64 - whole.len() as i64).abs()).is_less_than_or_equal_to(1);
    assert_that!(out.iter().zip(&whole).all(|(a, b)| (a - b).abs() < 1e-4)).is_true();
}

#[test]
fn stt_default_config_is_turbo_auto_language_auto_stop_and_persists() {
    let dir = crate::test_support::fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    let s = stt.status();
    assert_that!(s.config.model).is_equal_to(Model::Turbo);
    assert_that!(s.config.language).is_none();
    assert_that!(s.config.auto_stop).is_true();
    assert_that!(s.models).has_length(3);
    stt.configure(Config { model: Model::Large, language: Some("fr".into()), auto_stop: false, silence: 2.0 });
    let back = Stt::new(&platform(dir.path())).status().config;
    assert_that!(back.model).is_equal_to(Model::Large);
    assert_that!(back.auto_stop).is_false();
}

#[test]
fn stt_request_partial_configure_fills_defaults() {
    let r: SttRequest = serde_json::from_str(r#"{"cmd":"configure","model":"base","language":null}"#).unwrap();
    match r {
        SttRequest::Configure(c) => {
            assert_that!(c.model).is_equal_to(Model::Base);
            assert_that!(c.auto_stop).is_true();
        }
        _ => panic!(),
    }
}
