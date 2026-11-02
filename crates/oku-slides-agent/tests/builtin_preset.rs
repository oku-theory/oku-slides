//! The built-in preset without the app binary: a prompt fails with the no-program reason.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::{cmd, stack_with_builtin, wait_turn_done};
use oku_slides_i18n::Localizer;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn builtin_preset_without_program_prompt_ends_the_turn_with_no_program() {
    let (stack, rx) = stack_with_builtin(json!({ "turns": [] })).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "renomme le deck" }))).unwrap();
    let (_, done) = wait_turn_done(&rx, "t-1", 1).await;
    let outcome = done["event"]["messages"]
        .as_array()
        .and_then(|ms| ms.iter().find(|m| m["updateDataModel"]["path"] == "/outcome"))
        .map(|m| m["updateDataModel"]["value"].clone());
    // No app binary to run with `--local-agent`: the reason the agent settings show too.
    let no_program = Localizer::new(Some("fr")).tr("agent-local-no-program");
    assert_that!(outcome).is_equal_to(Some(json!(no_program)));
}
