//! Prompt ChatML de Qwen3 : système, bloc `<tools>`, appels et résultats d'outils.
//!
//! Rendu ici plutôt que par le modèle de chat du GGUF : llama.cpp n'y passe pas les outils.
//!
//! Le prompt est tokenisé avec les jetons spéciaux (ceux du gabarit) : tout texte inséré
//! (système, messages, résultats et descriptions d'outils) est non fiable. `turn` remplace
//! donc chaque `<|` par `< |`, pour qu'aucun corps n'ouvre ni ne ferme un tour, et `plain`
//! neutralise de même les balises d'outils et de réflexion de Qwen (`<tool…`, `</tool…`,
//! `<think>`, `</think>`) dans chaque corps inséré, avant qu'il soit enveloppé des vraies balises.

use serde_json::json;

use crate::engine::{Completion, Message};

const TOOLS_HEAD: &str = "# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n";
const TOOLS_TAIL: &str = "</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>";

/// Le prompt complet, terminé par l'ouverture du tour de l'assistant.
pub(crate) fn render(c: &Completion) -> String {
    let mut system = plain(&c.system);
    if !c.tools.is_empty() {
        system.push_str("\n\n");
        system.push_str(TOOLS_HEAD);
        for t in &c.tools {
            let line = json!({"type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.schema}});
            system.push_str(&plain(&line.to_string()));
            system.push('\n');
        }
        system.push_str(TOOLS_TAIL);
    }
    let mut out = turn("system", &system);
    for m in &c.messages {
        out.push_str(&match m {
            Message::User(text) => turn("user", &plain(text)),
            Message::Assistant(text) => turn("assistant", &plain(text)),
            Message::Call { name, args } => turn(
                "assistant",
                &format!("<tool_call>\n{}\n</tool_call>", plain(&json!({"name": name, "arguments": args}).to_string())),
            ),
            Message::ToolResult { output, .. } => {
                turn("user", &format!("<tool_response>\n{}\n</tool_response>", plain(output)))
            }
        });
    }
    out.push_str("<|im_start|>assistant\n");
    out
}

/// Ouverture des marqueurs spéciaux de ChatML (`<|im_start|>`, `<|im_end|>`…).
const SPECIAL_OPEN: &str = "<|";

/// Rend inertes les balises d'outils et de réflexion d'un corps non fiable.
fn plain(body: &str) -> String {
    body.replace("</tool", "< /tool")
        .replace("<tool", "< tool")
        .replace("</think>", "< /think>")
        .replace("<think>", "< think>")
}

fn turn(role: &str, content: &str) -> String {
    let content = content.replace(SPECIAL_OPEN, "< |");
    format!("<|im_start|>{role}\n{content}<|im_end|>\n")
}

#[cfg(test)]
#[path = "chatml_tests.rs"]
mod chatml_tests;
