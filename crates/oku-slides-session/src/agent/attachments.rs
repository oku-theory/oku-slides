//! What is attached to the agent's draft, kept per tab in memory: pasted images and, in a deck
//! tab, documents.
//!
//! An attachment is checked when attached, by its bytes' magic only (an image's header gives its
//! pixels; a document is never parsed in this process), and kept under its asset id
//! `sha256:…`. An image enters the deck only when an op cites it
//! (`Session::embed_op_assets`, which also takes the assets of a paste between tabs); a document
//! enters it only through its import into the knowledge base.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use oku_slides_core::knowledge::DocumentType;
use oku_slides_core::media::{SVG, SniffRefusal, dimensions, sniff, sniff_document};
use oku_slides_core::{Deck, WidgetRegistry};
use serde::{Deserialize, Serialize};

use crate::error::{PasteSnafu, UnknownAssetSnafu};
use crate::{Mode, Result, Session};

/// The image types a paste accepts, those Typst draws as is; an agent is shown only these.
pub const PASTE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// The longest attached document name kept, in characters.
const NAME_MAX: usize = 255;

/// An attachment of the draft, as the views and the agent host know it (never persisted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AttachmentRef {
    /// A pasted image.
    Image {
        /// `sha256:<hex>` of its bytes, the same as its asset once placed.
        id: String,
        /// Its actual type, read from its bytes.
        mime: String,
        /// Its width in pixels.
        width: u32,
        /// Its height in pixels.
        height: u32,
    },
    /// A document attached in a deck tab, for the knowledge base.
    Document {
        /// `sha256:<hex>` of its bytes.
        id: String,
        /// Its file name, last path segment only, cleaned for display.
        name: String,
        /// Its kind, read from its magic bytes and its name's extension.
        kind: DocumentType,
        /// Its size in bytes.
        bytes: u64,
    },
}

impl AttachmentRef {
    /// Its id, `sha256:<hex>` of its bytes.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Image { id, .. } | Self::Document { id, .. } => id,
        }
    }

    /// A document's file name; an image has none.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Image { .. } => None,
            Self::Document { name, .. } => Some(name),
        }
    }

    /// What the thread's bubble shows of an image; a document is not an image.
    #[must_use]
    pub fn shown(&self) -> Option<oku_slides_a2ui::PromptImage> {
        match self {
            Self::Image { mime, width, height, .. } => {
                Some(oku_slides_a2ui::PromptImage { mime: mime.clone(), width: *width, height: *height })
            }
            Self::Document { .. } => None,
        }
    }
}

/// Why an attachment is refused (error [`crate::SessionErrorKind::Paste`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PasteReason {
    /// Neither a PNG, JPEG, GIF or WebP image nor, by its magic, a document that can be imported.
    Unsupported,
    /// An image past `[paste] max_image_mib`.
    TooLarge,
    /// An image past `[paste] max_image_pixels`.
    TooManyPixels,
    /// An image whose header cannot be read.
    Unreadable,
    /// A document past `[paste] max_document_mib`.
    DocumentTooLarge,
    /// A pre-2007 Office binary file, or an encrypted OOXML one.
    Legacy,
    /// A document whose content is another kind than its name says.
    TypeMismatch,
    /// A document attached outside a deck tab.
    DocumentDeckOnly,
    /// The draft's attachments with this one would pass `[paste] max_kept_mib`.
    KeptFull,
    /// A document whose cleaned name is empty.
    NameEmpty,
}

