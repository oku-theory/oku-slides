use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_mcp::{Origin, VoiceApplied, VoiceErrorKind, VoicePort};
use oku_slides_platform::config::{AgentPreset, AgentSettings};
use oku_slides_session::{DictationLanguage, DictationPatch, VoiceEvent};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

/// Une boîte de test et ce qui la garde en vie.
pub(crate) struct T {
    pub voice: Arc<AppVoice>,
    pub stt: Option<Stt>,
    pub data: std::path::PathBuf,
    _rt: tokio::runtime::Runtime,
    _dir: tempfile::TempDir,
}

impl T {
    /// Le choix d'agent écrit dans `config.toml`.
    pub fn saved_agent(&self) -> Option<String> {
        let toml = std::fs::read_to_string(self.data.join("config.toml")).unwrap_or_default();
        toml.lines().find_map(|l| l.strip_prefix("selected = \"")).map(|v| v.trim_end_matches('"').to_owned())
    }

    /// Attend que `cond` tienne, deux secondes au plus ; échoue sinon.
    pub fn until(&self, what: &str, cond: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !cond(self) {
            assert!(Instant::now() < deadline, "attente dépassée : {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn preset(name: &str) -> AgentPreset {
    AgentPreset::new(name, "true", vec![])
}

/// Une plateforme de test à deux agents (`faux` choisi, `bis`), retouchée par `tweak`.
pub(crate) fn build(with_stt: bool, tweak: impl FnOnce(&mut oku_slides_session::Settings)) -> T {
    let dir = tempfile::Builder::new().prefix("oku-voice-").tempdir().unwrap();
    let data = dir.path().to_path_buf();
    let mut settings = oku_slides_session::Settings::defaults().unwrap();
    settings.paths.data = Some(data.clone());
    settings.paths.decks = Some(data.join("decks"));
    let presets = [("faux".to_owned(), preset("Faux")), ("bis".to_owned(), preset("Bis"))].into_iter().collect();
    settings.agent = AgentSettings::new("faux", presets).unwrap();
    settings.i18n.locale = Some("fr".into());
    settings.stt.apple = false;
    tweak(&mut settings);
    let platform = oku_slides_session::Platform::with_settings(settings).unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let choice =
        Arc::new(AgentChoice::new(&platform, crate::Link::new(AgentChoice::launch_of(&preset("Faux"), &platform))));
    let stt = with_stt.then(|| Stt::new(&platform));
    T { voice: AppVoice::new(stt.clone(), choice, rt.handle().clone()), stt, data, _rt: rt, _dir: dir }
}

pub(crate) fn voice_with_stt() -> T {
    build(true, |_| {})
}

fn voice_without_stt() -> T {
    build(false, |_| {})
}

fn voice_locked() -> T {
    build(true, |s| s.agent.locked = true)
}

fn dictation(p: DictationPatch) -> VoiceEvent {
    VoiceEvent::Dictation(p)
}

fn agent_origin(tab: &str, confirmed: bool) -> Origin {
    Origin::Agent { tab: tab.into(), confirmed }
}

#[rstest]
#[case(DictationPatch { model: Some("nope".into()), ..Default::default() }, VoiceErrorKind::UnknownModel)]
#[case(DictationPatch { silence: Some(0.2), ..Default::default() }, VoiceErrorKind::BadSilence)]
#[case(DictationPatch { silence: Some(30.0), ..Default::default() }, VoiceErrorKind::BadSilence)]
#[case(DictationPatch { language: Some(DictationLanguage::Code("xx".into())), ..Default::default() }, VoiceErrorKind::BadLanguage)]
#[case(DictationPatch { api_url: Some("http://evil.example/v1".into()), ..Default::default() }, VoiceErrorKind::BadUrl)]
#[case(DictationPatch { api_key_env: Some("sk-abc-123".into()), ..Default::default() }, VoiceErrorKind::BadKeyEnv)]
#[case(DictationPatch { api_model: Some("m\r\nx".into()), ..Default::default() }, VoiceErrorKind::BadApiModel)]
#[case(DictationPatch { api_key_env: Some(String::new()), ..Default::default() }, VoiceErrorKind::BadKeyEnv)]
#[case(DictationPatch { api_version: Some("2025/03".into()), ..Default::default() }, VoiceErrorKind::BadApiVersion)]
#[case(DictationPatch { api_headers: Some("Authorization: Basic x".into()), ..Default::default() }, VoiceErrorKind::BadApiHeaders)]
#[case(DictationPatch { api_headers: Some("Host: evil.example".into()), ..Default::default() }, VoiceErrorKind::BadApiHeaders)]
#[case(DictationPatch { api_headers: Some("X-A: $lower".into()), ..Default::default() }, VoiceErrorKind::BadApiHeaders)]
fn apply_invalid_patch_from_the_user_is_refused(#[case] patch: DictationPatch, #[case] kind: VoiceErrorKind) {
    let t = voice_with_stt();
    assert_that!(t.voice.apply(dictation(patch), Origin::User).map_err(|e| e.kind())).is_err_containing(kind);
}

#[rstest]
#[case(DictationPatch { api_url: Some("https://api.groq.com/openai/v1".into()), ..Default::default() })]
#[case(DictationPatch { api_key_env: Some("GROQ_API_KEY".into()), ..Default::default() })]
#[case(DictationPatch { api_version: Some("2024-10-21".into()), ..Default::default() })]
#[case(DictationPatch { api_headers: Some("X-Token: $GITHUB_TOKEN".into()), ..Default::default() })]
fn check_api_target_from_the_agent_is_reserved_to_the_user(#[case] patch: DictationPatch) {
    let t = voice_with_stt();
    let r = t.voice.check(&dictation(patch), &agent_origin("t-1", false));
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::ApiReservedToUser);
}

#[rstest]
#[case(DictationPatch { api_url: Some("https://api.groq.com/openai/v1".into()), ..Default::default() })]
#[case(DictationPatch { api_key_env: Some("GROQ_API_KEY".into()), ..Default::default() })]
fn apply_api_target_from_the_agent_is_reserved_to_the_user(#[case] patch: DictationPatch) {
    let t = voice_with_stt();
    let r = t.voice.apply(dictation(patch), agent_origin("t-1", true));
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::ApiReservedToUser);
}

#[test]
fn apply_azure_settings_from_the_user_are_shown_back() {
    let t = voice_with_stt();
    let patch = DictationPatch {
        api_key_env: Some("AZURE_OPENAI_API_KEY".into()),
        api_version: Some("2025-03-01-preview".into()),
        api_headers: Some("X-Tenant:acme".into()),
        ..Default::default()
    };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    let api = t.voice.view().dictation.unwrap().api;
    assert_that!((api.api_version.as_str(), api.headers.as_str()))
        .is_equal_to(("2025-03-01-preview", "X-Tenant: acme"));
}

#[test]
fn apply_azure_deployment_with_a_slash_is_accepted_until_use() {
    let t = voice_with_stt();
    for patch in [
        DictationPatch { api_model: Some("team/whisper".into()), ..Default::default() },
        DictationPatch { api_version: Some("2025-03-01-preview".into()), ..Default::default() },
    ] {
        t.voice.apply(dictation(patch), Origin::User).unwrap();
    }
    assert_that!(t.voice.view().dictation.unwrap().api.model).is_equal_to("team/whisper".to_owned());
}

#[test]
fn check_switch_to_api_from_the_agent_with_a_bad_azure_deployment_is_refused() {
    let t = voice_with_stt();
    let patch = DictationPatch {
        api_model: Some("team/whisper".into()),
        api_version: Some("2025-03-01-preview".into()),
        ..Default::default()
    };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    let r = t.voice.check(
        &dictation(DictationPatch { model: Some("api".into()), ..Default::default() }),
        &agent_origin("t-1", false),
    );
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::BadApiModel);
}

