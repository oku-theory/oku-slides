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
    let input: Vec<f32> = (0..48_000u16).map(|i| (f32::from(i) / 100.0).sin()).collect();
    let mut r = audio::Resampler::new(48_000);
    let mut out = Vec::new();
    for chunk in input.chunks(997) {
        r.push(chunk, &mut out);
    }
    let whole = resample(&input, 48_000, SAMPLE_RATE);
    assert_that!(out.len().abs_diff(whole.len())).is_less_than_or_equal_to(1);
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
    assert_that!(s.models).has_length(4);
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(Config {
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
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(Config { language: Some("xx".into()), ..stt.status().config });
    assert_that!(stt.status().models[0].available).is_false();
}

#[test]
fn configure_always_writes_current_version() {
    let dir = fresh_dir("apple-version");
    let stt = apple_stt(dir.path(), Availability::Ready);
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(Config { version: 0, ..stt.status().config });
    assert_that!(stt.status().config.version).is_equal_to(CONFIG_VERSION);
}

#[test]
fn start_apple_with_unknown_language_reports_error() {
    let dir = fresh_dir("apple-start-xx");
    let stt = apple_stt(dir.path(), Availability::Ready);
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(Config { language: Some("xx".into()), ..stt.status().config });
    stt.start(Vec::new());
    assert_that!(stt.status().error.as_deref())
        .is_equal_to(Some("Le moteur d'Apple ne prend pas en charge cette langue (xx)"));
}

fn api_config(url: &str, key_env: &str) -> Config {
    Config {
        model: Model::Api,
        api: ApiConfig { url: url.into(), model: "whisper-1".into(), key_env: key_env.into() },
        ..Config::default()
    }
}

#[test]
fn status_api_without_key_is_listed_not_installed() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config("http://127.0.0.1:9/v1", "OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a"));
    let api = stt.status().models.into_iter().find(|m| m.id == Model::Api).unwrap();
    assert_that!(api.installed).is_false();
}

#[test]
fn start_api_without_key_reports_error_and_stays_idle() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config("http://127.0.0.1:9/v1", "OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a"));
    stt.start(Vec::new());
    let s = stt.status();
    assert_that!(s.state).is_equal_to(State::Idle);
    assert_that!(s.error.unwrap()).contains("OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a");
}

#[test]
fn transcribe_loop_api_commits_the_api_text() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    let (url, _calls) = crate::test_support::fake_api(200, r#"{"text":"Bonjour."}"#);
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config(&url, "HOME"));
    let session = {
        let mut g = stt.state();
        g.phrases.push(None);
        g.state = State::Transcribing;
        g.session
    };
    let (jobs, rx) = mpsc::channel();
    jobs.send(Job::Final { index: 0, audio: vec![0.0; 1600] }).unwrap();
    jobs.send(Job::Done).unwrap();
    stt.transcribe_loop(session, &rx, &Arc::new(AtomicBool::new(false)));
    assert_that!(stt.status().text).is_equal_to(Some("Bonjour.".to_owned()));
}

#[test]
fn api_model_sends_no_previews() {
    assert_that!(Model::Api.previews()).is_false();
}

#[test]
fn config_without_api_section_loads_default_api() {
    let c: Config = serde_json::from_str(r#"{"model":"turbo"}"#).unwrap();
    assert_that!(c.api).is_equal_to(ApiConfig::default());
}

#[test]
fn engine_api_with_negative_timeout_returns_model_error() {
    let dir = fresh_dir("oku-slides-stt-");
    let mut settings = oku_slides_platform::config::Settings::defaults().unwrap();
    settings.paths.models = Some(dir.path().to_path_buf());
    settings.stt.apple = false;
    settings.stt.api_timeout_secs = -1.0;
    let stt = Stt::new(&Platform::with_settings(settings).unwrap());
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config("http://127.0.0.1:9/v1", "HOME"));
    assert_that!(stt.engine().map(|_| ()).unwrap_err().kind()).is_equal_to(SttErrorKind::Model);
}

#[test]
fn engine_api_url_change_returns_a_new_engine() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config("http://127.0.0.1:9/a", "HOME"));
    let first = stt.engine().unwrap();
    assert_that!(Arc::ptr_eq(&first, &stt.engine().unwrap())).is_true();
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config("http://127.0.0.1:9/b", "HOME"));
    assert_that!(Arc::ptr_eq(&first, &stt.engine().unwrap())).is_false();
}

#[test]
fn configure_while_idle_returns_true() {
    let dir = fresh_dir("configure-idle");
    let stt = Stt::new(&platform(dir.path()));
    assert_that!(stt.configure(Config { auto_stop: false, ..stt.status().config })).is_true();
}

#[test]
fn download_of_an_installed_model_returns_false() {
    let dir = fresh_dir("download-installed");
    let installed = Model::Base.path(dir.path());
    std::fs::create_dir_all(&installed).unwrap();
    std::fs::write(installed.join(oku_slides_platform::models::COMPLETE), b"").unwrap();
    let stt = Stt::new(&platform(dir.path()));
    assert_that!(stt.download(Model::Base, false)).is_false();
}

#[test]
fn download_of_the_api_model_returns_false() {
    let dir = fresh_dir("download-api");
    let stt = Stt::new(&platform(dir.path()));
    assert_that!(stt.download(Model::Api, false)).is_false();
}

#[test]
fn api_model_installed_follows_the_key_variable() {
    let dir = fresh_dir("api-installed");
    let stt = Stt::new(&platform(dir.path()));
    let installed = |key_env: &str| {
        // Dictée au repos : le résultat de `configure` est sans objet ici.
        let _applied = stt.configure(api_config("http://127.0.0.1:9/v1", key_env));
        stt.status().models.into_iter().find(|m| m.id == Model::Api).map(|m| m.installed)
    };
    assert_that!(installed("HOME")).is_equal_to(Some(true));
    assert_that!(installed("OKU_T1146_UNSET_7F3A")).is_equal_to(Some(false));
}
