//! Tests du tour, sur le moteur factice (builds de débogage seulement).
#![cfg(debug_assertions)]

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use oku_slides_i18n::Localizer;
use oku_slides_session::DocumentKind;
use serde_json::{Value, json};
use speculoos::prelude::*;
use tokio_util::sync::CancellationToken;

use super::{FailKind, MAX_CALLS, ToolPort, TurnEnd, Updates, run_turn};
use crate::compact::ToolSpec;
use crate::engine::{Completion, Engine, EngineError, FailedSnafu, Message, Reply, Step};
use crate::fake::FakeEngine;
use crate::families::{Family, families_of};
use crate::prompt::Prompt;
use crate::tool_error::ToolError;

/// Port d'outils factice : `apply`, `save`, `report` ; `failing` rend une erreur.
#[derive(Default)]
struct FakeTools {
    failing: Option<String>,
    /// `list` échoue côté transport.
    list_fails: bool,
    /// Cet outil échoue côté transport, pas comme un appel à réparer.
    transport_on: Option<String>,
    /// Annule ce jeton pendant le deuxième appel de cet outil.
    cancel_on_fail: Option<(String, CancellationToken)>,
    /// The listed tools, in place of the default three.
    listed: Option<Vec<String>>,
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
    async fn list(&self) -> Result<Vec<ToolSpec>, ToolError> {
        if self.list_fails {
            return Err(ToolError::transport_for_test());
        }
        let names = self.listed.clone().unwrap_or_else(|| ["apply", "save", "report"].map(str::to_owned).to_vec());
        Ok(names
            .into_iter()
            .map(|n| ToolSpec { description: format!("{n} tool."), name: n, schema: json!({}) })
            .collect())
    }

    async fn call(&self, name: &str, args: &Value) -> Result<String, ToolError> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push((name.to_owned(), args.clone()));
        // Comme le vrai serveur : `report` exige 1 à 4 faits.
        if name == "report" && args["facts"].as_array().is_none_or(Vec::is_empty) {
            return Err(ToolError::rejected("report: facts required"));
        }
        // Le jeton est annulé pendant le deuxième échec, l'appel étant en vol.
        if let Some((n, token)) = &self.cancel_on_fail
            && n == name
            && self.calls.lock().unwrap_or_else(PoisonError::into_inner).iter().filter(|(c, _)| c == name).count() == 2
        {
            token.cancel();
        }
        if self.transport_on.as_deref() == Some(name) {
            return Err(ToolError::transport_for_test());
        }
        if self.failing.as_deref() == Some(name) {
            Err(ToolError::rejected(format!("{name} failed")))
        } else {
            Ok("ok".to_owned())
        }
    }
}

struct NoUpdates;

impl Updates for NoUpdates {
    fn tool_started(&self, _id: &str, _name: &str, _args: &Value) {}
    fn tool_finished(&self, _id: &str, _ok: bool) {}
    fn note(&self, _text: &str) {}
}

/// Garde les textes intermédiaires du modèle.
#[derive(Default)]
struct RecordingUpdates {
    notes: Mutex<Vec<String>>,
}

