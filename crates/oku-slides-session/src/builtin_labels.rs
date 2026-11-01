//! Inline labels for a copy of a built-in pack, taken from its Fluent keys.
//!
//! A built-in widget (`base/card`) takes its shown name, description, parameter
//! labels and help from Fluent (`param_fields::kind_key`). A copy lives under
//! another namespace and is no longer built-in: before it is saved, each widget
//! source declares them inline instead, as a dictionary over every shipped
//! locale that has the key (`p.plain(label: ("en": "Title", "fr": "Titre"), …)`).
//! Calls are found by a small scanner over the shipped sources (strings and
//! comments skipped, parentheses matched); no Typst runs in-process.
//! The scanner does not model markup: a `"` inside `[…]` content or a raw block of a
//! user-edited built-in can make it misread a call; it then logs and keeps the source verbatim.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::LazyLock;

use uuid::Uuid;

use oku_slides_core::WidgetFile;
use oku_slides_i18n::Localizer;
use oku_slides_render::BuiltinPack;

use crate::deck_pack::builtin_id;

/// The parameter helpers of `params.typ` whose calls carry a label.
const HELPERS: [&str; 10] = ["plain", "rich", "flag", "number", "choice", "asset", "slot", "list", "grid", "host"];

/// One localizer per shipped locale, English first.
static LOCALIZERS: LazyLock<Vec<Localizer>> =
    LazyLock::new(|| oku_slides_i18n::available().into_iter().map(|code| Localizer::new(Some(code))).collect());

/// The built-in pack whose manifest id is `id`, if any.
pub(crate) fn builtin_pack_of(id: Uuid) -> Option<BuiltinPack> {
    BuiltinPack::ALL.into_iter().find(|b| builtin_id(*b) == id)
}

/// The widget files of built-in pack `pack`, each declaring its labels inline.
///
/// The shared `_commun` file stays as is; a source the scanner cannot read is
/// kept verbatim, with a warning.
pub(crate) fn inline_labels(pack: BuiltinPack, widgets: BTreeMap<WidgetFile, String>) -> BTreeMap<WidgetFile, String> {
    let ns = pack.namespace();
    widgets
        .into_iter()
        .map(|(file, source)| {
            if file.is_common() {
                return (file, source);
            }
            let key = format!("{}-{}", ns.as_str(), file.as_str());
            if let Some(rewritten) = labelled(&source, &key, &LOCALIZERS) {
                (file, rewritten)
            } else {
                tracing::warn!(widget = %file, "built-in widget source unreadable: copied without inline labels");
                (file, source)
            }
        })
        .collect()
}

/// What a located call declares: a parameter `name` or the widget itself.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    /// A parameter, `<name>: p.<helper>(`.
    Param(String),
    /// The widget, `widget(`.
    Widget,
}

/// A located call: its target, the index of its `(`, and its named arguments.
#[derive(Debug)]
struct Call {
    target: Target,
    open: usize,
    /// True when only whitespace stands between its parentheses.
    empty: bool,
    named: BTreeSet<String>,
}

/// `source` with label arguments inserted, for widget Fluent key `key` (`base-card`).
///
/// `None` when the scanner cannot read `source` (an unbalanced parenthesis, an
/// unterminated string or comment).
fn labelled(source: &str, key: &str, localizers: &[Localizer]) -> Option<String> {
    let mut calls = scan(source)?;
    calls.sort_by_key(|c| std::cmp::Reverse(c.open));
    let mut out = source.to_owned();
    for call in calls {
        // An author's own label wins: the call stays as written.
        if call.named.contains("label") {
            continue;
        }
        let (label, extra, extra_key) = match &call.target {
            Target::Param(name) => (
                format!("session-widget-param-label-{key}-{name}"),
                "help",
                format!("session-widget-param-help-{key}-{name}"),
            ),
            Target::Widget => {
                (format!("session-widget-kind-{key}"), "description", format!("session-widget-description-{key}"))
            }
        };
        let mut args = Vec::new();
        if let Some(dict) = dictionary(&label, localizers) {
            args.push(format!("label: {dict}"));
        }
        if !call.named.contains(extra)
            && let Some(dict) = dictionary(&extra_key, localizers)
        {
            args.push(format!("{extra}: {dict}"));
        }
        if args.is_empty() {
            continue;
        }
        let mut text = args.join(", ");
        if !call.empty {
            text.push_str(", ");
        }
        out.insert_str(call.open + 1, &text);
    }
    Some(out)
}

