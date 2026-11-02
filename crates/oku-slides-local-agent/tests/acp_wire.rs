//! The local agent on the wire: raw ACP lines on its stdio, fake engine (debug builds).
#![cfg(debug_assertions)]
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};
use speculoos::prelude::*;

/// The local agent child, its stdin, and its stdout lines read on a thread.
struct Agent {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
    _dir: tempfile::TempDir,
}

impl Agent {
    fn start() -> Self {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("dossier : {e}"));
        let script = dir.path().join("script.json");
        std::fs::write(&script, r#"{"turns":[]}"#).unwrap_or_else(|e| panic!("scénario : {e}"));
        let mut child = Command::new(env!("CARGO_BIN_EXE_oku-slides-local-agent-dev"))
            .arg("--local-agent")
            .env("OKU_SLIDES_LOCAL_AGENT__ENGINE", "fake")
            .env("OKU_SLIDES_LOCAL_AGENT__FAKE_SCRIPT", &script)
            .env("OKU_SLIDES_PATHS__DATA", dir.path())
            .env("OKU_SLIDES_PATHS__DECKS", dir.path().join("decks"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("agent local : {e}"));
        let stdin = child.stdin.take().unwrap_or_else(|| panic!("stdin"));
        let stdout = child.stdout.take().unwrap_or_else(|| panic!("stdout"));
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    return;
                }
            }
        });
        Self { child, stdin, lines, _dir: dir }
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").unwrap_or_else(|e| panic!("écriture : {e}"));
    }

    /// The reply to request `id`, within 10 s; notifications are skipped.
    fn reply(&self, id: u64) -> Value {
        loop {
            let line =
                self.lines.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|e| panic!("réponse {id} : {e}"));
            let value: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("JSON : {e}"));
            if value["id"] == json!(id) {
                return value;
            }
        }
    }

    fn initialize(&mut self) -> Value {
        self.send(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": 1, "clientCapabilities": {}}}));
        self.reply(1)
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// JSON-RPC code of ACP's `resource_not_found`, which the host handles as `Refused`.
const RESOURCE_NOT_FOUND: i64 = -32002;

fn mcp_servers(url: &str) -> Value {
    json!([{"type": "http", "name": "oku-slides", "url": url,
        "headers": [{"name": "Authorization", "value": "Bearer t"}]}])
}

#[test]
fn acp_initialize_advertises_load_session() {
    let mut agent = Agent::start();
    let reply = agent.initialize();
    assert_that!(reply["result"]["agentCapabilities"]["loadSession"]).is_equal_to(json!(true));
}

#[test]
fn acp_load_of_an_unknown_session_is_resource_not_found() {
    let mut agent = Agent::start();
    agent.initialize();
    agent.send(&json!({"jsonrpc": "2.0", "id": 2, "method": "session/load",
        "params": {"sessionId": "local-0123456789abcdef0123456789abcdef-1", "cwd": "/", "mcpServers": mcp_servers("http://127.0.0.1:9/mcp")}}));
    assert_that!(agent.reply(2)["error"]["code"]).is_equal_to(json!(RESOURCE_NOT_FOUND));
}

#[test]
fn acp_load_from_another_tabs_mcp_url_is_resource_not_found() {
    let mut agent = Agent::start();
    agent.initialize();
    agent.send(&json!({"jsonrpc": "2.0", "id": 2, "method": "session/new",
        "params": {"cwd": "/", "mcpServers": mcp_servers("http://127.0.0.1:9/mcp")}}));
    let id = agent.reply(2)["result"]["sessionId"].clone();
    agent.send(&json!({"jsonrpc": "2.0", "id": 3, "method": "session/load",
        "params": {"sessionId": id, "cwd": "/", "mcpServers": mcp_servers("http://127.0.0.1:8/mcp")}}));
    assert_that!(agent.reply(3)["error"]["code"]).is_equal_to(json!(RESOURCE_NOT_FOUND));
}

#[test]
fn acp_load_of_a_known_session_answers() {
    let mut agent = Agent::start();
    agent.initialize();
    agent.send(&json!({"jsonrpc": "2.0", "id": 2, "method": "session/new",
        "params": {"cwd": "/", "mcpServers": mcp_servers("http://127.0.0.1:9/mcp")}}));
    let id = agent.reply(2)["result"]["sessionId"].clone();
    agent.send(&json!({"jsonrpc": "2.0", "id": 3, "method": "session/load",
        "params": {"sessionId": id, "cwd": "/", "mcpServers": mcp_servers("http://127.0.0.1:9/mcp")}}));
    let reply = agent.reply(3);
    assert_that!(reply.get("error")).is_none();
    assert_that!(reply["result"].is_object()).is_true();
}

#[test]
fn acp_cancel_during_the_first_mcp_connect_ends_the_prompt_cancelled() {
    // A server that accepts and never answers: the first connect hangs.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("écoute : {e}"));
    let url = format!("http://{}/mcp", listener.local_addr().unwrap_or_else(|e| panic!("adresse : {e}")));
    std::thread::spawn(move || {
        let held: Vec<_> = listener.incoming().map_while(Result::ok).collect();
        drop(held);
    });
    let mut agent = Agent::start();
    agent.initialize();
    agent.send(&json!({"jsonrpc": "2.0", "id": 2, "method": "session/new",
        "params": {"cwd": "/", "mcpServers": mcp_servers(&url)}}));
    let id = agent.reply(2)["result"]["sessionId"].clone();
    agent.send(&json!({"jsonrpc": "2.0", "id": 3, "method": "session/prompt",
        "params": {"sessionId": id, "prompt": [{"type": "text", "text": "rename the deck"}]}}));
    std::thread::sleep(Duration::from_millis(300));
    agent.send(&json!({"jsonrpc": "2.0", "method": "session/cancel", "params": {"sessionId": id}}));
    assert_that!(agent.reply(3)["result"]["stopReason"]).is_equal_to(json!("cancelled"));
}
