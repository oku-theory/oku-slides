use serde_json::json;
use speculoos::prelude::*;

use super::{render, render_call};
use crate::compact::ToolSpec;
use crate::engine::{Completion, Message, Step};

const CONTEXT: &str =
    "<oku-slides-context>\ninterface language: Français (fr)\n</oku-slides-context>\n\nRenomme le deck";

fn completion(step: Step, messages: Vec<Message>) -> Completion {
    let report = ToolSpec {
        name: "report".into(),
        description: "Report back to the user.".into(),
        schema: json!({"type": "object"}),
    };
    Completion { step, system: "You are the assistant.".into(), messages, tools: vec![report], offered: vec![] }
}

#[test]
fn render_history_is_text_in_order() {
    let messages = vec![
        Message::User(CONTEXT.into()),
        Message::Assistant("Reading the deck.".into()),
        Message::Call { name: "apply".into(), args: json!({"ops": []}) },
        Message::ToolResult { name: "apply".into(), output: "applied".into() },
    ];
    let prompt = render(&completion(Step::Act, messages)).prompt;
    let expected = format!(
        "User: {CONTEXT}\n\nAssistant: Reading the deck.\n\nAssistant called apply with {{\"ops\":[]}}\n\n\
         Result of apply: applied\n\nYour next step: pick the one tool to call now."
    );
    assert_that!(prompt).is_equal_to(expected);
}

#[test]
fn render_keeps_the_interface_locale_line() {
    let prompt = render(&completion(Step::Act, vec![Message::User(CONTEXT.into())])).prompt;
    assert_that!(prompt.as_str()).contains("interface language: Français (fr)");
}

#[test]
fn render_act_lists_tools_in_instructions() {
    let instructions = render(&completion(Step::Act, vec![])).instructions;
    assert_that!(instructions.as_str()).starts_with("You are the assistant.\n\nTools");
    assert_that!(instructions.as_str()).ends_with("\n- report: Report back to the user.");
}

#[test]
fn render_route_without_tools_keeps_system_and_asks_for_families() {
    let mut c = completion(Step::Route, vec![Message::User("Make the title bigger".into())]);
    c.tools.clear();
    let rendered = render(&c);
    assert_that!(rendered.instructions.as_str()).is_equal_to("You are the assistant.");
    assert_that!(rendered.prompt.as_str()).ends_with("Pick the one or two tool families this request needs.");
}

#[test]
fn render_call_names_only_the_picked_tool_and_the_ask() {
    let c = completion(Step::Act, vec![Message::User(CONTEXT.into())]);
    let save = ToolSpec { name: "save".into(), description: "Save the deck.".into(), schema: json!({}) };
    let rendered = render_call(&c, &save, "Call save: write its arguments.");
    assert_that!(rendered.instructions.as_str()).is_equal_to("You are the assistant.\n\nTool save: Save the deck.");
    assert_that!(rendered.prompt.as_str()).ends_with("\n\nCall save: write its arguments.");
    assert_that!(rendered.prompt.as_str()).contains("interface language: Français (fr)");
}
