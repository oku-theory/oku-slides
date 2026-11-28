//! The frontmatter fields the knowledge tab shows, read leniently from the strict parser's nodes.

use super::frontmatter::{Node, parse_block, scalar_of, split, stale_of};
use crate::error::FrontmatterReason;

/// The frontmatter of a knowledge file, as the knowledge tab displays it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnowledgeHeader {
    /// The `type` value, when it is text.
    pub kind: Option<String>,
    /// The `title` value, when it is text.
    pub title: Option<String>,
    /// The `description` value, when it is text.
    pub description: Option<String>,
    /// The `status` value, when it is text.
    pub status: Option<String>,
    /// True only when `stale` is the plain scalar `true`.
    pub stale: bool,
    /// The scalar items of `tags`, or a lone scalar `tags`; at most [`MAX_HEADER_ITEMS`](super::limits::MAX_HEADER_ITEMS).
    pub tags: Vec<String>,
    /// `generated.by`, when it is text.
    pub generated_by: Option<String>,
    /// `generated.at` as written, not parsed.
    pub generated_at: Option<String>,
    /// A scalar `verified`, otherwise a scalar `verified.tier`.
    pub verified: Option<String>,
    /// The usable items of `sources`; at most [`MAX_HEADER_ITEMS`](super::limits::MAX_HEADER_ITEMS).
    pub sources: Vec<KnowledgeSource>,
}

/// One cited source of a knowledge file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnowledgeSource {
    /// The `resource` value, or the whole item when it is a scalar.
    pub resource: Option<String>,
    /// The `title` value.
    pub title: Option<String>,
    /// The `url` value; text only, never a link.
    pub url: Option<String>,
}

/// Reads the display header of a file, on the strict frontmatter parser.
///
/// # Errors
///
/// The structural [`FrontmatterReason`]s, as [`super::frontmatter`].
pub fn knowledge_header(text: &str) -> Result<KnowledgeHeader, FrontmatterReason> {
    let (yaml, _) = split(text)?;
    let entries = parse_block(yaml)?;
    let get = |name: &str| entries.iter().find(|(k, _)| k == name).map(|(_, v)| v);
    let scalar = |name: &str| scalar_of(&entries, name);
    let field = |name: &str, key: &str| match get(name) {
        Some(Node::Map(fields)) => field_text(fields, key),
        _ => None,
    };
    let tags = match get("tags") {
        Some(Node::Seq(items)) => items.iter().filter_map(|n| n.text().map(str::to_owned)).collect(),
        Some(node) => node.text().map(str::to_owned).into_iter().collect(),
        None => Vec::new(),
    };
    let sources = match get("sources") {
        Some(Node::Seq(items)) => items.iter().filter_map(source).collect(),
        Some(node) => source(node).into_iter().collect(),
        None => Vec::new(),
    };
    Ok(KnowledgeHeader {
        kind: scalar("type"),
        title: scalar("title"),
        description: scalar("description"),
        status: scalar("status"),
        stale: stale_of(&entries),
        tags,
        generated_by: field("generated", "by"),
        generated_at: field("generated", "at"),
        verified: scalar("verified").or_else(|| field("verified", "tier")),
        sources,
    })
}

fn field_text(fields: &[(String, Node)], key: &str) -> Option<String> {
    fields.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.text()).map(str::to_owned)
}

fn source(node: &Node) -> Option<KnowledgeSource> {
    let source = match node {
        Node::Map(fields) => KnowledgeSource {
            resource: field_text(fields, "resource"),
            title: field_text(fields, "title"),
            url: field_text(fields, "url"),
        },
        other => KnowledgeSource { resource: other.text().map(str::to_owned), ..KnowledgeSource::default() },
    };
    (source != KnowledgeSource::default()).then_some(source)
}

/// The text after a terminated frontmatter block, or the whole text when there is none or it is
/// unterminated. It never parses YAML.
#[must_use]
pub fn body(text: &str) -> &str {
    split(text).map_or(text, |(_, rest)| rest)
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