impl PasteReason {
    /// The Fluent key of the refusal.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Unsupported => "session-paste-unsupported",
            Self::TooLarge => "session-paste-too-large",
            Self::TooManyPixels => "session-paste-too-many-pixels",
            Self::Unreadable => "session-paste-unreadable",
            Self::DocumentTooLarge => "session-attach-document-too-large",
            Self::Legacy => "session-attach-legacy",
            Self::TypeMismatch => "session-attach-type-mismatch",
            Self::DocumentDeckOnly => "session-attach-document-deck-only",
            Self::KeptFull => "session-attach-kept-full",
            Self::NameEmpty => "session-attach-name-empty",
        }
    }

    /// The refusal of a document [`sniff_document`] refused.
    fn of_document(refusal: SniffRefusal) -> Self {
        match refusal {
            SniffRefusal::Legacy => Self::Legacy,
            SniffRefusal::TypeMismatch => Self::TypeMismatch,
            SniffRefusal::Unsupported => Self::Unsupported,
        }
    }
}

/// The attachment bounds the interface checks before reading a file (`Snapshot.attach_limits`).
///
/// The host still checks every attach itself; these only spare reading and sending a file it
/// would refuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AttachLimits {
    /// Largest image, in bytes (`[paste] max_image_mib`).
    pub image_bytes: usize,
    /// Largest document, in bytes (`[paste] max_document_mib`, within its ceiling).
    pub document_bytes: usize,
    /// Images at most in one draft (`[paste] max_draft_images`).
    pub draft_images: usize,
    /// Documents at most in one draft (`[paste] max_draft_documents`).
    pub draft_documents: usize,
}

/// Bytes to attach, hashed and sniffed once: the agent host counts them toward the draft's
/// caps, then the session checks and keeps them, without reading them again.
#[derive(Debug)]
pub struct Upload {
    bytes: Vec<u8>,
    name: String,
    id: String,
    sniffed: Sniffed,
}

/// What the magic of an [`Upload`] says it is.
#[derive(Debug)]
enum Sniffed {
    /// One of [`PASTE_TYPES`].
    Image(&'static str),
    /// An SVG: refused as an image Typst does not draw as is, never read as text.
    Svg,
    /// Anything else, as [`sniff_document`] reads it with the name.
    Document(std::result::Result<DocumentType, SniffRefusal>),
}

impl Upload {
    /// Hashes and sniffs `bytes`, named `name` (a document's file name, else empty).
    #[must_use]
    pub fn new(bytes: Vec<u8>, name: &str) -> Self {
        let id = oku_slides_core::asset_id(&bytes);
        let mime = sniff(&bytes);
        let sniffed = if PASTE_TYPES.contains(&mime) {
            Sniffed::Image(mime)
        } else if mime == SVG {
            Sniffed::Svg
        } else {
            Sniffed::Document(sniff_document(&bytes, name))
        };
        Self { bytes, name: name.to_owned(), id, sniffed }
    }

    /// Its asset id, `sha256:…`.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Whether its magic is one of [`PASTE_TYPES`].
    #[must_use]
    pub fn is_image(&self) -> bool {
        matches!(self.sniffed, Sniffed::Image(_))
    }

    /// Whether [`sniff_document`] accepts it as a document.
    #[must_use]
    pub fn is_document(&self) -> bool {
        matches!(self.sniffed, Sniffed::Document(Ok(_)))
    }
}

/// The attachments of a tab, from the oldest to the newest.
#[derive(Debug)]
pub(crate) struct Attachments {
    limits: oku_slides_platform::config::PasteSettings,
    items: VecDeque<(AttachmentRef, Arc<[u8]>)>,
    bytes: usize,
}

impl Attachments {
    pub(crate) fn new(limits: oku_slides_platform::config::PasteSettings) -> Self {
        Self { limits, items: VecDeque::new(), bytes: 0 }
    }

    /// The `[paste]` bounds of this tab's attachments.
    pub(crate) fn limits(&self) -> &oku_slides_platform::config::PasteSettings {
        &self.limits
    }

    /// The bounds the interface checks before sending a file.
    pub(crate) fn ui_limits(&self) -> AttachLimits {
        AttachLimits {
            image_bytes: self.limits.max_image_bytes(),
            document_bytes: self.limits.max_document_bytes(),
            draft_images: self.limits.max_draft_images,
            draft_documents: self.limits.max_draft_documents.get(),
        }
    }

