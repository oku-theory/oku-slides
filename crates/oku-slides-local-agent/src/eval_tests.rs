//! The evaluation set (`eval/requests.json`): checked always, run on a real engine by hand.
//!
//! `cargo nextest run -p oku-slides-local-agent --run-ignored only -E 'test(eval_)' --no-capture`
//! runs Route then the first Act step of each request on the engine `local_agent.engine`
//! picks, with a deck tab's tools, and prints one line per request and the accuracy.

use std::time::Instant;

use oku_slides_platform::config::Settings;
use oku_slides_session::{DocumentKind, Platform};
use serde::Deserialize;
use speculoos::prelude::*;
use tokio_util::sync::CancellationToken;

use crate::compact::exposed;
use crate::engine::{Message, Reply};
use crate::families::{Family, families_of, offered_families, parse_route};
use crate::test_tools::deck_tools;
use crate::turn::{act_completion, route_completion};

/// One request of the set, with what a good turn does first.
#[derive(Debug, Deserialize)]
struct Case {
    lang: String,
    request: String,
    /// The family keys that serve it; the route must name one.
    families: Vec<String>,
    /// The tools a good first Act step calls.
    first_tool: Vec<String>,
}

fn cases() -> Vec<Case> {
    serde_json::from_str(include_str!("../eval/requests.json")).unwrap()
}

fn family(key: &str) -> Family {
    Family::ROUTABLE.into_iter().find(|f| f.key() == key).unwrap()
}

#[test]
fn eval_set_has_thirty_requests_in_french_and_english() {
    let cases = cases();
    assert_that!(cases.len()).is_greater_than_or_equal_to(30);
    for lang in ["fr", "en"] {
        assert_that!(cases.iter().any(|c| c.lang == lang)).is_true();
    }
}

#[test]
fn eval_set_first_tools_are_deck_tools_of_their_families() {
    let deck = oku_slides_mcp::tool_names_for(DocumentKind::Deck);
    for case in cases() {
        let wanted: Vec<Family> = case.families.iter().map(|k| family(k)).collect();
        for tool in &case.first_tool {
            assert_that!(deck.contains(tool)).named(tool).is_true();
            let of = families_of(tool);
            let served = of.contains(&Family::AskReport) || of.iter().any(|f| wanted.contains(f));
            assert_that!(served).named(&format!("{tool} for {}", case.request)).is_true();
        }
    }
}

fn context(lang: &str) -> String {
    let language = if lang == "fr" { "Français (fr)" } else { "English (en)" };
    format!("<oku-slides-context>\ninterface language: {language}\ncurrent slide: 1\n</oku-slides-context>")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "manual: a real local engine (Apple Intelligence on, or the Gguf model downloaded)"]
async fn eval_requests_route_and_first_act_step() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let (all, _server) = deck_tools(dir.path(), &cancel).await;
    let data = dir.path().join("engine");
    // `local_engine` reads the disk through `Storage`: off the runtime threads.
    let (platform, engine) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap();
        settings.paths.data = Some(data.clone());
        settings.paths.decks = Some(data.join("decks"));
        let platform = Platform::with_settings(settings).unwrap();
        let engine = crate::entry::build_engine(&platform).unwrap();
        (platform, engine)
    })
    .await
    .unwrap();
    eprintln!("engine: {engine:?}");
    let offered = offered_families(all.iter().map(|s| s.name.as_str()));
    let (mut routed_ok, mut tool_ok) = (0, 0);
    let cases = cases();
    for case in &cases {
        let messages = vec![Message::User(format!("{}\n\n{}", context(&case.lang), case.request))];
        let started = Instant::now();
        let route = match engine.complete(&route_completion(&messages, &offered), &cancel).await {
            Ok(Reply::Text(text)) => parse_route(&text, &offered),
            other => {
                eprintln!("FAIL route {other:?} | {}", case.request);
                continue;
            }
        };
        let route_hit = route.iter().any(|f| case.families.iter().any(|k| k == f.key()));
        let act = act_completion(&exposed(&all, &route), &messages, &offered);
        let reply = engine.complete(&act, &cancel).await;
        let called = match &reply {
            Ok(Reply::Call { name, .. }) => name.clone(),
            Ok(Reply::Text(_)) => "<text>".to_owned(),
            Ok(Reply::Invalid(_)) => "<invalid>".to_owned(),
            Err(_) => "<error>".to_owned(),
        };
        let tool_hit = case.first_tool.contains(&called);
        routed_ok += usize::from(route_hit);
        tool_ok += usize::from(tool_hit);
        let keys: Vec<&str> = route.iter().map(|f| f.key()).collect();
        let mark = |hit: bool| if hit { "ok " } else { "BAD" };
        eprintln!(
            "{} route {:<24} {} tool {:<24} {:>5.1}s | {} {}",
            mark(route_hit),
            keys.join(","),
            mark(tool_hit),
            called,
            started.elapsed().as_secs_f32(),
            case.lang,
            case.request
        );
        if let Err(e) = &reply {
            eprintln!("    error: {}", e.to_string().lines().next().unwrap_or_default());
        }
    }
    eprintln!("route {routed_ok}/{n}, first tool {tool_ok}/{n}", n = cases.len());
    // `Storage` owns a runtime: its last copy must not drop inside the async test.
    std::mem::forget(platform);
}
