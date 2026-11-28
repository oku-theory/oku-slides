//! What the user reads of the deck knowledge base in the knowledge tab: tree, page, image.
//!
//! User-only reads, like the review (`knowledge_review.rs`): the agent window and the agent
//! never send them. An awaiting file shows its path and its raw text, never a parsed header or
//! an image: it is untrusted until the user approves it. Every header string shown is cleaned
//! (`label`); a path is sent whole, as `KnowledgePath` already keeps it to `[a-z0-9._-/]`, since
//! the tab resolves links against it. The body stays raw text, rendered as inert markdown.

use std::collections::BTreeSet;

use base64::Engine;
use serde::Serialize;
use snafu::{OptionExt, ensure};

use oku_slides_core::knowledge::limits::MAX_FILE_BYTES;
use oku_slides_core::{
    FrontmatterReason, KnowledgeFile, KnowledgeHeader, KnowledgeSource, body, cited_images, knowledge_header, media,
};

use crate::agent::PASTE_TYPES;
use crate::agent::bounds::{has_visible_text, label_within};
use crate::error::{KnowledgeAssetUnavailableSnafu, KnowledgeIdNotFoundSnafu};
use crate::knowledge_view::cut;
use crate::views::KnowledgeRevision;
use crate::{Document, Result, Session};

/// Largest image the tab loads, in bytes.
pub(crate) const MAX_KNOWLEDGE_ASSET_BYTES: usize = 8 * 1024 * 1024;
/// Most characters of a listed title.
const TITLE_MAX: usize = 80;
/// Most characters of a listed kind.
const KIND_MAX: usize = 40;
/// Most characters of a header value.
const VALUE_MAX: usize = 200;

/// The knowledge files, by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeTree {
    /// Every file of the deck.
    pub files: Vec<KnowledgeNode>,
}

/// One file of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeNode {
    /// The file's id.
    pub id: String,
    /// Its path, whole: the tab's key for links and folders.
    pub path: String,
    /// Its frontmatter `title`; `None` while awaiting or when it has none.
    pub title: Option<String>,
    /// Its frontmatter `type`; `None` while awaiting or when it has none.
    pub kind: Option<String>,
    /// True while the user has not approved this text.
    pub awaiting: bool,
}

/// One file as the tab shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgePage {
    /// The file's id.
    pub id: String,
    /// Its path, whole: relative links and images resolve against it.
    pub path: String,
    /// True while the user has not approved this text.
    pub awaiting: bool,
    /// The parsed header; `None` while awaiting or when the frontmatter does not parse.
    pub header: Option<KnowledgeHeaderView>,
    /// True when the approved text opens a frontmatter that does not parse: shown raw, with a notice.
    pub header_invalid: bool,
    /// The text after the header: the whole text when there is no header to show.
    pub body: String,
    /// Bytes of the file left out past the read ceiling.
    pub truncated_bytes: usize,
    /// The images the body cites, none while awaiting.
    pub images: Vec<KnowledgePageImage>,
}

/// An image a page cites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgePageImage {
    /// The bundle path the body cites.
    pub path: String,
    /// The asset id (`sha256:<hex>`).
    pub asset: String,
}

/// A header as displayed: every string cleaned and short.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeHeaderView {
    /// The `type` value.
    pub kind: Option<String>,
    /// The `title` value.
    pub title: Option<String>,
    /// The `description` value.
    pub description: Option<String>,
    /// The `status` value.
    pub status: Option<String>,
    /// True when the file is marked stale.
    pub stale: bool,
    /// The tags.
    pub tags: Vec<String>,
    /// Who generated the file.
    pub generated_by: Option<String>,
    /// When the file was generated, as written.
    pub generated_at: Option<String>,
    /// The verification tier.
    pub verified: Option<String>,
    /// The cited sources.
    pub sources: Vec<KnowledgeSourceView>,
}

/// A cited source as displayed; the url is text, never a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeSourceView {
    /// The `resource` value.
    pub resource: Option<String>,
    /// The `title` value.
    pub title: Option<String>,
    /// The `url` value.
    pub url: Option<String>,
}

/// An image's bytes, base64, decoded by the tab into a `blob:` image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeAsset {
    /// The sniffed type: png, jpeg, gif or webp.
    pub mime: &'static str,
    /// The bytes, base64.
    pub data: String,
}

/// A header value, cleaned and cut; nothing when it has no visible character.
fn shown(value: Option<&str>) -> Option<String> {
    shown_within(value, VALUE_MAX)
}

/// `value` cleaned and cut at `max` characters; nothing when it has no visible character.
fn shown_within(value: Option<&str>, max: usize) -> Option<String> {
    value.filter(|v| has_visible_text(v)).map(|v| label_within(v, max))
}

impl From<KnowledgeSource> for KnowledgeSourceView {
    fn from(s: KnowledgeSource) -> Self {
        Self { resource: shown(s.resource.as_deref()), title: shown(s.title.as_deref()), url: shown(s.url.as_deref()) }
    }
}