    /// Checks and keeps `upload`; `deck`: the tab is a deck tab.
    ///
    /// Past `max_kept_mib`, forgets the oldest outside `keep` (the draft's ids); refuses one the
    /// draft alone would push past it. Attached again, under another name or none, an item keeps
    /// its first reference: the bytes are its identity, and its first name the one shown and
    /// imported under.
    fn add(&mut self, upload: Upload, deck: bool, keep: &[String]) -> std::result::Result<AttachmentRef, PasteReason> {
        let r = self.checked(&upload, deck)?;
        let bytes = upload.bytes;
        // Already there: it becomes the newest.
        if let Some(i) = self.items.iter().position(|(kept, _)| kept.id() == r.id())
            && let Some(item) = self.items.remove(i)
        {
            let kept = item.0.clone();
            self.items.push_back(item);
            return Ok(kept);
        }
        let draft: usize =
            self.items.iter().filter(|(kept, _)| keep.iter().any(|k| k == kept.id())).map(|(_, b)| b.len()).sum();
        if draft.saturating_add(bytes.len()) > self.limits.max_kept_bytes() {
            return Err(PasteReason::KeptFull);
        }
        self.bytes += bytes.len();
        self.items.push_back((r.clone(), bytes.into()));
        self.evict(keep, r.id());
        Ok(r)
    }

    /// The reference of `upload`, once checked as an image or a document.
    fn checked(&self, upload: &Upload, deck: bool) -> std::result::Result<AttachmentRef, PasteReason> {
        let (bytes, name, id) = (upload.bytes.as_slice(), upload.name.as_str(), upload.id.clone());
        let kind = match upload.sniffed {
            Sniffed::Image(mime) => {
                if bytes.len() > self.limits.max_image_bytes() {
                    return Err(PasteReason::TooLarge);
                }
                let (width, height) = dimensions(bytes, mime).ok_or(PasteReason::Unreadable)?;
                if u64::from(width) * u64::from(height) > self.limits.max_image_pixels {
                    return Err(PasteReason::TooManyPixels);
                }
                return Ok(AttachmentRef::Image { id, mime: mime.to_owned(), width, height });
            }
            // An SVG is UTF-8 text: never read as a text document either.
            Sniffed::Svg => return Err(PasteReason::Unsupported),
            Sniffed::Document(kind) => kind.map_err(PasteReason::of_document)?,
        };
        if !deck {
            return Err(PasteReason::DocumentDeckOnly);
        }
        if bytes.len() > self.limits.max_document_bytes() {
            return Err(PasteReason::DocumentTooLarge);
        }
        let last = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if !super::bounds::has_visible_text(last) {
            return Err(PasteReason::NameEmpty);
        }
        let name = super::bounds::label_within(last, NAME_MAX);
        Ok(AttachmentRef::Document { id, name, kind, bytes: oku_slides_core::num::count_u64(bytes.len()) })
    }