impl RecordingUpdates {
    fn notes(&self) -> Vec<String> {
        self.notes.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl Updates for RecordingUpdates {
    fn tool_started(&self, _id: &str, _name: &str, _args: &Value) {}
    fn tool_finished(&self, _id: &str, _ok: bool) {}
    fn note(&self, text: &str) {
        self.notes.lock().unwrap_or_else(PoisonError::into_inner).push(text.to_owned());
    }
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

fn engine_with_log(script: &Value, log: Option<PathBuf>) -> Recorder {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("script.json");
    std::fs::write(&path, script.to_string()).unwrap();
    let inner = FakeEngine::from_file(&path, log).unwrap();
    Recorder { inner, seen: Mutex::new(vec![]), _dir: dir }
}

fn engine(script: &Value) -> Recorder {
    engine_with_log(script, None)
}

fn prompt(request: &str) -> Prompt {
    Prompt { request: request.to_owned(), context: None, locale: Some("de".to_owned()) }
}

#[tokio::test]
async fn run_turn_routes_calls_and_ends_on_report() {
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[
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
        &json!({"turns":[{"route":"edit","replies":[{"call":"report","args":{"title":"ok","facts":["f"]}}]}]}),
        Some(log.clone()),
    );
    run_turn(&engine, &FakeTools::default(), &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    let lines: Vec<Value> =
        std::fs::read_to_string(&log).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let act = lines.iter().find(|l| l["step"] == "act").unwrap();
    assert_that!(act["tools"]).is_equal_to(json!(["apply", "report"]));
}

fn act_tools_of(log: &std::path::Path) -> Vec<Value> {
    let lines: Vec<Value> =
        std::fs::read_to_string(log).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    lines.iter().filter(|l| l["step"] == "act").map(|l| l["tools"].clone()).collect()
}

#[tokio::test]
async fn run_turn_theme_tab_list_exposes_no_files_tools() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("engine.log");
    let engine = engine_with_log(
        &json!({"turns":[{"route":"files, edit","replies":[{"call":"report","args":{"title":"ok","facts":["f"]}}]}]}),
        Some(log.clone()),
    );
    let tools = FakeTools { listed: Some(oku_slides_mcp::tool_names_for(DocumentKind::Theme)), ..FakeTools::default() };
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
    let exposed: Vec<String> =
        act_tools_of(&log)[0].as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_owned()).collect();
    assert_that!(exposed.iter().any(|t| t == "apply")).is_true();
    let files: Vec<&String> = exposed.iter().filter(|t| families_of(t) == [Family::Files]).collect();
    assert_that!(files).is_empty();
}

#[tokio::test]
async fn run_turn_ends_on_no_route_when_the_reply_names_no_offered_family() {
    let engine = engine(&json!({"turns":[{"route":"present","replies":[]},{"route":"present","replies":[]}]}));
    let tools = FakeTools { listed: Some(oku_slides_mcp::tool_names_for(DocumentKind::Pack)), ..FakeTools::default() };
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::NoRoute));
}

#[tokio::test]
async fn run_turn_with_one_offered_family_skips_the_route_step() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("engine.log");
    let engine = engine_with_log(
        &json!({"turns":[{"route":"","replies":[{"call":"report","args":{"title":"ok","facts":["f"]}}]}]}),
        Some(log.clone()),
    );
    let tools = FakeTools { listed: Some(vec!["report".to_owned()]), ..FakeTools::default() };
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
    let text = std::fs::read_to_string(&log).unwrap();
    assert_that!(text.contains("\"step\":\"route\"")).is_false();
}

#[tokio::test]
async fn run_turn_unknown_tool_is_repaired_once() {
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[
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
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[
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
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[
        {"call":"apply","args":{}},
        {"call":"apply","args":{}}]}]}));
    let tools = FakeTools::failing("apply");
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::InvalidCall));
}

#[tokio::test]
async fn run_turn_stops_after_max_calls() {
    let replies: Vec<Value> = (0..9).map(|_| json!({"call":"apply","args":{}})).collect();
    let engine = engine(&json!({"turns":[{"route":"edit","replies":replies}]}));
    let tools = FakeTools::default();
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::ToolLimit));
    assert_that!(tools.called().iter().filter(|n| *n == "apply").count()).is_equal_to(MAX_CALLS);
}

#[tokio::test]
async fn run_turn_empty_route_fails_after_retry() {
    let engine = engine(&json!({"turns":[{"route":"nothing"},{"route":"nothing"}]}));
    let tools = FakeTools::default();
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::NoRoute));
    assert_that!(tools.called()).is_equal_to(vec!["report".to_owned()]);
}

