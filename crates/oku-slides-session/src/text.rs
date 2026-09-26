//! Texte affiché dans l'interface : paragraphes en markdown éditable, et
//! variables du deck remplacées.

use std::collections::BTreeMap;

use oku_slides_core::{ListKind, Paragraph, Run};
use serde_json::Value;

/// Texte éditable en markdown : un paragraphe par ligne, `- ` pour une puce,
/// `1. ` pour une liste numérotée ; `**gras**`, `*italique*`, `[texte](url)`.
///
/// `color` et `size` des runs n'ont pas d'écriture markdown : ils sont omis.
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
        out.push_str(&escape_list_marker(&runs_to_markdown(&p.runs)));
    }
    out
}

/// Runs en markdown inline ; les runs voisins de même format ne font qu'un.
fn runs_to_markdown(runs: &[Run]) -> String {
    let same =
        |a: &Run, b: &Run| a.bold == b.bold && a.italic == b.italic && a.link == b.link && a.highlight == b.highlight;
    let mut merged: Vec<Run> = Vec::new();
    for r in runs.iter().filter(|r| !r.t.is_empty()) {
        match merged.last_mut() {
            Some(last) if same(last, r) => last.t.push_str(&r.t),
            _ => merged.push(r.clone()),
        }
    }
    let mut out = String::new();
    let mut after_highlight = false;
    for Run { t, bold, italic, link, highlight, .. } in merged {
        let marks = match (bold, italic) {
            (true, true) => "***",
            (true, false) => "**",
            (false, true) => "*",
            (false, false) => "",
        };
        let mut body = format!("{marks}{}{marks}", escape_inline(&t));
        // `{n}` juste après une pastille serait lu comme sa couleur.
        if after_highlight && body.starts_with('{') {
            body.insert(0, '\\');
        }
        after_highlight = highlight.is_some() && link.is_none();
        if let Some(n) = highlight {
            let color = if n == 1 { String::new() } else { format!("{{{n}}}") };
            body = format!("=={body}=={color}");
        }
        match link {
            Some(url) => {
                let url = url.replace(' ', "%20").replace('(', "%28").replace(')', "%29");
                out.push_str(&format!("[{body}]({url})"));
            }
            None => out.push_str(&body),
        }
    }
    out
}

/// Échappe ce qui se lirait comme du markdown ; `_` au milieu d'un mot et
/// `=` isolé restent tels quels.
fn escape_inline(t: &str) -> String {
    let chars: Vec<char> = t.chars().collect();
    let mut out = String::with_capacity(t.len());
    for (i, &c) in chars.iter().enumerate() {
        let intraword = || {
            let alnum = |j: Option<usize>| j.and_then(|j| chars.get(j)).is_some_and(|c| c.is_alphanumeric());
            alnum(i.checked_sub(1)) && alnum(Some(i + 1))
        };
        // `=` contre un autre `=` ou au bord du run pourrait former `==`.
        let equals = || {
            [i.checked_sub(1), Some(i + 1)].into_iter().any(|j| j.and_then(|j| chars.get(j)).is_none_or(|&n| n == '='))
        };
        if matches!(c, '\\' | '*' | '[' | ']') || (c == '_' && !intraword()) || (c == '=' && equals()) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Un texte qui commence comme une liste (`- `, `1. `) ne doit pas en devenir une.
fn escape_list_marker(line: &str) -> String {
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