    /// Forgets the oldest, outside `keep` and `fresh`, while the total passes the limit.
    fn evict(&mut self, keep: &[String], fresh: &str) {
        while self.bytes > self.limits.max_kept_bytes() {
            let Some(i) = self.items.iter().position(|(r, _)| r.id() != fresh && !keep.iter().any(|k| k == r.id()))
            else {
                return;
            };
            if let Some((_, b)) = self.items.remove(i) {
                self.bytes -= b.len();
            }
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<(AttachmentRef, Arc<[u8]>)> {
        self.items.iter().find(|(r, _)| r.id() == id).map(|(r, b)| (r.clone(), b.clone()))
    }

    /// The bytes of the image `id`; `None` for a document or an id not kept.
    pub(crate) fn image(&self, id: &str) -> Option<Arc<[u8]>> {
        self.items
            .iter()
            .find(|(r, _)| r.id() == id && matches!(r, AttachmentRef::Image { .. }))
            .map(|(_, b)| b.clone())
    }
}

impl Session {
    /// Checks an attachment and keeps it for the tab; `keep`: the draft's ids.
    ///
    /// # Errors
    ///
    /// Fails (`Paste`) if it is refused: kind, size, a document outside a deck tab, its name, or
    /// the draft's attachments too heavy together.
    pub fn attach(&mut self, upload: Upload, keep: &[String]) -> Result<AttachmentRef> {
        let deck = matches!(self.mode, Mode::Deck);
        self.attachments.add(upload, deck, keep).map_err(|reason| PasteSnafu { reason }.build().into())
    }

    /// An attachment the tab keeps, and its bytes.
    #[must_use]
    pub fn attached(&self, id: &str) -> Option<(AttachmentRef, Arc<[u8]>)> {
        self.attachments.get(id)
    }

    /// Adds to the bundle the attached images that `next` cites and the deck did not.
    ///
    /// The assets brought by an element pasted from another tab (`carried_assets`) enter the
    /// same way. Another newly cited asset, absent from the bundle, is refused (`UnknownAsset`):
    /// an attached document is not an asset. An asset already missing before the op (an imported
    /// deck) is left to `validate`. The images that knowledge files newly cite
    /// (`knowledge_asset_refs`, new minus before) enter too when attached, staged by an import
    /// (`knowledge_import`) or carried; one that is
    /// neither stays missing, shown so by the knowledge views. New assets past the sources budget
    /// are refused (`SourcesOverBudget`), none added.
    pub(crate) fn embed_op_assets(&mut self, next: &Deck, registry: &WidgetRegistry) -> Result<()> {
        let deck = self.doc.sources.deck();
        let before = oku_slides_core::asset_refs(deck, registry);
        let before_knowledge = oku_slides_core::knowledge_asset_refs(deck);
        let (mut pasted, mut carried, mut staged) = (BTreeMap::new(), BTreeSet::new(), BTreeSet::new());
        for id in oku_slides_core::asset_refs(next, registry) {
            if before.contains(&id) || self.doc.sources.assets.contains_key(&id) {
                continue;
            }
            if let Some(bytes) = self.attachments.image(&id) {
                pasted.insert(id, bytes);
            } else if self.carried.contains_key(&id) {
                carried.insert(id);
            } else {
                return UnknownAssetSnafu { asset: id }.fail()?;
            }
        }
        for id in oku_slides_core::knowledge_asset_refs(next) {
            if before_knowledge.contains(&id) || self.doc.sources.assets.contains_key(&id) {
                continue;
            }
            if let Some(bytes) = self.attachments.image(&id) {
                pasted.insert(id, bytes);
            } else if self.import_staged.contains_key(&id) {
                staged.insert(id);
            } else if self.carried.contains_key(&id) {
                carried.insert(id);
            }
        }
        // Everything is checked before anything is added: a refusal adds nothing.
        let sizes = pasted
            .values()
            .map(|b| b.len())
            .chain(carried.iter().filter_map(|id| self.carried.get(id)).map(Vec::len))
            .chain(staged.iter().filter_map(|id| self.import_staged.get(id)).map(Vec::len));
        self.ensure_sources_room(sizes.map(oku_slides_core::num::count_u64).fold(0, u64::saturating_add))?;
        for bytes in pasted.into_values() {
            self.doc.sources.add_asset(bytes.to_vec());
        }
        // A carried or staged asset goes into the bundle without a copy: it is no longer kept
        // aside. `remove` always finds it: `contains_key` just saw it, under the same `&mut self`.
        for id in carried {
            if let Some(bytes) = self.carried.remove(&id) {
                self.doc.sources.add_asset(bytes);
            }
        }
        for id in staged {
            if let Some(bytes) = self.import_staged.remove(&id) {
                self.doc.sources.add_asset(bytes);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod attachments_tests;
