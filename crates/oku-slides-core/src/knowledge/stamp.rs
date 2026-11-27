//! Provenance stamp written into each knowledge file an agent batch authors.
//!
//! A concept file gets a `generated` mapping (and `x-oku-slides-agent` when the agent is known)
//! at the end of its frontmatter; an `index.md` or `log.md` gets a last comment line. The id is
//! random, so an agent cannot write a text whose stamped form it knows in advance: what it writes
//! never hashes to a received file's text. Lines are found by text, never by parser markers, and
//! every byte outside the replaced entries' line ranges is kept (comment lines trailing a replaced
//! entry go with it).

use std::fmt::Write as _;

use saphyr_parser::{Event, Parser};
use time::{OffsetDateTime, UtcOffset};

use super::KnowledgePath;
use super::frontmatter::{lines, parse_block, split};
use crate::error::FrontmatterReason;

/// The start of the stamp comment line of an `index.md` or `log.md`.
const COMMENT_PREFIX: &str = "<!-- oku-slides generated ";
/// The frontmatter key of the provenance mapping.
const GENERATED: &str = "generated";
/// The frontmatter key of the agent label.
const AGENT: &str = "x-oku-slides-agent";

/// Who authored a knowledge write, when, and under which unpredictable id.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Provenance {
    /// The writing tool, `oku-slides/<version>`.
    pub by: String,
    /// When the batch was applied.
    pub at: OffsetDateTime,
    /// Random bytes no agent can predict.
    pub id: [u8; 8],
    /// The agent label, when the turn has one.
    pub agent: Option<String>,
}

impl Provenance {
    /// A provenance from its parts.
    #[must_use]
    pub fn new(by: String, at: OffsetDateTime, id: [u8; 8], agent: Option<String>) -> Self {
        Self { by, at, id, agent }
    }

    /// `at` in UTC, RFC 3339 with exactly three fraction digits.
    fn at_text(&self) -> String {
        let at = self.at.to_offset(UtcOffset::UTC);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            at.year(),
            u8::from(at.month()),
            at.day(),
            at.hour(),
            at.minute(),
            at.second(),
            at.millisecond()
        )
    }
}

/// Stamps `text`, to be written at `path`, with `p`.
///
/// A concept file loses its earlier `generated` and `x-oku-slides-agent` entries and gets new
/// ones at the end of its frontmatter; without a frontmatter it is returned unchanged, for
/// [`super::check_text`] to refuse. An `index.md` or `log.md` loses its earlier stamp comment
/// lines and gets one as its last line. The line end follows the text's first line.
///
/// # Errors
///
/// The structural [`FrontmatterReason`]s of the block, and [`FrontmatterReason::FlowMapping`]
/// when its top-level keys are not one per column-0 line, or it holds a `...` end.
pub fn stamp(path: &KnowledgePath, text: &str, p: &Provenance) -> Result<String, FrontmatterReason> {
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", text),
    };
    let eol = if body.split('\n').next().is_some_and(|first| first.ends_with('\r')) { "\r\n" } else { "\n" };
    if path.is_index() || path.is_log() {
        return Ok(format!("{bom}{}", stamp_comment(body, eol, p)));
    }
    let Ok((yaml, _)) = split(text) else { return Ok(text.to_owned()) };
    let yaml_start = body.find('\n').map_or(body.len(), |i| i + 1);
    let yaml_end = yaml_start + yaml.len();
    let mut out = String::with_capacity(text.len() + 160);
    out.push_str(bom);
    out.push_str(&body[..yaml_start]);
    out.push_str(&restamp_yaml(yaml, eol, p)?);
    out.push_str(&body[yaml_end..]);
    Ok(out)
}

/// `body` without its stamp comment lines, with a new one last.
fn stamp_comment(body: &str, eol: &str, p: &Provenance) -> String {
    let mut out = String::with_capacity(body.len() + 64);
    for (start, line, next) in lines(body) {
        if !line.starts_with(COMMENT_PREFIX) {
            out.push_str(&body[start..next]);
        }
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(eol);
    }
    let _ = write!(out, "{COMMENT_PREFIX}{} {} -->{eol}", p.at_text(), hex::encode(p.id));
    out
}