#[test]
fn view_api_names_the_first_unset_variable() {
    let t = voice_with_stt();
    let patch = DictationPatch {
        api_key_env: Some("HOME".into()),
        api_headers: Some("X-Token: $OKU_T1213_UNSET".into()),
        ..Default::default()
    };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.voice.view().dictation.unwrap().api.missing).is_equal_to(Some("OKU_T1213_UNSET".to_owned()));
}

#[test]
fn apply_api_url_from_the_user_is_stored_normalized() {
    let t = voice_with_stt();
    let patch = DictationPatch { api_url: Some("HTTPS://API.groq.com/openai/v1/".into()), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.voice.view().dictation.unwrap().api.url).is_equal_to("https://api.groq.com/openai/v1".to_owned());
}

#[test]
fn apply_invalid_patch_changes_nothing() {
    let t = voice_with_stt();
    let before = t.voice.view();
    let _ = t.voice.apply(dictation(DictationPatch { silence: Some(0.2), ..Default::default() }), Origin::User);
    assert_that!(t.voice.view()).is_equal_to(before);
}

#[test]
fn apply_language_patch_configures_stt() {
    let t = voice_with_stt();
    let patch = DictationPatch { language: Some(DictationLanguage::Code("fr".into())), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.voice.view().dictation.unwrap().language).is_equal_to(Some("fr".to_owned()));
}

