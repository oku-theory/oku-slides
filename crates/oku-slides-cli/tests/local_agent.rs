//! `oku-slides --local-agent` répond à la poignée de main ACP.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use speculoos::prelude::*;

// Le moteur factice n'existe qu'en build de débogage.
#[cfg(debug_assertions)]
#[test]
fn local_agent_flag_answers_initialize() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("script.json");
    std::fs::write(&script, r#"{"turns":[]}"#).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_oku-slides"))
        .arg("--local-agent")
        .env("OKU_SLIDES_LOCAL_AGENT__ENGINE", "fake")
        .env("OKU_SLIDES_LOCAL_AGENT__FAKE_SCRIPT", &script)
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
    // Read on a thread: a silent agent fails the test in 30 s instead of hanging it.
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let read = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = tx.send(read);
    });
    let line = rx.recv_timeout(Duration::from_secs(30));
    child.kill().unwrap();
    child.wait().unwrap();
    let reply: serde_json::Value = serde_json::from_str(&line.unwrap().unwrap()).unwrap();
    assert_that!(reply["id"]).is_equal_to(serde_json::json!(1));
    assert_that!(reply["result"]["agentCapabilities"]["mcpCapabilities"]["http"]).is_equal_to(serde_json::json!(true));
}
