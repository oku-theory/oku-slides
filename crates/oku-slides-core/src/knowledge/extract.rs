//! The terms of a document import into the knowledge base: what kind of document it is, the
//! bounds of its extraction, what was cut, and why a document is refused.
//!
//! The extractor (`oku-slides-extract`) runs in a worker process; these types cross the wire.

use serde::{Deserialize, Serialize};

/// The kind of a document, chosen from its name and checked against its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// A PDF with a text layer.
    Pdf,
    /// A Word document (`.docx`).
    Docx,
    /// A PowerPoint presentation (`.pptx`).
    Pptx,
    /// An Excel workbook (`.xlsx`).
    Xlsx,
    /// An `OpenDocument` spreadsheet (`.ods`).
    Ods,
    /// Markdown (`.md`).
    Markdown,
    /// Plain text (`.txt`).
    Text,
}

impl DocumentType {
    /// The short name of the kind, as people know it (`PDF`, `DOCX`…).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            DocumentType::Pdf => "PDF",
            DocumentType::Docx => "DOCX",
            DocumentType::Pptx => "PPTX",
            DocumentType::Xlsx => "XLSX",
            DocumentType::Ods => "ODS",
            DocumentType::Markdown => "Markdown",
            DocumentType::Text => "text",
        }
    }
}

/// Why a document cannot be imported: said by its Fluent key ([`Self::key`]), in English by
/// `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractRefusal {
    /// A PDF without any text (a scan).
    NoTextLayer,
    /// A pre-2007 Office binary format.
    Legacy,
    /// A password-protected document.
    Encrypted,
    /// A kind the extractor does not read.
    Unsupported,
    /// The bytes do not parse as their kind.
    Damaged,
    /// The content is another kind than its name says.
    TypeMismatch,
    /// A read limit was passed.
    TooLarge,
}

impl ExtractRefusal {
    /// The Fluent key of the refusal (`core.ftl`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            ExtractRefusal::NoTextLayer => "knowledge-no-text-layer",
            ExtractRefusal::Legacy => "knowledge-legacy-format",
            ExtractRefusal::Encrypted => "knowledge-document-encrypted",
            ExtractRefusal::Unsupported => "knowledge-unsupported-document",
            ExtractRefusal::Damaged => "knowledge-document-damaged",
            ExtractRefusal::TypeMismatch => "knowledge-document-type-mismatch",
            ExtractRefusal::TooLarge => "knowledge-document-too-large",
        }
    }
}

impl std::fmt::Display for ExtractRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ExtractRefusal::NoTextLayer => {
                "this PDF has no text layer (a scan): export it with text, or paste its text"
            }
            ExtractRefusal::Legacy => "this older Office format cannot be read: save it as .pptx, .docx or .xlsx",
            ExtractRefusal::Encrypted => "this document is protected by a password: save an unprotected copy",
            ExtractRefusal::Unsupported => {
                "this kind of document cannot be imported: PDF, DOCX, PPTX, XLSX, ODS, Markdown or text"
            }
            ExtractRefusal::Damaged => "this document is damaged and cannot be read",
            ExtractRefusal::TypeMismatch => "this file's content does not match its name",
            ExtractRefusal::TooLarge => "this document is too large to import",
        })
    }
}

/// The bounds of one extraction, set by the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractLimits {
    /// Most bytes of Markdown produced: the room left in the bundle.
    pub max_text_bytes: usize,
    /// Most rows kept per sheet (`[knowledge] sheet_rows`).
    pub sheet_rows: usize,
    /// Most columns kept per sheet (`[knowledge] sheet_columns`).
    pub sheet_columns: usize,
    /// Most images kept.
    pub max_images: usize,
    /// Most pixels of one raster image, read from its header.
    pub max_image_pixels: u64,
    /// Most bytes of all images kept.
    pub max_image_bytes: u64,
}

/// What a cut dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CutKind {
    /// Sheet rows past `sheet_rows`.
    Rows,
    /// Sheet columns past `sheet_columns`.
    Columns,
    /// Whole sheets not read once the document's text budget was spent.
    Sheets,
    /// PDF pages past the page limit.
    Pages,
    /// Bytes of text past `max_text_bytes`.
    Text,
    /// Images past a limit.
    Images,
    /// Content nested deeper than the walk goes.
    Depth,
}

impl CutKind {
    /// Every kind, once: an extraction states at most this many cuts.
    pub const ALL: [Self; 7] =
        [Self::Rows, Self::Columns, Self::Sheets, Self::Pages, Self::Text, Self::Images, Self::Depth];
}

/// Content left out of an extraction, also stated in its Markdown where it applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cut {
    /// What was dropped.
    pub what: CutKind,
    /// How many of them (rows, columns, sheets, pages, bytes, images or nested elements).
    pub count: u64,
}

/// Media types an image is imported with: the rasters whose size their header gives.
pub const RASTERS: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Most characters of a document's own title kept in [`Extracted::title`].
pub const MAX_TITLE_CHARS: usize = 1024;

/// A document turned into Markdown, with its images and what was cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    /// The document's own title, when its format has one: raw and untrusted, only capped at
    /// [`MAX_TITLE_CHARS`] characters; it must go through `label_within` before any use.
    pub title: Option<String>,
    /// The sanitized Markdown body, at most `max_text_bytes` long.
    pub markdown: String,
    /// The images the Markdown cites, in order.
    pub images: Vec<ExtractedImage>,
    /// What was left out, one entry per kind; each is also stated in the Markdown.
    pub cuts: Vec<Cut>,
}

/// One image of an extracted document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImage {
    /// Its media type, among [`RASTERS`], read from its bytes.
    pub mime: String,
    /// Its bytes.
    pub bytes: Vec<u8>,
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod extract_tests;