#[test]
fn apply_the_stored_unlisted_language_is_accepted() {
    let t = voice_with_stt();
    let stt = t.stt.as_ref().unwrap();
    let mut config = stt.status().config;
    config.language = Some("ja".into());
    // Une langue d'avant la liste fixe, écrite directement : seul son retour est testé.
    let _applied = stt.configure(config);
    let patch = DictationPatch { language: Some(DictationLanguage::Code("ja".into())), ..Default::default() };
    assert_that!(t.voice.apply(dictation(patch), Origin::User).map_err(|e| e.kind()))
        .is_ok_containing(VoiceApplied::Unchanged);
}

#[test]
fn apply_accepted_patch_bumps_the_revision() {
    let t = voice_with_stt();
    let rx = t.voice.revision();
    let before = *rx.borrow();
    t.voice.apply(dictation(DictationPatch { auto_stop: Some(false), ..Default::default() }), Origin::User).unwrap();
    assert_that!(*rx.borrow()).is_greater_than(before);
}

#[test]
fn apply_refused_patch_keeps_the_revision() {
    let t = voice_with_stt();
    let rx = t.voice.revision();
    let before = *rx.borrow();
    let _ = t.voice.apply(dictation(DictationPatch { silence: Some(0.1), ..Default::default() }), Origin::User);
    assert_that!(*rx.borrow()).is_equal_to(before);
}

#[test]
fn apply_same_value_is_unchanged() {
    let t = voice_with_stt();
    let current = t.voice.view().dictation.unwrap().model;
    let r = t.voice.apply(dictation(DictationPatch { model: Some(current), ..Default::default() }), Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_ok_containing(VoiceApplied::Unchanged);
}

#[test]
fn check_switch_to_api_from_the_agent_names_the_user_target() {
    let t = voice_with_stt();
    let checked = t
        .voice
        .check(
            &dictation(DictationPatch { model: Some("api".into()), ..Default::default() }),
            &agent_origin("t-1", false),
        )
        .unwrap();
    let target = checked.api.unwrap();
    assert_that!((target.host.as_str(), target.variables.as_slice()))
        .is_equal_to(("api.openai.com", ["OPENAI_API_KEY".to_owned()].as_slice()));
}

#[test]
fn check_switch_to_api_names_the_requested_api_model() {
    let t = voice_with_stt();
    let patch = DictationPatch { model: Some("api".into()), api_model: Some("whisper-1".into()), ..Default::default() };
    let checked = t.voice.check(&dictation(patch), &agent_origin("t-1", false)).unwrap();
    assert_that!(checked.api.map(|a| a.model)).is_equal_to(Some("whisper-1".to_owned()));
}

fn store_invalid_url(t: &T) {
    // Une adresse d'avant les règles (t-1137), écrite directement dans la config de la dictée.
    let stt = t.stt.as_ref().unwrap();
    let mut config = stt.status().config;
    config.api.url = "http://192.168.1.5/v1".into();
    let _applied = stt.configure(config);
}

#[test]
fn check_switch_to_api_from_the_agent_with_an_invalid_stored_url_is_refused() {
    let t = voice_with_stt();
    store_invalid_url(&t);
    let r = t.voice.check(
        &dictation(DictationPatch { model: Some("api".into()), ..Default::default() }),
        &agent_origin("t-1", false),
    );
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::BadUrl);
}

fn store_legacy_query(t: &T) {
    // Settings saved by 7.0.0 or earlier: their query parameters are not migrated (t-1233).
    let stt = t.stt.as_ref().unwrap();
    let mut config = stt.status().config;
    config.api.legacy_query =
        vec![oku_slides_stt::LegacyQueryParam { name: "api-version".into(), value: "2024-06-01".into() }];
    let _applied = stt.configure(config);
}

#[test]
fn check_switch_to_api_from_the_agent_with_legacy_query_is_refused_as_bad_version() {
    let t = voice_with_stt();
    store_legacy_query(&t);
    let r = t.voice.check(
        &dictation(DictationPatch { model: Some("api".into()), ..Default::default() }),
        &agent_origin("t-1", false),
    );
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::BadApiVersion);
}

#[test]
fn apply_api_field_from_the_user_drops_the_legacy_query() {
    let t = voice_with_stt();
    store_legacy_query(&t);
    let patch = DictationPatch { api_version: Some("2024-06-01".into()), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.stt.as_ref().unwrap().status().config.api.legacy_query).is_empty();
}

