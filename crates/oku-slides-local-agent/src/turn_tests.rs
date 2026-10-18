//! Tests du tour, sur le moteur factice (builds de débogage seulement).
#![cfg(debug_assertions)]

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};
use speculoos::prelude::*;
use tokio_util::sync::CancellationToken;

use super::{FailKind, MAX_CALLS, ToolPort, TurnEnd, Updates, run_turn};
use crate::compact::ToolSpec;
use crate::engine::{Completion, Engine, EngineError, Message, Reply, Step};
use crate::fake::FakeEngine;
use crate::prompt::Prompt;

/// Port d'outils factice : `apply`, `save`, `report` ; `failing` rend une erreur.
#[derive(Default)]
struct FakeTools {
    failing: Option<String>,
    /// Annule ce jeton pendant le deuxième appel de cet outil.
    cancel_on_fail: Option<(String, CancellationToken)>,
    calls: Mutex<Vec<(String, Value)>>,
}

impl FakeTools {
    fn failing(name: &str) -> Self {
        Self { failing: Some(name.to_owned()), ..Self::default() }
    }

    fn cancelling(name: &str, token: CancellationToken) -> Self {
        Self { cancel_on_fail: Some((name.to_owned(), token)), ..Self::failing(name) }
    }

    fn called(&self) -> Vec<String> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).iter().map(|(n, _)| n.clone()).collect()
    }

    fn last_args(&self) -> Value {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).last().map(|(_, a)| a.clone()).unwrap()
    }
}

#[async_trait]
impl ToolPort for FakeTools {
    async fn list(&self) -> Result<Vec<ToolSpec>, String> {
        Ok(["apply", "save", "report"]
            .iter()
            .map(|n| ToolSpec { name: (*n).to_owned(), description: format!("{n} tool."), schema: json!({}) })
            .collect())
    }

    async fn call(&self, name: &str, args: &Value) -> Result<String, String> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push((name.to_owned(), args.clone()));
        // Comme le vrai serveur : `report` exige 1 à 4 faits.
        if name == "report" && args["facts"].as_array().is_none_or(Vec::is_empty) {
            return Err("report: facts required".to_owned());
        }
        // Le jeton est annulé pendant le deuxième échec, l'appel étant en vol.
        if let Some((n, token)) = &self.cancel_on_fail
            && n == name
            && self.calls.lock().unwrap_or_else(PoisonError::into_inner).iter().filter(|(c, _)| c == name).count() == 2
        {
            token.cancel();
        }
        if self.failing.as_deref() == Some(name) { Err(format!("{name} failed")) } else { Ok("ok".to_owned()) }
    }
}

struct NoUpdates;

impl Updates for NoUpdates {
    fn tool_started(&self, _id: &str, _name: &str, _args: &Value) {}
    fn tool_finished(&self, _id: &str, _ok: bool) {}
    fn note(&self, _text: &str) {}
}

/// Moteur factice qui garde aussi chaque complétion reçue.
#[derive(Debug)]
struct Recorder {
    inner: FakeEngine,
    seen: Mutex<Vec<Completion>>,
    _dir: tempfile::TempDir,
}

impl Recorder {
    fn acts(&self) -> Vec<Completion> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|c| c.step == Step::Act)
            .cloned()
            .collect()
    }
}

#[async_trait]
impl Engine for Recorder {
    async fn complete(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner).push(c.clone());
        self.inner.complete(c, cancel).await
    }
}

fn engine_with_log(script: Value, log: Option<PathBuf>) -> Recorder {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("script.json");
    std::fs::write(&path, script.to_string()).unwrap();
    let inner = FakeEngine::from_file(&path, log).unwrap();
    Recorder { inner, seen: Mutex::new(vec![]), _dir: dir }
}

fn engine(script: Value) -> Recorder {
    engine_with_log(script, None)
}

fn prompt(request: &str) -> Prompt {
    Prompt { request: request.to_owned(), context: None, locale: Some("de".to_owned()) }
}

