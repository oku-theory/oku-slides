//! `oku-slides --local-agent` répond à la poignée de main ACP.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use speculoos::prelude::*;

// Le moteur factice n'existe qu'en build de débogage.
#[cfg(debug_assertions)]
#[test]
fn local_agent_flag_answers_initialize() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("script.json");
    std::fs::write(&script, r#"{"turns":[]}"#).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_oku-slides"))
        .args(["--local-agent", "--engine", "fake", "--script"])
        .arg(&script)
        .env("OKU_SLIDES_PATHS__DATA", dir.path())
        .env("OKU_SLIDES_PATHS__DECKS", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":1,"clientCapabilities":{{}}}}}}"#
    )
    .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    let reply: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_that!(reply["id"]).is_equal_to(serde_json::json!(1));
    assert_that!(reply["result"]["agentCapabilities"]["mcpCapabilities"]["http"]).is_equal_to(serde_json::json!(true));
}
