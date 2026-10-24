use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;

fn ctx(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

fn patch(p: DictationPatch) -> Option<VoiceEvent> {
    Some(VoiceEvent::Dictation(p))
}

#[rstest]
#[case("model", json!({"id": "large"}), patch(DictationPatch { model: Some("large".into()), ..Default::default() }))]
#[case("language", json!({"code": ""}), patch(DictationPatch { language: Some(DictationLanguage::Auto), ..Default::default() }))]
#[case("language", json!({"code": "fr"}), patch(DictationPatch { language: Some(DictationLanguage::Code("fr".into())), ..Default::default() }))]
#[case("silence", json!({"value": "off"}), patch(DictationPatch { auto_stop: Some(false), ..Default::default() }))]
#[case("silence", json!({"value": "3"}), patch(DictationPatch { auto_stop: Some(true), silence: Some(3.0), ..Default::default() }))]
#[case("api", json!({"field": "url", "value": "http://localhost:8000/v1"}), patch(DictationPatch { api_url: Some("http://localhost:8000/v1".into()), ..Default::default() }))]
#[case("api", json!({"field": "model", "value": "whisper-1"}), patch(DictationPatch { api_model: Some("whisper-1".into()), ..Default::default() }))]
#[case("api", json!({"field": "key_env", "value": "GROQ_API_KEY"}), patch(DictationPatch { api_key_env: Some("GROQ_API_KEY".into()), ..Default::default() }))]
#[case("download", json!({"model": "base"}), Some(VoiceEvent::DownloadDictation { model: "base".into() }))]
#[case("cancel_download", json!({}), Some(VoiceEvent::CancelDictationDownload))]
#[case("select", json!({"id": "bis"}), Some(VoiceEvent::SelectAgent { id: "bis".into() }))]
#[case("download_agent_model", json!({}), Some(VoiceEvent::DownloadAgentModel))]
#[case("cancel_agent_model_download", json!({}), Some(VoiceEvent::CancelAgentModelDownload))]
fn voice_event_known_action_parses(#[case] name: &str, #[case] context: Value, #[case] expected: Option<VoiceEvent>) {
    assert_that!(voice_event(name, &ctx(context))).is_equal_to(expected);
}

#[rstest]
#[case("model", json!({}))]
#[case("language", json!({}))]
#[case("download", json!({}))]
#[case("select", json!({}))]
#[case("silence", json!({"value": "0"}))]
#[case("silence", json!({"value": "-1"}))]
#[case("silence", json!({"value": "inf"}))]
#[case("silence", json!({"value": "NaN"}))]
#[case("silence", json!({"value": "abc"}))]
#[case("api", json!({"field": "key", "value": "x"}))]
#[case("api", json!({"field": "url"}))]
#[case("dismiss", json!({}))]
#[case("nope", json!({}))]
fn voice_event_malformed_action_is_none(#[case] name: &str, #[case] context: Value) {
    assert_that!(voice_event(name, &ctx(context))).is_none();
}

#[rstest]
#[case(DictationPatch { model: Some("api".into()), ..Default::default() }, false)]
#[case(DictationPatch { api_model: Some("m".into()), ..Default::default() }, false)]
#[case(DictationPatch { api_url: Some("https://a.example".into()), ..Default::default() }, true)]
#[case(DictationPatch { api_key_env: Some("K".into()), ..Default::default() }, true)]
fn dictation_patch_touches_api_target_with_url_or_variable(#[case] p: DictationPatch, #[case] touches: bool) {
    assert_that!(p.touches_api_target()).is_equal_to(touches);
}

#[rstest]
#[case(DictationPatch::default(), true)]
#[case(DictationPatch { auto_stop: Some(true), ..Default::default() }, false)]
fn dictation_patch_is_empty_only_without_fields(#[case] p: DictationPatch, #[case] empty: bool) {
    assert_that!(p.is_empty()).is_equal_to(empty);
}
