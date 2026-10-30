//! Le vrai modèle (2,5 Go) : test manuel, `OKU_SLIDES_GGUF_MODEL=<fichier .gguf>`.
//!
//! Seul test de la crate à initialiser llama.cpp (≈ 23 s de compilation des shaders Metal
//! au premier lancement de chaque binaire).

use std::path::{Path, PathBuf};

use oku_slides_platform::config::{LocalAgentSettings, Settings};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use tokio_util::sync::CancellationToken;

use super::{GgufEngine, panic_text, validate};
use crate::compact::ToolSpec;
use crate::engine::{Completion, Engine, Message, Reply, Step};
use crate::families::{Family, parse_route};

fn settings_with(edit: impl FnOnce(&mut LocalAgentSettings)) -> LocalAgentSettings {
    let mut settings = Settings::defaults().expect("réglages par défaut").local_agent;
    edit(&mut settings);
    settings
}

#[test]
fn validate_default_settings_is_ok() {
    assert_that!(validate(Path::new("m.gguf"), &settings_with(|_| {}))).is_ok();
}

#[rstest]
#[case::zero_context(|s: &mut LocalAgentSettings| s.context = 0, "context")]
#[case::reply_equals_context(|s: &mut LocalAgentSettings| s.max_reply_tokens = s.context, "max_reply_tokens")]
#[case::reply_above_context(|s: &mut LocalAgentSettings| s.max_reply_tokens = s.context + 1, "max_reply_tokens")]
#[case::negative_temperature(|s: &mut LocalAgentSettings| s.temperature = -0.1, "temperature")]
#[case::nan_temperature(|s: &mut LocalAgentSettings| s.temperature = f32::NAN, "temperature")]
#[case::infinite_temperature(|s: &mut LocalAgentSettings| s.temperature = f32::INFINITY, "temperature")]
fn validate_bad_setting_names_the_setting(#[case] edit: fn(&mut LocalAgentSettings), #[case] name: &str) {
    let err = validate(Path::new("m.gguf"), &settings_with(edit)).unwrap_err();
    assert_that!(err.to_string()).contains(name);
}

#[test]
fn panic_text_reads_str_and_string_payloads() {
    assert_that!(panic_text(&"boom")).is_equal_to("boom".to_owned());
    assert_that!(panic_text(&"bang".to_owned())).is_equal_to("bang".to_owned());
    assert_that!(panic_text(&42_u8)).contains("not a string");
}

fn route_prompt() -> String {
    let lines: Vec<String> = Family::ROUTABLE.iter().map(|f| format!("- {}: {}", f.key(), f.summary())).collect();
    format!("Pick one or two tool families for the request, as `key` or `key, key`:\n{}", lines.join("\n"))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "manuel : charge le vrai modèle Qwen3 4B"]
async fn gguf_real_model_routes_and_acts() {
    let model = PathBuf::from(std::env::var("OKU_SLIDES_GGUF_MODEL").expect("OKU_SLIDES_GGUF_MODEL"));
    let settings = Settings::defaults().expect("réglages par défaut").local_agent;
    let engine = GgufEngine::load(&model, &settings).expect("chargement du modèle");
    let cancel = CancellationToken::new();

    let route = Completion {
        step: Step::Route,
        system: route_prompt(),
        messages: vec![Message::User("Make the title bigger".into())],
        tools: vec![],
        offered: Family::ROUTABLE.to_vec(),
    };
    let reply = engine.complete(&route, &cancel).await.expect("routage");
    eprintln!("route: {reply:?}");
    let Reply::Text(text) = reply else { panic!("le routage rend du texte") };
    assert_that!(parse_route(&text, &Family::ROUTABLE)).is_not_empty();
    assert_that!(text.split(", ").all(|k| Family::ROUTABLE.iter().any(|f| f.key() == k))).is_true();

    let report = ToolSpec {
        name: "report".into(),
        description: "Report back to the user with a title and facts.".into(),
        schema: json!({"type": "object", "properties": {"title": {"type": "string"}}, "required": ["title"]}),
    };
    let act = Completion {
        step: Step::Act,
        system: "You are a slide editing assistant. Call the report tool to answer.".into(),
        messages: vec![Message::User("Say hello to the user.".into())],
        tools: vec![report],
        offered: vec![],
    };
    let reply = engine.complete(&act, &cancel).await.expect("action");
    eprintln!("act: {reply:?}");
    match reply {
        Reply::Call { name, .. } => assert_that!(name.as_str()).is_equal_to("report"),
        Reply::Text(text) => assert_that!(text.is_empty()).is_false(),
        Reply::Invalid(reason) => panic!("appel mal formé : {reason}"),
    }
}
