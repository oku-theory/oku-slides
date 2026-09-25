//! Texte affiché dans l'interface : paragraphes en texte éditable, et
//! variables du deck remplacées.

use std::collections::BTreeMap;

use serde_json::Value;
use slider_core::{ListKind, Paragraph};

/// Texte éditable : un paragraphe par ligne, `- ` pour une puce, `1. ` pour une liste numérotée.
pub fn paragraphs_to_text(paras: &[Paragraph]) -> String {
    let mut out = String::new();
    let mut n = 0;
    for (i, p) in paras.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match p.list {
            Some(ListKind::Bullet) => out.push_str("- "),
            Some(ListKind::Number) => {
                n += 1;
                out.push_str(&n.to_string());
                out.push_str(". ");
            }
            None => {}
        }
        for r in &p.runs {
            out.push_str(&r.t);
        }
    }
    out
}

/// Remplace `{{nom}}` par la variable du deck (affichage dans l'interface).
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
