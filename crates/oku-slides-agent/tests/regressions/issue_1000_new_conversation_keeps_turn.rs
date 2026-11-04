//! t-1000: the `new_conversation` tool starts the conversation over once its turn has ended. The
//! turn's receipt must still show first: starting over clears the thread, and a receipt put after
//! that has no turn left to go in. Then the thread is new: the next prompt opens a new session.

use std::time::Duration;

use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{Relayed, cmd, is_tool_surface, logged, stack_with_sink_and_log, wait_for};

/// Whether `v` closes turn 1 of `t-1`'s thread: the thread starting over.
fn is_new_thread(ch: &str, v: &Value) -> bool {
    ch == "mcp" && v["tab"] == "t-1" && v["event"]["kind"] == "close_surface" && v["event"]["id"] == "turn-1"
}

/// Where the first relayed event matching `pred` is.
fn position(rx: &Relayed, pred: impl Fn(&str, &Value) -> bool) -> Option<usize> {
    rx.lock().unwrap_or_else(|e| panic!("relay: {e}")).iter().position(|(ch, v)| pred(ch, v))
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1000_new_conversation_keeps_turn() {
    let script = json!({ "by_prompt": {
        "on recommence": { "mcp": [{ "tool": "new_conversation", "args": {} }], "chunks": ["On repart de zéro."] },
        "et ensuite": { "chunks": ["Me voilà."] }
    } });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1"]).await;
    let prompt = |text: &str| {
        let asked = json!({ "cmd": "prompt", "tab": "t-1", "text": text });
        stack.handle(cmd(asked)).unwrap_or_else(|e| panic!("prompt: {e}"));
    };
    prompt("on recommence");
    wait_for(&rx, |(ch, v)| is_new_thread(ch, v)).await;
    let receipt = position(&rx, |_, v| is_tool_surface(v, "t-1", "report-"));
    let fresh = position(&rx, is_new_thread);
    assert_that!(receipt.is_some()).is_true();
    assert_that!(receipt < fresh).is_true();
    prompt("et ensuite");
    let reopened = tokio::time::timeout(Duration::from_secs(10), async {
        while logged(&log, "session/new").len() < 2 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert_that!(reopened).is_ok();
}