#[test]
fn apply_non_api_field_from_the_user_keeps_the_legacy_query() {
    let t = voice_with_stt();
    store_legacy_query(&t);
    let patch = DictationPatch { auto_stop: Some(true), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.stt.as_ref().unwrap().status().config.api.legacy_query).has_length(1);
}

#[test]
fn check_switch_to_api_from_the_agent_with_an_empty_key_variable_is_refused_as_bad_key() {
    let t = voice_with_stt();
    let stt = t.stt.as_ref().unwrap();
    let mut config = stt.status().config;
    config.api.key_env = String::new();
    let _applied = stt.configure(config);
    let r = t.voice.check(
        &dictation(DictationPatch { model: Some("api".into()), ..Default::default() }),
        &agent_origin("t-1", false),
    );
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::BadKeyEnv);
}

#[test]
fn apply_switch_to_api_from_the_user_with_an_invalid_stored_url_is_accepted() {
    // L'utilisateur doit pouvoir passer à l'API pour voir les champs et corriger l'adresse.
    let t = voice_with_stt();
    store_invalid_url(&t);
    let r = t.voice.apply(dictation(DictationPatch { model: Some("api".into()), ..Default::default() }), Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_ok_containing(VoiceApplied::Now);
}

#[test]
fn check_switch_to_an_uninstalled_model_names_its_download() {
    let t = voice_with_stt();
    let checked = t
        .voice
        .check(&dictation(DictationPatch { model: Some("large".into()), ..Default::default() }), &Origin::User)
        .unwrap();
    assert_that!(checked.download.map(|(m, _)| m.id)).is_equal_to(Some("large".to_owned()));
}

#[test]
fn check_download_of_an_uninstalled_model_names_it() {
    let t = voice_with_stt();
    let checked = t.voice.check(&VoiceEvent::DownloadDictation { model: "large".into() }, &Origin::User).unwrap();
    assert_that!(checked.download.map(|(m, _)| m.id)).is_equal_to(Some("large".to_owned()));
}

#[test]
fn apply_unconfirmed_agent_switch_to_api_is_changed() {
    let t = voice_with_stt();
    let r = t.voice.apply(
        dictation(DictationPatch { model: Some("api".into()), ..Default::default() }),
        agent_origin("t-1", false),
    );
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::Changed);
}

#[test]
fn view_api_key_found_for_a_set_variable() {
    let t = voice_with_stt();
    let patch = DictationPatch { model: Some("api".into()), api_key_env: Some("HOME".into()), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.voice.view().dictation.unwrap().api.key_found).is_true();
}

#[rstest]
#[case(VoiceEvent::DownloadDictation { model: "api".into() }, VoiceErrorKind::NotDownloadable)]
#[case(VoiceEvent::DownloadDictation { model: "nope".into() }, VoiceErrorKind::UnknownModel)]
#[case(VoiceEvent::CancelDictationDownload, VoiceErrorKind::NothingToCancel)]
#[case(VoiceEvent::CancelAgentModelDownload, VoiceErrorKind::NothingToCancel)]
#[case(VoiceEvent::SelectAgent { id: "nope".into() }, VoiceErrorKind::UnknownAgent)]
#[case::model_before_any_session(VoiceEvent::SelectAgentModel { id: "deep".into() }, VoiceErrorKind::UnknownAgentModel)]
fn check_refuses_before_any_change(#[case] event: VoiceEvent, #[case] kind: VoiceErrorKind) {
    let t = voice_with_stt();
    assert_that!(t.voice.check(&event, &Origin::User).map_err(|e| e.kind())).is_err_containing(kind);
}

#[test]
fn check_agent_when_locked_is_refused() {
    let t = voice_locked();
    let r = t.voice.check(&VoiceEvent::SelectAgent { id: "bis".into() }, &Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::AgentLocked);
}

#[rstest]
#[case(oku_slides_stt::State::Idle, false)]
#[case(oku_slides_stt::State::Recording, false)]
#[case(oku_slides_stt::State::Transcribing, false)]
#[case(oku_slides_stt::State::Downloading, true)]
fn cancel_dictation_allowed_only_while_downloading(#[case] state: oku_slides_stt::State, #[case] allowed: bool) {
    assert_that!(cancel_dictation_allowed(state)).is_equal_to(allowed);
}

#[test]
fn view_without_stt_has_no_dictation() {
    let t = voice_without_stt();
    assert_that!(t.voice.view().dictation).is_none();
}

#[test]
fn dictation_event_without_stt_is_no_dictation() {
    let t = voice_without_stt();
    let r = t.voice.check(&VoiceEvent::CancelDictationDownload, &Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::NoDictation);
}
