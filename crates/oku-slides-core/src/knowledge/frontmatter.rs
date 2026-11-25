//! OKF frontmatter: a `---` block of YAML at the top of a knowledge file, validated strictly.
//!
//! The YAML is read as parser events (`saphyr-parser`), never loaded into a value API: that is
//! what lets the check refuse anchors, aliases and tags before anything is expanded, stop after
//! the first document and bound the nesting depth.

use std::collections::HashSet;

use saphyr_parser::{Event, Parser, ScalarStyle};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::KnowledgePath;
use super::limits::{MAX_FRONTMATTER_BYTES, MAX_YAML_DEPTH};
use crate::error::FrontmatterReason;

/// Keys the root `index.md` may carry; no other `index.md` and no `log.md` carries any.
const ROOT_INDEX_KEYS: [&str; 4] = ["okf_version", "title", "description", "license"];

/// The part of a frontmatter block the bundle index reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Frontmatter {
    /// The `type` value, when it is text.
    pub kind: Option<String>,
    /// The `title` value, when it is text.
    pub title: Option<String>,
    /// The `description` value, when it is text.
    pub description: Option<String>,
    /// Every top-level key, in file order.
    pub keys: Vec<String>,
}

/// A parsed YAML node, kept only as far as validation and the index need it.
enum Node {
    Scalar { text: String, plain: bool },
    Map(Vec<(String, Node)>),
    Seq,
}

impl Node {
    /// The text of a scalar that is not a plain YAML null (`~`, `null`, empty).
    fn text(&self) -> Option<&str> {
        match self {
            Node::Scalar { text, plain }
                if !(*plain && matches!(text.as_str(), "" | "~" | "null" | "Null" | "NULL")) =>
            {
                Some(text)
            }
            _ => None,
        }
    }
}

/// A collection being built while its events arrive.
enum Open {
    Map { entries: Vec<(String, Node)>, seen: HashSet<String>, key: Option<String> },
    Seq,
}

/// Splits `text` into its YAML block and the body after the closing line.
///
/// One leading BOM is tolerated; the fence lines are `---` with a `\n` or `\r\n` end.
///
/// # Errors
///
/// [`FrontmatterReason::Missing`] when the text does not open with a `---` line,
/// [`FrontmatterReason::Unterminated`] when no closing `---` line follows.
pub fn split(text: &str) -> Result<(&str, &str), FrontmatterReason> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = lines(text);
    match lines.next() {
        Some((_, "---", _)) => {}
        _ => return Err(FrontmatterReason::Missing),
    }
    let yaml_start = match text.find('\n') {
        Some(i) => i + 1,
        None => return Err(FrontmatterReason::Unterminated),
    };
    for (start, line, next) in lines {
        if line == "---" {
            return Ok((&text[yaml_start..start], &text[next..]));
        }
    }
    Err(FrontmatterReason::Unterminated)
}

/// The lines of `text` as (start offset, content without `\r\n`, offset of the next line).
fn lines(text: &str) -> impl Iterator<Item = (usize, &str, usize)> {
    let mut at = 0;
    std::iter::from_fn(move || {
        if at >= text.len() {
            return None;
        }
        let start = at;
        let (end, next) = match text[start..].find('\n') {
            Some(i) => (start + i, start + i + 1),
            None => (text.len(), text.len()),
        };
        at = next;
        Some((start, text[start..end].strip_suffix('\r').unwrap_or(&text[start..end]), next))
    })
}

/// Reads the frontmatter of a file: the structure is checked (see [`check_text`] for the content
/// rules), and the fields the index lists are returned.
///
/// # Errors
///
/// The structural [`FrontmatterReason`]s: `Missing`, `Unterminated`, `TooLarge`, `NotYaml`,
/// `NotMapping`, `DuplicateKey`, `Alias`, `Tag`, `TooDeep`.
pub fn frontmatter(text: &str) -> Result<Frontmatter, FrontmatterReason> {
    let (yaml, _) = split(text)?;
    let entries = parse_block(yaml)?;
    let scalar = |name: &str| entries.iter().find(|(k, _)| k == name).and_then(|(_, v)| v.text()).map(str::to_owned);
    Ok(Frontmatter {
        kind: scalar("type"),
        title: scalar("title"),
        description: scalar("description"),
        keys: entries.into_iter().map(|(k, _)| k).collect(),
    })
}

