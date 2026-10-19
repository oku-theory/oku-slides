use speculoos::prelude::*;

use super::*;
use crate::stream::{Availability, NoStreamEngine};
use crate::test_support::{ScriptedEngine, french_platform, fresh_dir, platform_in};

/// Une plateforme isolée : ses modèles dans `dir`.
fn platform(dir: &Path) -> Platform {
    let mut settings = oku_slides_platform::config::Settings::defaults().unwrap();
    settings.paths.models = Some(dir.to_path_buf());
    settings.stt.apple = false;
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
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    let s = stt.status();
    assert_that!(s.config.model).is_equal_to(Model::Turbo);
    assert_that!(s.config.language).is_none();
    assert_that!(s.config.auto_stop).is_true();
    assert_that!(s.models).has_length(3);
    stt.configure(Config {
        model: Model::Large,
        language: Some("fr".into()),
        auto_stop: false,
        silence: 2.0,
        ..Config::default()
    });
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

#[test]
fn whisper_prompt_terms_are_joined_with_periods() {
    let vocabulary = vec!["Bilan".to_owned(), "  ".to_owned(), "Q3".to_owned()];
    assert_that!(whisper_prompt(&vocabulary)).is_equal_to("Bilan. Q3".to_owned());
}

#[test]
fn whisper_prompt_long_vocabulary_is_cut_at_600_chars() {
    let vocabulary = vec!["é".repeat(700)];
    assert_that!(whisper_prompt(&vocabulary).chars().count()).is_equal_to(600);
}

#[test]
fn context_of_vocabulary_comes_before_dictated_text() {
    assert_that!(context_of(&["Bilan".to_owned()], " déjà dit ")).is_equal_to("Bilan déjà dit".to_owned());
}

#[test]
fn start_request_reads_vocabulary_list() {
    let req: SttRequest = serde_json::from_str(r#"{"cmd":"start","vocabulary":["Bilan","Q3"]}"#).unwrap();
    assert_that!(matches!(req, SttRequest::Start { vocabulary } if vocabulary == ["Bilan", "Q3"])).is_true();
}

#[test]
fn download_apple_model_is_refused() {
    let dir = fresh_dir("apple-download");
    let settings = oku_slides_platform::config::Settings::defaults().unwrap();
    let storage = oku_slides_platform::storage::Storage::local().unwrap();
    let err = download(Model::Apple, dir.path(), &settings.models, &storage, |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}

fn config(model: Model, version: u32) -> Config {
    Config { model, version, ..Config::default() }
}

#[test]
fn migrate_old_config_switches_to_apple_when_available() {
    assert_that!(migrate(config(Model::Turbo, 0), true, true).model).is_equal_to(Model::Apple);
}

#[test]
fn migrate_old_config_without_apple_keeps_model() {
    assert_that!(migrate(config(Model::Large, 0), false, false).model).is_equal_to(Model::Large);
}

#[test]
fn migrate_old_config_with_unknown_language_keeps_model() {
    let migrated = migrate(config(Model::Turbo, 0), true, false);
    assert_that!((migrated.model, migrated.version)).is_equal_to((Model::Turbo, CONFIG_VERSION));
}

#[test]
fn migrate_current_config_keeps_user_choice() {
    assert_that!(migrate(config(Model::Base, CONFIG_VERSION), true, true).model).is_equal_to(Model::Base);
}

#[test]
fn migrate_apple_on_machine_without_it_falls_back_to_turbo() {
    assert_that!(migrate(config(Model::Apple, CONFIG_VERSION), false, false).model).is_equal_to(Model::Turbo);
}

#[test]
fn migrate_apple_with_unknown_language_keeps_apple() {
    assert_that!(migrate(config(Model::Apple, CONFIG_VERSION), true, false).model).is_equal_to(Model::Apple);
}

#[test]
fn config_json_without_version_reads_version_zero() {
    let c: Config = serde_json::from_str(r#"{"model":"turbo"}"#).unwrap();
    assert_that!(c.version).is_equal_to(0);
}

fn apple_stt(dir: &Path, french: Availability) -> Stt {
    Stt::with_engine(&french_platform(dir, |_| {}), Arc::new(ScriptedEngine::new(french, &[])), None)
}

#[test]
fn new_config_with_interface_language_unknown_to_apple_stays_turbo() {
    let dir = fresh_dir("apple-de");
    let engine = Arc::new(ScriptedEngine::new(Availability::Ready, &[]));
    let stt = Stt::with_engine(&platform_in(dir.path(), "de", |_| {}), engine, None);
    assert_that!(stt.status().config.model).is_equal_to(Model::Turbo);
}

#[test]
fn status_with_stream_engine_lists_apple_first_and_ready() {
    let dir = fresh_dir("apple-status");
    let status = apple_stt(dir.path(), Availability::Ready).status();
    let apple = &status.models[0];
    assert_that!((apple.id, apple.installed, apple.available)).is_equal_to((Model::Apple, true, true));
}

#[test]
fn new_config_with_stream_engine_chooses_apple() {
    let dir = fresh_dir("apple-default");
    assert_that!(apple_stt(dir.path(), Availability::Ready).status().config.model).is_equal_to(Model::Apple);
}

#[test]
fn status_with_apple_auto_language_reports_interface_language() {
    let dir = fresh_dir("apple-detected");
    assert_that!(apple_stt(dir.path(), Availability::Ready).status().detected).is_equal_to(Some("fr".to_owned()));
}

#[test]
fn status_without_stream_engine_omits_apple() {
    let dir = fresh_dir("apple-none");
    let stt = Stt::with_engine(&french_platform(dir.path(), |_| {}), Arc::new(NoStreamEngine), None);
    assert_that!(stt.status().models.iter().any(|m| m.id == Model::Apple)).is_false();
}

#[test]
fn configure_unknown_language_makes_apple_unavailable() {
    let dir = fresh_dir("apple-xx");
    let stt = apple_stt(dir.path(), Availability::Ready);
    stt.configure(Config { language: Some("xx".into()), ..stt.status().config });
    assert_that!(stt.status().models[0].available).is_false();
}

#[test]
fn configure_always_writes_current_version() {
    let dir = fresh_dir("apple-version");
    let stt = apple_stt(dir.path(), Availability::Ready);
    stt.configure(Config { version: 0, ..stt.status().config });
    assert_that!(stt.status().config.version).is_equal_to(CONFIG_VERSION);
}

#[test]
fn start_apple_with_unknown_language_reports_error() {
    let dir = fresh_dir("apple-start-xx");
    let stt = apple_stt(dir.path(), Availability::Ready);
    stt.configure(Config { language: Some("xx".into()), ..stt.status().config });
    stt.start(Vec::new());
    assert_that!(stt.status().error.as_deref())
        .is_equal_to(Some("Le moteur d'Apple ne prend pas en charge cette langue (xx)"));
}