/// The Typst dictionary of message `id` by locale; `None` when no locale has it.
fn dictionary(id: &str, localizers: &[Localizer]) -> Option<String> {
    let entries: Vec<String> = localizers
        .iter()
        .filter(|l| l.has(id))
        .map(|l| format!("{}: {}", dict_key(l.locale()), typst_string(&l.tr(id))))
        .collect();
    (!entries.is_empty()).then(|| format!("({})", entries.join(", ")))
}

/// A dictionary key, always quoted: `"zh-Hans"` needs it, and a code may be a Typst keyword (`in`).
fn dict_key(code: &str) -> String {
    typst_string(code)
}

/// `text` as a Typst string literal, its quotes, backslashes and controls escaped.
fn typst_string(text: &str) -> String {
    let mut s = String::with_capacity(text.len() + 2);
    s.push('"');
    for c in text.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(s, "\\u{{{:x}}}", u32::from(c));
            }
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

/// True for a byte of a Typst identifier (`item-label`, `x2`).
fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// The identifier that ends right before `end`, with its start.
fn ident_before(src: &[u8], end: usize) -> Option<(usize, &str)> {
    let start = src[..end].iter().rposition(|b| !is_ident(*b)).map_or(0, |i| i + 1);
    let ident = std::str::from_utf8(&src[start..end]).ok()?;
    (!ident.is_empty() && !ident.starts_with('-')).then_some((start, ident))
}

/// The index of the last non-whitespace byte before `end`.
fn skip_ws_back(src: &[u8], end: usize) -> Option<usize> {
    src[..end].iter().rposition(|b| !b.is_ascii_whitespace())
}

/// What the `(` at `open` calls, if it is a labelled call.
fn target_at(src: &[u8], open: usize) -> Option<Target> {
    let (start, callee) = ident_before(src, open)?;
    let before = start.checked_sub(1).map(|i| src[i]);
    if callee == "widget" {
        return (!before.is_some_and(|b| b == b'.' || is_ident(b))).then_some(Target::Widget);
    }
    if !HELPERS.contains(&callee) || before != Some(b'.') {
        return None;
    }
    let (p_start, module) = ident_before(src, start - 1)?;
    if module != "p" || p_start.checked_sub(1).is_some_and(|i| src[i] == b'.' || is_ident(src[i])) {
        return None;
    }
    let colon = skip_ws_back(src, p_start)?;
    if src[colon] != b':' {
        return None;
    }
    let name_end = skip_ws_back(src, colon)? + 1;
    let (_, name) = ident_before(src, name_end)?;
    Some(Target::Param(name.to_owned()))
}

/// The labelled calls of `source`; `None` if its parentheses, strings or comments do not close.
fn scan(source: &str) -> Option<Vec<Call>> {
    let src = source.as_bytes();
    let mut done = Vec::new();
    // Open parentheses, innermost last: the call each one begins, if any.
    let mut stack: Vec<Option<Call>> = Vec::new();
    let mut i = 0;
    while i < src.len() {
        match src[i] {
            b'"' => i = string_end(src, i)?,
            b'/' if src.get(i + 1) == Some(&b'/') => {
                i = src[i..].iter().position(|b| *b == b'\n').map_or(src.len(), |n| i + n);
            }
            b'/' if src.get(i + 1) == Some(&b'*') => i = block_comment_end(src, i)?,
            b'(' => {
                let empty = src[i + 1..].iter().find(|b| !b.is_ascii_whitespace()) == Some(&b')');
                let call = target_at(src, i).map(|target| Call { target, open: i, empty, named: BTreeSet::new() });
                stack.push(call);
                i += 1;
            }
            b')' => {
                if let Some(call) = stack.pop()? {
                    done.push(call);
                }
                i += 1;
            }
            b if is_ident(b) && !b.is_ascii_digit() && b != b'-' && (i == 0 || !is_ident(src[i - 1])) => {
                let end = src[i..].iter().position(|b| !is_ident(*b)).map_or(src.len(), |n| i + n);
                let next = src[end..].iter().find(|b| !b.is_ascii_whitespace());
                if next == Some(&b':')
                    && let Some(Some(call)) = stack.last_mut()
                {
                    call.named.insert(std::str::from_utf8(&src[i..end]).ok()?.to_owned());
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    stack.is_empty().then_some(done)
}

/// The index right after the string that opens at `start`.
fn string_end(src: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// The index right after the block comment that opens at `start`; they nest.
fn block_comment_end(src: &[u8], start: usize) -> Option<usize> {
    let (mut depth, mut i) = (0usize, start);
    while i + 1 < src.len() {
        match (src[i], src[i + 1]) {
            (b'/', b'*') => (depth, i) = (depth + 1, i + 2),
            (b'*', b'/') => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}

#[cfg(test)]
#[path = "builtin_labels_tests.rs"]
mod builtin_labels_tests;