#[tokio::test]
async fn run_turn_routes_calls_and_ends_on_report() {
    let engine = engine(json!({"turns":[{"route":"edit","replies":[
        {"call":"apply","args":{"ops":[]}},
        {"call":"report","args":{"title":"Titre en rouge","facts":["f"]}}]}]}));
    let tools = FakeTools::default();
    let end =
        run_turn(&engine, &tools, &NoUpdates, &prompt("mets le titre en rouge"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
    assert_that!(tools.called()).is_equal_to(vec!["apply".to_owned(), "report".to_owned()]);
}

#[tokio::test]
async fn run_turn_act_exposes_only_routed_families() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("engine.log");
    let engine = engine_with_log(
        json!({"turns":[{"route":"edit","replies":[{"call":"report","args":{"title":"ok","facts":["f"]}}]}]}),
        Some(log.clone()),
    );
    run_turn(&engine, &FakeTools::default(), &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    let lines: Vec<Value> =
        std::fs::read_to_string(&log).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let act = lines.iter().find(|l| l["step"] == "act").unwrap();
    assert_that!(act["tools"]).is_equal_to(json!(["apply", "report"]));
}

#[tokio::test]
async fn run_turn_unknown_tool_is_repaired_once() {
    let engine = engine(json!({"turns":[{"route":"edit","replies":[
        {"call":"nope","args":{}},
        {"call":"report","args":{"title":"ok","facts":["f"]}}]}]}));
    let end = run_turn(&engine, &FakeTools::default(), &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
    let second = engine.acts()[1].messages.last().cloned().unwrap();
    let Message::ToolResult { name, output } = second else { panic!("not a tool result: {second:?}") };
    assert_that!(name.as_str()).is_equal_to("nope");
    assert_that!(output.as_str()).starts_with("error:");
}

#[tokio::test]
async fn run_turn_two_invalid_calls_fail_with_report() {
    let engine = engine(json!({"turns":[{"route":"edit","replies":[
        {"call":"nope","args":{}},
        {"call":"nope","args":{}}]}]}));
    let tools = FakeTools::default();
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::InvalidCall));
    assert_that!(tools.called().last().cloned()).is_equal_to(Some("report".to_owned()));
    let title = Localizer::new(Some("de")).tr("local-agent-invalid-call");
    assert_that!(tools.last_args()["title"].as_str()).is_equal_to(Some(title.as_str()));
    let retry = Localizer::new(Some("de")).tr("local-agent-retry");
    assert_that!(tools.last_args()["facts"]).is_equal_to(json!([retry]));
}

#[tokio::test]
async fn run_turn_failed_tool_call_is_repaired_like_an_unknown_tool() {
    let engine = engine(json!({"turns":[{"route":"edit","replies":[
        {"call":"apply","args":{}},
        {"call":"apply","args":{}}]}]}));
    let tools = FakeTools::failing("apply");
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::InvalidCall));
}

#[tokio::test]
async fn run_turn_stops_after_max_calls() {
    let replies: Vec<Value> = (0..9).map(|_| json!({"call":"apply","args":{}})).collect();
    let engine = engine(json!({"turns":[{"route":"edit","replies":replies}]}));
    let tools = FakeTools::default();
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::ToolLimit));
    assert_that!(tools.called().iter().filter(|n| *n == "apply").count()).is_equal_to(MAX_CALLS);
}

#[tokio::test]
async fn run_turn_empty_route_fails_after_retry() {
    let engine = engine(json!({"turns":[{"route":"nothing"},{"route":"nothing"}]}));
    let tools = FakeTools::default();
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::NoRoute));
    assert_that!(tools.called()).is_equal_to(vec!["report".to_owned()]);
}

#[tokio::test]
async fn run_turn_cancel_ends_without_report() {
    let engine = engine(json!({"turns":[{"route":"edit","hang":true}]}));
    let tools = FakeTools::default();
    let cancel = CancellationToken::new();
    let later = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        later.cancel();
    });
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &cancel).await;
    assert_that!(end).is_equal_to(TurnEnd::Cancelled);
    assert_that!(tools.called()).is_empty();
}

#[tokio::test]
async fn run_turn_cancel_during_failing_call_is_cancelled() {
    let engine = engine(json!({"turns":[{"route":"edit","replies":[
        {"call":"apply","args":{}},
        {"call":"apply","args":{}}]}]}));
    let cancel = CancellationToken::new();
    let tools = FakeTools::cancelling("apply", cancel.clone());
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &cancel).await;
    assert_that!(end).is_equal_to(TurnEnd::Cancelled);
    assert_that!(tools.called().iter().filter(|n| *n == "report").count()).is_equal_to(0);
}

#[tokio::test]
async fn run_turn_route_sees_summaries_then_request() {
    let engine =
        engine(json!({"turns":[{"route":"edit","replies":[{"call":"report","args":{"title":"ok","facts":["f"]}}]}]}));
    let summaries = vec!["avant → fait".to_owned()];
    run_turn(&engine, &FakeTools::default(), &NoUpdates, &prompt("x"), &summaries, &CancellationToken::new()).await;
    let route = engine.seen.lock().unwrap()[0].clone();
    assert_that!(route.messages).is_equal_to(vec![Message::User("avant → fait".into()), Message::User("x".into())]);
    assert_that!(route.system.as_str()).contains("edit: ");
}