/// `yaml` without its `generated` and agent entries, with new ones last.
fn restamp_yaml(yaml: &str, eol: &str, p: &Provenance) -> Result<String, FrontmatterReason> {
    let parsed: Vec<String> = parse_block(yaml)?.into_iter().map(|(key, _)| key).collect();
    let mut found = Vec::new();
    let mut out = String::with_capacity(yaml.len() + 160);
    // Whether the lines being read belong to a replaced entry.
    let mut dropping = false;
    for (start, line, next) in lines(yaml) {
        if is_document_end(line) {
            return Err(FrontmatterReason::FlowMapping);
        }
        if starts_key(line) {
            let key = key_of(line).ok_or(FrontmatterReason::FlowMapping)?;
            dropping = key == GENERATED || key == AGENT;
            found.push(key);
        }
        if !dropping {
            out.push_str(&yaml[start..next]);
        }
    }
    if found != parsed {
        return Err(FrontmatterReason::FlowMapping);
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(eol);
    }
    let _ = write!(out, "{GENERATED}:{eol}");
    let _ = write!(out, "  by: {}{eol}", json_string(&p.by));
    let _ = write!(out, "  at: {}{eol}", json_string(&p.at_text()));
    let _ = write!(out, "  id: {}{eol}", json_string(&hex::encode(p.id)));
    if let Some(agent) = &p.agent {
        let _ = write!(out, "{AGENT}: {}{eol}", json_string(agent));
    }
    Ok(out)
}

/// `text` without the stamp [`stamp`] writes, for tests that compare what an agent wrote.
///
/// Drops, with the same line rules, the frontmatter's `generated` and `x-oku-slides-agent`
/// entries and every stamp comment line; the line end `stamp` may have added stays.
#[cfg(any(test, feature = "test-support"))]
#[must_use]
pub fn unstamped(text: &str) -> String {
    without_entries(text, &[], true)
}

/// A concept file's `text` without its stamp entries nor its top-level frontmatter `keys`,
/// every other byte kept: what was written, whoever stamped it since.
///
/// Entries go by the line rules of [`stamp`]; a text without a frontmatter is kept whole.
#[must_use]
pub fn unstamped_without_keys(text: &str, keys: &[&str]) -> String {
    without_entries(text, keys, false)
}

/// `text` without its frontmatter's stamp entries and `keys`, and its stamp comment lines when
/// `comments`.
fn without_entries(text: &str, keys: &[&str], comments: bool) -> String {
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", text),
    };
    let yaml = split(text).ok().map(|(yaml, _)| {
        let start = body.find('\n').map_or(body.len(), |i| i + 1);
        start..start + yaml.len()
    });
    let mut out = String::from(bom);
    let mut dropping = false;
    for (start, line, next) in lines(body) {
        let in_yaml = yaml.as_ref().is_some_and(|y| y.contains(&start));
        if in_yaml && starts_key(line) {
            dropping = key_of(line).is_some_and(|key| key == GENERATED || key == AGENT || keys.contains(&key.as_str()));
        }
        if (in_yaml && dropping) || (comments && line.starts_with(COMMENT_PREFIX)) {
            continue;
        }
        out.push_str(&body[start..next]);
    }
    out
}

/// A `...` document end line.
fn is_document_end(line: &str) -> bool {
    line.strip_prefix("...").is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t']))
}

/// A column-0 line that is not blank, a comment or a sequence item: it opens a top-level key.
fn starts_key(line: &str) -> bool {
    match line.chars().next() {
        None | Some(' ' | '\t' | '#') => false,
        Some('-') => !matches!(line.chars().nth(1), None | Some(' ' | '\t')),
        Some(_) => true,
    }
}

/// The key a key line opens, decoded as YAML reads it; `None` when it cannot be read alone.
fn key_of(line: &str) -> Option<String> {
    let source = if let quote @ ('"' | '\'') = line.chars().next()? {
        let end = closing_quote(line, quote)?;
        line[end + 1..].trim_start_matches([' ', '\t']).starts_with(':').then_some(&line[..=end])?
    } else {
        let colon = line
            .match_indices(':')
            .map(|(i, _)| i)
            .find(|&i| matches!(line[i + 1..].chars().next(), None | Some(' ' | '\t')))?;
        line[..colon].trim_end_matches([' ', '\t'])
    };
    single_scalar(source)
}

/// The byte offset of the quote closing the scalar that `line` opens with `quote`.
fn closing_quote(line: &str, quote: char) -> Option<usize> {
    let mut chars = line.char_indices().skip(1).peekable();
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            ('"', '\\') => {
                chars.next();
            }
            ('\'', '\'') if chars.peek().is_some_and(|&(_, n)| n == '\'') => {
                chars.next();
            }
            _ if c == quote => return Some(i),
            _ => {}
        }
    }
    None
}

/// The text of `source` when it is exactly one untagged, unanchored scalar.
fn single_scalar(source: &str) -> Option<String> {
    let mut text = None;
    for event in Parser::new_from_str(source) {
        match event.ok()?.0 {
            Event::StreamStart | Event::StreamEnd | Event::DocumentStart(_) | Event::DocumentEnd | Event::Nothing => {}
            Event::Scalar(value, _, 0, None) if text.is_none() => text = Some(value.into_owned()),
            _ => return None,
        }
    }
    text
}

/// `text` as a JSON string literal that YAML reads back the same: controls, line and paragraph
/// separators and non-characters are `\u` escapes.
#[must_use]
pub fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{fffe}' | '\u{ffff}') => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
#[path = "stamp_tests.rs"]
mod stamp_tests;
