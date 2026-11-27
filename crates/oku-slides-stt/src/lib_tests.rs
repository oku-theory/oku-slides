use speculoos::prelude::*;

use super::*;
use crate::test_support::{fresh_dir, platform_in};

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
fn stt_default_config_is_parakeet_auto_language_auto_stop_and_persists() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    let s = stt.status();
    assert_that!(s.config.model).is_equal_to(Model::Parakeet);
    assert_that!(s.config.language).is_none();
    assert_that!(s.config.auto_stop).is_true();
    assert_that!(s.models).has_length(2);
    // Dictation idle: the result of `configure` does not matter here.
    let _applied = stt.configure(Config {
        model: Model::Api,
        language: Some("fr".into()),
        auto_stop: false,
        silence: 2.0,
        ..Config::default()
    });
    let back = Stt::new(&platform(dir.path())).status().config;
    assert_that!(back.model).is_equal_to(Model::Api);
    assert_that!(back.auto_stop).is_false();
}

#[test]
fn api_prompt_terms_are_joined_with_periods() {
    let vocabulary = vec!["Bilan".to_owned(), "  ".to_owned(), "Q3".to_owned()];
    assert_that!(api_prompt(&vocabulary)).is_equal_to("Bilan. Q3".to_owned());
}

#[test]
fn api_prompt_long_vocabulary_is_cut_at_600_chars() {
    let vocabulary = vec!["é".repeat(700)];
    assert_that!(api_prompt(&vocabulary).chars().count()).is_equal_to(600);
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
fn download_api_model_is_refused() {
    let dir = fresh_dir("api-download");
    let settings = oku_slides_platform::config::Settings::defaults().unwrap();
    let storage = oku_slides_platform::storage::Storage::local().unwrap();
    let err = download(Model::Api, dir.path(), &settings.models, &storage, |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}

fn api_config(url: &str, key_env: &str) -> Config {
    Config {
        model: Model::Api,
        api: ApiConfig {
            url: url.into(),
            model: "gpt-4o-transcribe".into(),
            key_env: key_env.into(),
            ..ApiConfig::default()
        },
        ..Config::default()
    }
}

#[test]
fn status_api_without_key_is_listed_not_installed() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config("http://127.0.0.1:9/v1", "OKU_SLIDES_TEST_NO_SUCH_KEY_7F3A"));
    let api = stt.status().models.into_iter().find(|m| m.id == Model::Api).unwrap();
    assert_that!(api.installed).is_false();
}

#[test]
fn start_api_without_key_reports_error_and_stays_idle() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(api_config("http://127.0.0.1:9/v1", "OKU_SLIDES_TEST_NO_SUCH_KEY_7F3A"));
    stt.start(Vec::new());
    let s = stt.status();
    assert_that!(s.state).is_equal_to(State::Idle);
    assert_that!(s.error.unwrap()).contains("OKU_SLIDES_TEST_NO_SUCH_KEY_7F3A");
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
        g.phrases.push(crate::utterance::Spoken { text: None, start: 0, end: 1600 });
        g.state = State::Transcribing;
        g.session
    };
    let (jobs, rx) = mpsc::channel();
    let epoch = stt.state().epoch;
    jobs.send(Job::Final { index: 0, audio: vec![0.0; 1600], epoch }).unwrap();
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
    let c: Config = serde_json::from_str(r#"{"model":"parakeet"}"#).unwrap();
    assert_that!(c.api).is_equal_to(ApiConfig::default());
}

#[test]
fn engine_api_with_negative_timeout_returns_model_error() {
    let dir = fresh_dir("oku-slides-stt-");
    let mut settings = oku_slides_platform::config::Settings::defaults().unwrap();
    settings.paths.models = Some(dir.path().to_path_buf());
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
    let installed = Model::Parakeet.path(dir.path());
    std::fs::create_dir_all(&installed).unwrap();
    std::fs::write(installed.join(oku_slides_platform::models::COMPLETE), b"").unwrap();
    let stt = Stt::new(&platform(dir.path()));
    assert_that!(stt.download(Model::Parakeet, false)).is_false();
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

/// Parakeet's entry in the status of a dictation in UI `locale`, with `language` chosen.
fn parakeet_info(locale: &str, language: Option<&str>) -> ModelInfo {
    let dir = fresh_dir("parakeet-info");
    let stt = Stt::new(&platform_in(dir.path(), locale));
    // Dictation idle: the result of `configure` does not matter here.
    let _applied = stt.configure(Config { language: language.map(str::to_owned), ..stt.status().config });
    stt.status().models.into_iter().find(|m| m.id == Model::Parakeet).unwrap()
}

#[test]
fn model_info_parakeet_available_for_french() {
    assert_that!(parakeet_info("fr", None).available).is_true();
}

#[test]
fn model_info_parakeet_greyed_for_japanese() {
    assert_that!(parakeet_info("fr", Some("ja")).available).is_false();
}

#[test]
fn model_info_parakeet_uses_ui_locale_when_no_language() {
    assert_that!(parakeet_info("ja", None).available).is_false();
}

#[test]
fn model_info_chosen_language_wins_over_ui_locale() {
    assert_that!(parakeet_info("ja", Some("fr")).available).is_true();
}

#[test]
fn model_info_api_is_available_whatever_the_language() {
    let dir = fresh_dir("api-info-ja");
    let stt = Stt::new(&platform_in(dir.path(), "ja"));
    let api = stt.status().models.into_iter().find(|m| m.id == Model::Api).unwrap();
    assert_that!(api.available).is_true();
}

/// A phrase engine that reports no language, as Parakeet does.
struct NoLanguageEngine;

impl PhraseEngine for NoLanguageEngine {
    fn transcribe(&self, _: &[f32], _: Option<&str>, _: Option<&str>, _: bool) -> Result<Transcript, SttError> {
        Ok(Transcript { text: "Bonjour.".into(), language: None, no_speech: 0.0 })
    }
}

#[test]
fn auto_language_leaves_detected_empty() {
    let dir = fresh_dir("auto-language");
    let stt = Stt::new(&platform_in(dir.path(), "fr"));
    let config = stt.status().config;
    stt.state().engine = Some((engine_key(&config), Arc::new(NoLanguageEngine)));
    let session = {
        let mut g = stt.state();
        g.phrases.push(crate::utterance::Spoken { text: None, start: 0, end: 1600 });
        g.state = State::Transcribing;
        g.session
    };
    let (jobs, rx) = mpsc::channel();
    let epoch = stt.state().epoch;
    jobs.send(Job::Final { index: 0, audio: vec![0.0; 1600], epoch }).unwrap();
    jobs.send(Job::Done).unwrap();
    stt.transcribe_loop(session, &rx, &Arc::new(AtomicBool::new(false)));
    let s = stt.status();
    assert_that!((s.text.as_deref(), s.detected, s.error)).is_equal_to((Some("Bonjour."), None, None));
}
