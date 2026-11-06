use std::sync::Arc;

use oku_slides_mcp::{Applied, SettingsPort};
use oku_slides_session::{Platform, Settings};
use serde_json::json;
use speculoos::prelude::*;

use super::AppSettings;

/// Une plateforme sur un dossier de données neuf (t-273), à garder vivant
/// autant qu'elle.
fn platform() -> (Platform, tempfile::TempDir) {
    let mut settings = Settings::defaults().unwrap();
    let dir = tempfile::Builder::new().prefix("oku-slides-settings-port-").tempdir().unwrap();
    settings.paths.data = Some(dir.path().to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    (Platform::with_settings(settings).unwrap(), dir)
}

#[test]
fn read_exposes_history_limit_and_vad_keys() {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    let v = port.read();
    assert_that!(v.get("history.limit").cloned()).is_equal_to(Some(json!(500)));
    assert_that!(v.contains_key("stt.vad.pause")).is_true();
}

#[test]
fn write_config_key_goes_to_config_toml_for_next_launch() {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write("history.limit", &json!(42))).is_equal_to(Ok(Applied::NextLaunch));
    let text = std::fs::read_to_string(p.settings.config_file()).unwrap();
    assert_that!(text.contains("limit = 42")).is_true();
}

#[test]
fn write_agent_selected_is_refused_and_config_untouched() {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write("agent.selected", &json!("x")))
        .is_equal_to(Err("agent settings are reserved to the user".to_owned()));
    assert_that!(p.settings.config_file().exists()).is_false();
}

#[test]
fn write_agent_presets_key_is_refused_and_config_untouched() {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write("agent.presets.x.command", &json!("x")))
        .is_equal_to(Err("agent settings are reserved to the user".to_owned()));
    assert_that!(p.settings.config_file().exists()).is_false();
}

#[test]
fn write_agent_root_key_is_refused_and_config_untouched() {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write("agent", &json!("x")))
        .is_equal_to(Err("agent settings are reserved to the user".to_owned()));
    assert_that!(p.settings.config_file().exists()).is_false();
}

#[test]
fn write_dictation_key_is_refused() {
    let (p, _dir) = platform();
    let port = Arc::new(AppSettings::new(&p, None));
    assert_that!(port.write("dictation.model", &json!("base")))
        .is_equal_to(Err("dictation is set through set_dictation".to_owned()));
}

#[test]
fn write_dictation_key_writes_nothing() {
    let (p, _dir) = platform();
    let port = Arc::new(AppSettings::new(&p, None));
    let _ = port.write("dictation.model", &json!("base"));
    assert_that!(p.settings.config_file().exists()).is_false();
}

#[test]
fn read_with_dictation_hides_its_config_version() {
    let (mut settings, dir) = (Settings::defaults().unwrap(), tempfile::tempdir().unwrap());
    settings.paths.data = Some(dir.path().to_owned());
    settings.paths.decks = Some(dir.path().join("decks"));
    settings.paths.models = Some(dir.path().join("models"));
    settings.stt.apple = false; // pas de requête au système
    let p = Platform::with_settings(settings).unwrap();
    let v = AppSettings::new(&p, Some(oku_slides_stt::Stt::new(&p))).read();
    assert_that!((v.contains_key("dictation.model"), v.contains_key("dictation.version"))).is_equal_to((true, false));
}

fn speech_platform() -> (Platform, tempfile::TempDir) {
    let (mut settings, dir) = (Settings::defaults().unwrap(), tempfile::tempdir().unwrap());
    settings.paths.data = Some(dir.path().to_owned());
    settings.paths.decks = Some(dir.path().join("decks"));
    settings.paths.models = Some(dir.path().join("models"));
    settings.stt.apple = false;
    settings.i18n.locale = Some("fr".into());
    (Platform::with_settings(settings).unwrap(), dir)
}

#[rstest::rstest]
#[case::ready(true, "ready", None)]
#[case::unavailable(false, "unavailable", Some("stt-voice-no-engine"))]
fn settings_reports_speech_status(#[case] voice: bool, #[case] status: &str, #[case] reason: Option<&str>) {
    let (p, _dir) = speech_platform();
    let stt = oku_slides_stt::Stt::new(&p);
    let engine: Arc<dyn oku_slides_stt::SpeechEngine> = Arc::new(oku_slides_stt::hand_voice::HandEngine::default());
    let speaker = voice.then(|| oku_slides_stt::Speaker::new(engine, Some(stt.clone())));
    let v = AppSettings::new(&p, Some(stt)).with_speaker(speaker).read();
    assert_that!(v.get("speech.status")).is_equal_to(Some(&json!(status)));
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(v.get("speech.reason").cloned()).is_equal_to(reason.map(|k| json!(fr.tr(k))));
    assert_that!(v.get("speech.read_aloud")).is_equal_to(Some(&json!(false)));
    assert_that!(v.contains_key("dictation.read_aloud") || v.contains_key("dictation.speech")).is_false();
}

#[test]
fn settings_reports_barge_in_and_why_not() {
    let (p, _dir) = speech_platform();
    let v = AppSettings::new(&p, Some(oku_slides_stt::Stt::new(&p))).read();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let why = (!cfg!(target_os = "macos")).then(|| json!(fr.tr("speech-barge-in-unavailable-os")));
    assert_that!((v.get("speech.barge_in").cloned(), v.get("speech.barge_in_unavailable").cloned()))
        .is_equal_to((Some(json!(true)), why));
}

#[test]
fn write_speech_key_is_refused() {
    let (p, _dir) = platform();
    assert_that!(AppSettings::new(&p, None).write("speech.rate", &json!(2)))
        .is_equal_to(Err("the read-aloud is set through set_speech".to_owned()));
}
