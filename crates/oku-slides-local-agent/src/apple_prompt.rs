//! Instructions and prompt of one Apple completion: a fresh session, the history as text.
//!
//! Each completion opens a new `LanguageModelSession`, so the whole conversation (requests,
//! notes, calls and tool results) is written into one prompt. The host's context block, with
//! the interface language, stays verbatim in the first user message. The pick step lists
//! the tools' names and one-line descriptions in the instructions (`render`); the kinds and
//! call steps name only the picked tool (`render_call`), its schema in the guided reply.

use std::fmt::Write as _;

use crate::compact::ToolSpec;
use crate::engine::{Completion, Message, Step};

/// What one Apple completion sends: the session's instructions and its prompt.
#[derive(Debug, PartialEq)]
pub(crate) struct Rendered {
    /// System prompt, with the tool list on the Act step.
    pub instructions: String,
    /// The conversation so far, then what to do now.
    pub prompt: String,
}

/// The instructions and prompt of `c`.
pub(crate) fn render(c: &Completion) -> Rendered {
    let mut instructions = c.system.clone();
    if !c.tools.is_empty() {
        instructions.push_str("\n\nTools (reply with the name of the one to call now):");
        for t in &c.tools {
            // `write!` to a `String` cannot fail.
            let _ = write!(instructions, "\n- {}: {}", t.name, t.description);
        }
    }
    let mut parts: Vec<String> = c.messages.iter().map(message).collect();
    parts.push(
        match c.step {
            Step::Route => "Pick the one or two tool families this request needs.",
            Step::Act => "Your next step: pick the one tool to call now.",
        }
        .to_owned(),
    );
    Rendered { instructions, prompt: parts.join("\n\n") }
}

/// The instructions and prompt of a step about `tool` only: the conversation, then `ask`.
pub(crate) fn render_call(c: &Completion, tool: &ToolSpec, ask: &str) -> Rendered {
    let instructions = format!("{}\n\nTool {}: {}", c.system, tool.name, tool.description);
    let mut parts: Vec<String> = c.messages.iter().map(message).collect();
    parts.push(ask.to_owned());
    Rendered { instructions, prompt: parts.join("\n\n") }
}

fn message(m: &Message) -> String {
    match m {
        Message::User(text) => format!("User: {text}"),
        Message::Assistant(text) => format!("Assistant: {text}"),
        Message::Call { name, args } => format!("Assistant called {name} with {args}"),
        Message::ToolResult { name, output } => format!("Result of {name}: {output}"),
    }
}

#[cfg(test)]
#[path = "apple_prompt_tests.rs"]
mod apple_prompt_tests;
