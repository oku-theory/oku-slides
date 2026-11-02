//! Apple's real model: manual tests (`--ignored`), on macOS 26+ with Apple Intelligence on.
//!
//! `OKU_SLIDES_APPLE_DUMP=<dir>` also writes each Act completion's schema, instructions and
//! prompt there, to count their tokens.

use std::path::PathBuf;

use oku_slides_platform::config::{LocalAgentSettings, Settings};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use tokio_util::sync::CancellationToken;

use super::AppleEngine;
use crate::compact::{ToolSpec, exposed};
use crate::engine::{Completion, Engine, Message, Reply, Step};
use crate::families::{Family, offered_families, parse_route};
use crate::prompt::INSTRUCTIONS;
use crate::test_tools::deck_tools;

fn settings_with(edit: impl FnOnce(&mut LocalAgentSettings)) -> LocalAgentSettings {
    let mut settings = Settings::defaults().unwrap().local_agent;
    edit(&mut settings);
    settings
}

#[test]
fn new_default_settings_is_ok() {
    assert_that!(AppleEngine::new(&settings_with(|_| {}))).is_ok();
}

#[rstest]
#[case::negative(-0.1)]
#[case::nan(f32::NAN)]
#[case::infinite(f32::INFINITY)]
fn new_bad_temperature_names_the_setting(#[case] temperature: f32) {
    let err = AppleEngine::new(&settings_with(|s| s.temperature = temperature)).unwrap_err();
    assert_that!(err.to_string()).contains("temperature");
}

fn dump(name: &str, text: &str) {
    if let Some(dir) = std::env::var_os("OKU_SLIDES_APPLE_DUMP") {
        std::fs::write(PathBuf::from(dir).join(name), text).unwrap();
    }
}

const CONTEXT: &str = "<oku-slides-context>\ninterface language: English (en)\ncurrent slide: 1\n</oku-slides-context>";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "manual: Apple's real model, macOS 26+ with Apple Intelligence on"]
async fn apple_real_model_routes_and_acts_on_deck_tools() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let (all, _server) = deck_tools(dir.path(), &cancel).await;
    let engine = AppleEngine::new(&Settings::defaults().unwrap().local_agent).unwrap();
    let request = format!("{CONTEXT}\n\nRename the deck to Roadmap 2027");
    let messages = vec![Message::User(request)];

    let offered = offered_families(all.iter().map(|s| s.name.as_str()));
    let families: Vec<String> = offered.iter().map(|f| format!("{}: {}", f.key(), f.summary())).collect();
    let route = Completion {
        step: Step::Route,
        system: format!("{INSTRUCTIONS}\n{}", families.join("\n")),
        messages: messages.clone(),
        tools: vec![],
        offered: offered.clone(),
    };
    let started = std::time::Instant::now();
    let reply = engine.complete(&route, &cancel).await.unwrap();
    eprintln!("route in {:?}: {reply:?}", started.elapsed());
    let Reply::Text(text) = reply else { panic!("Route gives text") };
    assert_that!(parse_route(&text, &offered)).is_not_empty();

    for family in [Family::Configure, Family::Edit] {
        let tools = exposed(&all, &[family]);
        let schema = crate::apple_steps::pick(&tools).to_string();
        let act = Completion {
            step: Step::Act,
            system: INSTRUCTIONS.to_owned(),
            messages: messages.clone(),
            tools,
            offered: vec![],
        };
        let rendered = crate::apple_prompt::render(&act);
        let key = family.key();
        eprintln!(
            "{key}: {} tools, schema {} bytes, instructions {} bytes, prompt {} bytes",
            act.tools.len(),
            schema.len(),
            rendered.instructions.len(),
            rendered.prompt.len()
        );
        dump(&format!("{key}-schema.json"), &schema);
        for tool in &act.tools {
            let schema = crate::apple_schema::call(tool);
            dump(&format!("tool-{}.json", tool.name), &schema.to_string());
            // The largest a narrowed call schema gets: its union cut to the three biggest branches.
            if let Some((union, tags)) = crate::apple_steps::tagged_union(&schema) {
                let mut sized: Vec<(usize, String)> = tags
                    .iter()
                    .map(|t| {
                        (
                            crate::apple_steps::narrow(&schema, &union, std::slice::from_ref(t)).to_string().len(),
                            t.clone(),
                        )
                    })
                    .collect();
                sized.sort();
                let biggest: Vec<String> = sized.iter().rev().take(3).map(|(_, t)| t.clone()).collect();
                let narrowed = crate::apple_steps::narrow(&schema, &union, &biggest);
                dump(&format!("narrowed-{}.json", tool.name), &narrowed.to_string());
                dump(&format!("kinds-{}.json", tool.name), &crate::apple_steps::kinds(&tags).to_string());
            }
        }
        dump(&format!("{key}-instructions.txt"), &rendered.instructions);
        dump(&format!("{key}-prompt.txt"), &rendered.prompt);
        let started = std::time::Instant::now();
        let reply = engine.complete(&act, &cancel).await;
        eprintln!("{key} act in {:?}: {reply:?}", started.elapsed());
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "manual: Apple's real model, macOS 26+ with Apple Intelligence on"]
async fn apple_real_model_calls_a_small_tool() {
    let engine = AppleEngine::new(&Settings::defaults().unwrap().local_agent).unwrap();
    let report = ToolSpec {
        name: "report".into(),
        description: "Report back to the user with a title and facts.".into(),
        schema: json!({"type": "object", "properties": {"title": {"type": "string"}, "facts": {"type": "array", "items": {"type": "string"}, "maxItems": 4}}, "required": ["title"]}),
    };
    let act = Completion {
        step: Step::Act,
        system: "You are a slide editing assistant. Call the report tool to answer.".into(),
        messages: vec![Message::User("Say hello to the user.".into())],
        tools: vec![report],
        offered: vec![],
    };
    let reply = engine.complete(&act, &CancellationToken::new()).await.unwrap();
    eprintln!("act: {reply:?}");
    // A call schema offers no text: anything but a call of the one tool fails the test.
    match reply {
        Reply::Call { name, args } => {
            assert_that!(name.as_str()).is_equal_to("report");
            assert_that!(args["title"].is_string()).is_true();
        }
        Reply::Text(text) => assert_that!(text.is_empty()).is_false(),
        Reply::Invalid(reason) => panic!("invalid reply: {reason}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "manual: Apple's real model, macOS 26+ with Apple Intelligence on"]
async fn apple_real_model_cancelled_is_cancelled() {
    let engine = AppleEngine::new(&Settings::defaults().unwrap().local_agent).unwrap();
    let act = Completion {
        step: Step::Act,
        system: "You write long texts.".into(),
        messages: vec![Message::User("Write a long story about slides.".into())],
        tools: vec![ToolSpec {
            name: "write".into(),
            description: "Write a text.".into(),
            schema: json!({"type": "object", "properties": {"story": {"type": "string"}}, "required": ["story"]}),
        }],
        offered: vec![],
    };
    let cancel = CancellationToken::new();
    let late = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        late.cancel();
    });
    let err = engine.complete(&act, &cancel).await.unwrap_err();
    assert_that!(err.is_cancelled()).is_true();
}
