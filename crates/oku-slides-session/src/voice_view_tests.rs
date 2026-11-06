use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;

fn ctx(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

#[rstest]
#[case(
    "model",
    json!({"id": "large"}),
    Some(VoiceEvent::Dictation(DictationPatch { model: Some("large".into()), ..Default::default() }))
)]
#[case(
    "language",
    json!({"code": ""}),
    Some(VoiceEvent::Dictation(DictationPatch { language: Some(DictationLanguage::Auto), ..Default::default() }))
)]
#[case(
    "language",
    json!({"code": "fr"}),
    Some(VoiceEvent::Dictation(DictationPatch {
        language: Some(DictationLanguage::Code("fr".into())),
        ..Default::default()
    }))
)]
#[case(
    "silence",
    json!({"value": "off"}),
    Some(VoiceEvent::Dictation(DictationPatch { auto_stop: Some(false), ..Default::default() }))
)]
#[case(
    "silence",
    json!({"value": "3"}),
    Some(VoiceEvent::Dictation(DictationPatch { auto_stop: Some(true), silence: Some(3.0), ..Default::default() }))
)]
#[case(
    "api",
    json!({"field": "url", "value": "http://localhost:8000/v1"}),
    Some(VoiceEvent::Dictation(DictationPatch {
        api_url: Some("http://localhost:8000/v1".into()),
        ..Default::default()
    }))
)]
#[case(
    "api",
    json!({"field": "model", "value": "whisper-1"}),
    Some(VoiceEvent::Dictation(DictationPatch { api_model: Some("whisper-1".into()), ..Default::default() }))
)]
#[case(
    "api",
    json!({"field": "key_env", "value": "GROQ_API_KEY"}),
    Some(VoiceEvent::Dictation(DictationPatch { api_key_env: Some("GROQ_API_KEY".into()), ..Default::default() }))
)]
#[case(
    "api",
    json!({"field": "api_version", "value": "2024-10-21"}),
    Some(VoiceEvent::Dictation(DictationPatch { api_version: Some("2024-10-21".into()), ..Default::default() }))
)]
#[case(
    "api",
    json!({"field": "headers", "value": "api-key: $K"}),
    Some(VoiceEvent::Dictation(DictationPatch { api_headers: Some("api-key: $K".into()), ..Default::default() }))
)]
#[case(
    "hands_free",
    json!({"on": true}),
    Some(VoiceEvent::Dictation(DictationPatch { hands_free: Some(true), ..Default::default() }))
)]
#[case(
    "hands_free",
    json!({"on": false}),
    Some(VoiceEvent::Dictation(DictationPatch { hands_free: Some(false), ..Default::default() }))
)]
#[case("download", json!({"model": "base"}), Some(VoiceEvent::DownloadDictation { model: "base".into() }))]
#[case("cancel_download", json!({}), Some(VoiceEvent::CancelDictationDownload))]
#[case("select", json!({"id": "bis"}), Some(VoiceEvent::SelectAgent { id: "bis".into() }))]
#[case("download_agent_model", json!({}), Some(VoiceEvent::DownloadAgentModel))]
#[case("cancel_agent_model_download", json!({}), Some(VoiceEvent::CancelAgentModelDownload))]
#[case("agent_model", json!({"id": "gpt-5"}), Some(VoiceEvent::SelectAgentModel { id: "gpt-5".into() }))]
#[case("read_aloud", json!({"on": true}), Some(VoiceEvent::Speech(SpeechPatch { read_aloud: Some(true), ..Default::default() })))]
#[case("speech_voice", json!({"id": "v-fr"}), Some(VoiceEvent::Speech(SpeechPatch { voice: Some("v-fr".into()), ..Default::default() })))]
#[case("speech_rate", json!({"value": "1.25"}), Some(VoiceEvent::Speech(SpeechPatch { rate: Some(1.25), ..Default::default() })))]
#[case(
    "speech_in_presentation",
    json!({"on": false}),
    Some(VoiceEvent::Speech(SpeechPatch { in_presentation: Some(false), ..Default::default() }))
)]
#[case(
    "speech_barge_in",
    json!({"on": false}),
    Some(VoiceEvent::Speech(SpeechPatch { barge_in: Some(false), ..Default::default() }))
)]
fn voice_event_known_action_parses(#[case] name: &str, #[case] context: Value, #[case] expected: Option<VoiceEvent>) {
    assert_that!(voice_event(name, &ctx(context))).is_equal_to(expected);
}

#[rstest]
#[case("model", json!({}))]
#[case("language", json!({}))]
#[case("download", json!({}))]
#[case("select", json!({}))]
#[case("agent_model", json!({}))]
#[case("agent_model", json!({"id": ""}))]
#[case("silence", json!({"value": "0"}))]
#[case("silence", json!({"value": "-1"}))]
#[case("silence", json!({"value": "inf"}))]
#[case("silence", json!({"value": "NaN"}))]
#[case("silence", json!({"value": "abc"}))]
#[case("api", json!({"field": "key", "value": "x"}))]
#[case("api", json!({"field": "api_version"}))]
#[case("api", json!({"field": "url"}))]
#[case("hands_free", json!({}))]
#[case("hands_free", json!({"on": "true"}))]
#[case("read_aloud", json!({"on": "yes"}))]
#[case("speech_rate", json!({"value": "inf"}))]
#[case("speech_voice", json!({}))]
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
#[case(DictationPatch { api_version: Some("2024-10-21".into()), ..Default::default() }, true)]
#[case(DictationPatch { api_headers: Some("api-key: $K".into()), ..Default::default() }, true)]
fn dictation_patch_touches_api_target_with_url_or_variable(#[case] p: DictationPatch, #[case] touches: bool) {
    assert_that!(p.touches_api_target()).is_equal_to(touches);
}

#[rstest]
#[case(DictationPatch::default(), true)]
#[case(DictationPatch { auto_stop: Some(true), ..Default::default() }, false)]
fn dictation_patch_is_empty_only_without_fields(#[case] p: DictationPatch, #[case] empty: bool) {
    assert_that!(p.is_empty()).is_equal_to(empty);
}

#[rstest]
#[case(None, true, Some(true))]
#[case(Some(false), true, Some(false))]
#[case(Some(true), true, Some(true))]
#[case(None, false, None)]
#[case(Some(false), false, Some(false))]
fn read_aloud_on_at_first_hands_free_only(
    #[case] before: Option<bool>,
    #[case] hands_free_on: bool,
    #[case] after: Option<bool>,
) {
    assert_that!(read_aloud_after_hands_free(before, hands_free_on)).is_equal_to(after);
}