#[tokio::test]
async fn run_turn_cancel_ends_without_report() {
    let engine = engine(&json!({"turns":[{"route":"edit","hang":true}]}));
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
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[
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
        engine(&json!({"turns":[{"route":"edit","replies":[{"call":"report","args":{"title":"ok","facts":["f"]}}]}]}));
    let summaries = vec!["avant → fait".to_owned()];
    run_turn(&engine, &FakeTools::default(), &NoUpdates, &prompt("x"), &summaries, &CancellationToken::new()).await;
    let route = engine.seen.lock().unwrap()[0].clone();
    assert_that!(route.messages).is_equal_to(vec![Message::User("avant → fait".into()), Message::User("x".into())]);
    assert_that!(route.system.as_str()).contains("edit: ");
}

#[tokio::test]
async fn run_turn_tools_list_failure_fails_with_engine() {
    let tools = FakeTools { list_fails: true, ..FakeTools::default() };
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[]}]}));
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::Engine));
}

#[tokio::test]
async fn run_turn_transport_error_fails_with_engine_not_repair() {
    let tools = FakeTools { transport_on: Some("apply".into()), ..FakeTools::default() };
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[{"call":"apply","args":{}}]}]}));
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::Engine));
}

#[tokio::test]
async fn run_turn_success_between_failures_resets_repairs() {
    // apply échoue, save réussit, apply échoue encore, puis report : jamais deux échecs de suite.
    let tools = FakeTools::failing("apply");
    let replies = json!([
        {"call": "apply"}, {"call": "save"}, {"call": "apply"},
        {"call": "report", "args": {"title": "ok", "facts": ["f"]}}
    ]);
    let engine = engine(&json!({"turns":[{"route":"edit, files","replies":replies}]}));
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
}

#[tokio::test]
async fn run_turn_text_reply_becomes_a_note() {
    let updates = RecordingUpdates::default();
    let replies = json!([{"text": "Je regarde."}, {"call": "report", "args": {"title": "ok", "facts": ["f"]}}]);
    let engine = engine(&json!({"turns":[{"route":"edit","replies":replies}]}));
    run_turn(&engine, &FakeTools::default(), &updates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(updates.notes()).is_equal_to(vec!["Je regarde.".to_owned()]);
}

/// Moteur qui rejoue des réponses prévues, dont `Invalid` que le factice ne sait pas produire.
#[derive(Debug)]
struct Scripted {
    replies: Mutex<std::collections::VecDeque<Reply>>,
    seen: Mutex<Vec<Completion>>,
}

#[async_trait]
impl Engine for Scripted {
    async fn complete(&self, c: &Completion, _cancel: &CancellationToken) -> Result<Reply, EngineError> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner).push(c.clone());
        let next = self.replies.lock().unwrap_or_else(PoisonError::into_inner).pop_front();
        next.ok_or_else(|| FailedSnafu { message: "script exhausted" }.build().into())
    }
}

#[tokio::test]
async fn run_turn_invalid_reply_is_repaired_once() {
    let report = Reply::Call { name: "report".into(), args: json!({"title": "ok", "facts": ["f"]}) };
    let engine = Scripted {
        replies: Mutex::new([Reply::Text("edit".into()), Reply::Invalid("bad".into()), report].into()),
        seen: Mutex::new(vec![]),
    };
    let end = run_turn(&engine, &FakeTools::default(), &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
    let seen = engine.seen.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(Message::ToolResult { name, output }) = seen[2].messages.last() else { panic!("no tool result") };
    assert_that!(name.as_str()).is_equal_to("invalid");
    assert_that!(output.as_str()).starts_with("error: invalid tool call");
}

#[tokio::test]
async fn run_turn_call_is_replayed_before_its_result() {
    let engine = engine(&json!({"turns":[{"route":"edit","replies":[
        {"call":"apply","args":{"a":1}},
        {"call":"report","args":{"title":"ok","facts":["f"]}}]}]}));
    run_turn(&engine, &FakeTools::default(), &NoUpdates, &prompt("x"), &[], &CancellationToken::new()).await;
    let messages = engine.acts()[1].messages.clone();
    let n = messages.len();
    assert_that!(messages[n - 2].clone()).is_equal_to(Message::Call { name: "apply".into(), args: json!({"a": 1}) });
    assert_that!(matches!(messages[n - 1], Message::ToolResult { .. })).is_true();
}