impl From<KnowledgeHeader> for KnowledgeHeaderView {
    fn from(h: KnowledgeHeader) -> Self {
        Self {
            kind: shown(h.kind.as_deref()),
            title: shown(h.title.as_deref()),
            description: shown(h.description.as_deref()),
            status: shown(h.status.as_deref()),
            stale: h.stale,
            tags: h.tags.iter().filter_map(|t| shown_within(Some(t), VALUE_MAX)).collect(),
            generated_by: shown(h.generated_by.as_deref()),
            generated_at: shown(h.generated_at.as_deref()),
            verified: shown(h.verified.as_deref()),
            sources: h.sources.into_iter().map(Into::into).collect(),
        }
    }
}

impl Session {
    /// Every knowledge file by path, with whether it awaits approval.
    ///
    /// # Errors
    ///
    /// `InHistory` while history is open, `WrongDocument` outside a deck tab.
    pub(crate) fn knowledge_tree(&self) -> Result<KnowledgeTree> {
        self.ensure_knowledge_review()?;
        let scan = self.knowledge_scan();
        let awaiting: BTreeSet<&str> = scan.awaiting.iter().map(|a| a.id.as_str()).collect();
        let mut files: Vec<(&String, &KnowledgeFile)> = self.doc.sources.deck().knowledge.iter().collect();
        files.sort_by(|a, b| a.1.path.cmp(&b.1.path));
        let files = files
            .into_iter()
            .map(|(id, f)| {
                let awaits = awaiting.contains(id.as_str());
                let header = if awaits { None } else { knowledge_header(&f.text).ok() };
                KnowledgeNode {
                    id: id.clone(),
                    path: f.path.clone(),
                    title: shown_within(header.as_ref().and_then(|h| h.title.as_deref()), TITLE_MAX),
                    kind: shown_within(header.as_ref().and_then(|h| h.kind.as_deref()), KIND_MAX),
                    awaiting: awaits,
                }
            })
            .collect();
        Ok(KnowledgeTree { files })
    }

    /// The file `id` as the tab shows it, cut at `MAX_FILE_BYTES`.
    ///
    /// # Errors
    ///
    /// `InHistory`, `WrongDocument`, or `KnowledgeIdNotFound` when no file has that id.
    pub(crate) fn knowledge_page(&self, id: &str) -> Result<KnowledgePage> {
        self.ensure_knowledge_review()?;
        let file = self.doc.sources.deck().knowledge.get(id).context(KnowledgeIdNotFoundSnafu { id })?;
        let text = cut(&file.text, MAX_FILE_BYTES);
        let awaiting = !self.knowledge_approved(file);
        let parsed = if awaiting { Err(FrontmatterReason::Missing) } else { knowledge_header(text) };
        let header_invalid = matches!(&parsed, Err(reason) if *reason != FrontmatterReason::Missing);
        let header = parsed.ok();
        let shown_body = if header.is_some() { body(text) } else { text };
        let images = if awaiting {
            Vec::new()
        } else {
            cited_images(&file.path, text).into_iter().map(|(asset, path)| KnowledgePageImage { path, asset }).collect()
        };
        Ok(KnowledgePage {
            id: id.to_owned(),
            path: file.path.clone(),
            awaiting,
            header: header.map(Into::into),
            header_invalid,
            body: shown_body.to_owned(),
            truncated_bytes: file.text.len() - text.len(),
            images,
        })
    }

    /// The bytes of image `asset`, cited by the approved file `id`.
    ///
    /// # Errors
    ///
    /// `InHistory`, `WrongDocument`, or `KnowledgeAssetUnavailable` when the file is unknown,
    /// awaiting, does not cite the asset, the deck lacks it, it is over 8 MiB or not a png,
    /// jpeg, gif or webp.
    pub(crate) fn knowledge_asset(&self, id: &str, asset: &str) -> Result<KnowledgeAsset> {
        self.ensure_knowledge_review()?;
        ensure!(is_asset_id(asset), KnowledgeAssetUnavailableSnafu { asset });
        let file = self.doc.sources.deck().knowledge.get(id).context(KnowledgeAssetUnavailableSnafu { asset })?;
        ensure!(self.knowledge_approved(file), KnowledgeAssetUnavailableSnafu { asset });
        let text = cut(&file.text, MAX_FILE_BYTES);
        ensure!(cited_images(&file.path, text).contains_key(asset), KnowledgeAssetUnavailableSnafu { asset });
        let bytes = self.doc.sources.assets.get(asset).context(KnowledgeAssetUnavailableSnafu { asset })?;
        let mime = media::sniff(bytes);
        ensure!(
            // The types Typst draws, as a paste takes them: never an SVG, which may carry script.
            bytes.len() <= MAX_KNOWLEDGE_ASSET_BYTES && PASTE_TYPES.contains(&mime),
            KnowledgeAssetUnavailableSnafu { asset }
        );
        Ok(KnowledgeAsset { mime, data: base64::engine::general_purpose::STANDARD.encode(bytes) })
    }

    /// The live revision of `deck`'s knowledge, for the tab bar.
    pub(crate) fn knowledge_revision(&self, deck: &Document) -> KnowledgeRevision {
        KnowledgeRevision {
            generation: deck.sources.knowledge_generation(),
            approvals: self.library.knowledge_approvals().count(),
            in_history: matches!(self.mode, crate::Mode::History { .. }),
        }
    }
}

/// Whether `asset` reads `sha256:` then 64 lowercase hex digits.
fn is_asset_id(asset: &str) -> bool {
    asset
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

#[cfg(test)]
#[path = "knowledge_read_tests.rs"]
mod knowledge_read_tests;
