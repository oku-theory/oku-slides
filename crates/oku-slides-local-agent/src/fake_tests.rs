use std::path::PathBuf;

use serde_json::json;
use speculoos::prelude::*;
use tokio_util::sync::CancellationToken;

use super::FakeEngine;
use crate::engine::{Completion, Engine, Reply, Step};

fn completion(step: Step) -> Completion {
    Completion { step, system: String::new(), messages: vec![], tools: vec![] }
}

fn engine(script: &serde_json::Value, log: Option<PathBuf>) -> (tempfile::TempDir, FakeEngine) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("script.json");
    std::fs::write(&path, script.to_string()).unwrap();
    let e = FakeEngine::from_file(&path, log).unwrap();
    (dir, e)
}

#[tokio::test]
async fn fake_route_then_replies_in_order() {
    let (_d, e) =
        engine(&json!({"turns":[{"route":"edit","replies":[{"call":"apply","args":{"a":1}},{"text":"done"}]}]}), None);
    let t = CancellationToken::new();
    let r = e.complete(&completion(Step::Route), &t).await.unwrap();
    assert_that!(r).is_equal_to(Reply::Text("edit".into()));
    let r = e.complete(&completion(Step::Act), &t).await.unwrap();
    assert_that!(r).is_equal_to(Reply::Call { name: "apply".into(), args: json!({"a":1}) });
    let r = e.complete(&completion(Step::Act), &t).await.unwrap();
    assert_that!(r).is_equal_to(Reply::Text("done".into()));
}

#[tokio::test]
async fn fake_exhausted_script_is_an_error() {
    let (_d, e) = engine(&json!({"turns":[{"route":"edit","replies":[]}]}), None);
    let t = CancellationToken::new();
    e.complete(&completion(Step::Route), &t).await.unwrap();
    assert_that!(e.complete(&completion(Step::Act), &t).await.unwrap_err().is_exhausted()).is_true();
    assert_that!(e.complete(&completion(Step::Route), &t).await.unwrap_err().is_exhausted()).is_true();
}

#[tokio::test]
async fn fake_hang_returns_cancelled_when_cancelled() {
    let (_d, e) = engine(&json!({"turns":[{"route":"edit","replies":[],"hang":true}]}), None);
    let t = CancellationToken::new();
    let t2 = t.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        t2.cancel();
    });
    let err = e.complete(&completion(Step::Route), &t).await.unwrap_err();
    assert_that!(err.is_cancelled()).is_true();
}

#[tokio::test]
async fn fake_logs_each_completion() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("log");
    let (_d, e) = engine(&json!({"turns":[{"route":"edit","replies":[{"text":"x"}]}]}), Some(log.clone()));
    let t = CancellationToken::new();
    e.complete(&completion(Step::Route), &t).await.unwrap();
    let mut c = completion(Step::Act);
    c.tools = vec![crate::compact::ToolSpec { name: "slide".into(), description: String::new(), schema: json!({}) }];
    e.complete(&c, &t).await.unwrap();
    let lines: Vec<serde_json::Value> =
        std::fs::read_to_string(&log).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_that!(lines.len()).is_equal_to(2);
    assert_that!(lines[0]["step"].as_str()).is_equal_to(Some("route"));
    assert_that!(lines[1]["step"].as_str()).is_equal_to(Some("act"));
    assert_that!(lines[1]["tools"]).is_equal_to(json!(["slide"]));
    assert_that!(lines[1]["messages"].as_u64()).is_equal_to(Some(0));
}

#[test]
fn fake_invalid_script_is_a_format_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("script.json");
    std::fs::write(&path, "not json").unwrap();
    let err = FakeEngine::from_file(&path, None).unwrap_err();
    assert_that!(matches!(err.0, crate::error::Inner::ScriptFormat { .. })).is_true();
}

#[test]
fn fake_missing_script_file_is_a_read_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = FakeEngine::from_file(&dir.path().join("absent.json"), None).unwrap_err();
    assert_that!(matches!(err.0, crate::error::Inner::Script { .. })).is_true();
}
