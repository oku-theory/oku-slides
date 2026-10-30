//! Texte affiché dans l'interface : paragraphes en markdown éditable
//! (l'inline est écrit par `inline.rs`), et variables du deck remplacées.

use std::collections::BTreeMap;

use oku_slides_core::{ListKind, Paragraph};
use serde_json::Value;

use crate::inline::runs_to_markdown;

/// Editable markdown text: one paragraph per line, `#` to `######` for a heading,
/// `- ` for a bullet, `1. ` for a numbered list, the rest as inline markdown (`inline.rs`).
#[must_use]
pub fn paragraphs_to_text(paras: &[Paragraph]) -> String {
    let mut out = String::new();
    let mut n = 0;
    for (i, p) in paras.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // A heading wins over a list (concurrent merge of the two marks).
        let list = if p.heading.is_some() { None } else { p.list };
        if let Some(h) = p.heading {
            out.push_str(&"#".repeat(usize::from(h.get())));
            out.push(' ');
        }
        match list {
            Some(ListKind::Bullet) => out.push_str("- "),
            Some(ListKind::Number) => {
                n += 1;
                out.push_str(&n.to_string());
                out.push_str(". ");
            }
            None => {}
        }
        // Comme au rendu : tout autre paragraphe clôt la liste numérotée.
        if list != Some(ListKind::Number) {
            n = 0;
        }
        out.push_str(&escape_block_marker(&runs_to_markdown(&p.runs)));
    }
    out
}

/// Text that starts like a prefix (`# `, `- `, `1. `) must not become one.
fn escape_block_marker(line: &str) -> String {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        return format!("\\{line}");
    }
    if line.starts_with("- ") {
        return format!("\\{line}");
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && line[digits..].starts_with(". ") {
        return format!("{}\\{}", &line[..digits], &line[digits..]);
    }
    line.to_owned()
}

/// Remplace `{{nom}}` par la variable du deck (affichage dans l'interface).
#[must_use]
pub fn interp(s: &str, vars: &BTreeMap<String, Value>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start + 2..].find("}}") else { break };
        let key = rest[start + 2..start + 2 + len].trim();
        out.push_str(&rest[..start]);
        match vars.get(key) {
            Some(Value::String(v)) => out.push_str(v),
            Some(v) => out.push_str(&v.to_string()),
            None => out.push_str(&rest[start..start + len + 4]),
        }
        rest = &rest[start + len + 4..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