/// Checks the frontmatter of a file written at `path`.
///
/// A concept file needs a non-empty text `type`, and `generated` when present is a mapping whose
/// `at` is an RFC 3339 timestamp with an offset; other keys are free. The root `index.md` may
/// carry only `okf_version`, `title`, `description` and `license`, any other `index.md` and every
/// `log.md` carry none (and need none).
///
/// # Errors
///
/// A [`FrontmatterReason`] naming the first thing wrong.
pub fn check_text(path: &KnowledgePath, text: &str) -> Result<(), FrontmatterReason> {
    if path.is_index() || path.is_log() {
        let yaml = match split(text) {
            Err(FrontmatterReason::Missing) => return Ok(()),
            other => other?.0,
        };
        let entries = parse_block(yaml)?;
        let allowed = path.is_root_index();
        return match entries.into_iter().find(|(k, _)| !allowed || !ROOT_INDEX_KEYS.contains(&k.as_str())) {
            Some((key, _)) => Err(FrontmatterReason::NotAllowed { key }),
            None => Ok(()),
        };
    }
    let (yaml, _) = split(text)?;
    let entries = parse_block(yaml)?;
    let kind = entries.iter().find(|(k, _)| k == "type").and_then(|(_, v)| v.text());
    if kind.is_none_or(|t| t.trim().is_empty()) {
        return Err(FrontmatterReason::TypeMissing);
    }
    if let Some((_, generated)) = entries.iter().find(|(k, _)| k == "generated") {
        let bad = |key: &str| FrontmatterReason::BadTimestamp { key: key.to_owned() };
        let Node::Map(fields) = generated else { return Err(bad("generated")) };
        let at = fields.iter().find(|(k, _)| k == "at").and_then(|(_, v)| v.text());
        if at.is_none_or(|t| OffsetDateTime::parse(t, &Rfc3339).is_err()) {
            return Err(bad("generated.at"));
        }
    }
    Ok(())
}

/// Parses the first YAML document of `yaml` into its top-level entries.
fn parse_block(yaml: &str) -> Result<Vec<(String, Node)>, FrontmatterReason> {
    if yaml.len() > MAX_FRONTMATTER_BYTES {
        return Err(FrontmatterReason::TooLarge);
    }
    let mut stack: Vec<Open> = Vec::new();
    let mut root: Option<Node> = None;
    for event in Parser::new_from_str(yaml) {
        let (event, _) = event.map_err(|_| FrontmatterReason::NotYaml)?;
        let node = match event {
            Event::DocumentEnd | Event::StreamEnd => break,
            Event::Nothing | Event::StreamStart | Event::DocumentStart(_) => continue,
            Event::Alias(_) => return Err(FrontmatterReason::Alias),
            Event::Scalar(text, style, anchor, tag) => {
                refuse_anchor_and_tag(anchor, tag.is_some())?;
                Node::Scalar { text: text.into_owned(), plain: style == ScalarStyle::Plain }
            }
            Event::SequenceStart(anchor, tag) => {
                refuse_anchor_and_tag(anchor, tag.is_some())?;
                open(&mut stack, Open::Seq)?;
                continue;
            }
            Event::MappingStart(anchor, tag) => {
                refuse_anchor_and_tag(anchor, tag.is_some())?;
                open(&mut stack, Open::Map { entries: Vec::new(), seen: HashSet::new(), key: None })?;
                continue;
            }
            Event::SequenceEnd | Event::MappingEnd => match stack.pop() {
                Some(Open::Map { entries, .. }) => Node::Map(entries),
                _ => Node::Seq,
            },
        };
        match stack.last_mut() {
            None => root = Some(node),
            Some(Open::Seq) => {}
            Some(Open::Map { entries, seen, key }) => match key.take() {
                Some(k) => entries.push((k, node)),
                None => match node {
                    Node::Scalar { text, .. } => {
                        if !seen.insert(text.clone()) {
                            return Err(FrontmatterReason::DuplicateKey { key: text });
                        }
                        *key = Some(text);
                    }
                    _ => return Err(FrontmatterReason::NotMapping),
                },
            },
        }
    }
    match root {
        None => Ok(Vec::new()),
        Some(Node::Map(entries)) => Ok(entries),
        Some(_) => Err(FrontmatterReason::NotMapping),
    }
}

fn open(stack: &mut Vec<Open>, frame: Open) -> Result<(), FrontmatterReason> {
    stack.push(frame);
    if stack.len() > MAX_YAML_DEPTH { Err(FrontmatterReason::TooDeep) } else { Ok(()) }
}

fn refuse_anchor_and_tag(anchor: usize, tagged: bool) -> Result<(), FrontmatterReason> {
    if anchor != 0 {
        Err(FrontmatterReason::Alias)
    } else if tagged {
        Err(FrontmatterReason::Tag)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "frontmatter_tests.rs"]
mod frontmatter_tests;
