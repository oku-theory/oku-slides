//! The real type of a medium, read from its first bytes, never inferred from its extension.
//!
//! The `.pptx` import and an image pasted into the agent draft use it; [`sniff_document`]
//! recognizes a document attached to a deck chat by its magic bytes.

use crate::knowledge::DocumentType;

/// The type of unrecognized content: projectors report it as an unsupported image.
pub const OCTET_STREAM: &str = "application/octet-stream";
/// The type of an SVG, recognized by its `<svg` root (prefixed or not) after an optional XML prolog.
pub const SVG: &str = "image/svg+xml";

/// Signatures: offset, expected bytes, type.
const MAGIC: [(usize, &[u8], &str); 13] = [
    (0, b"\x89PNG\r\n\x1a\n", "image/png"),
    (0, b"\xFF\xD8\xFF", "image/jpeg"),
    (0, b"GIF87a", "image/gif"),
    (0, b"GIF89a", "image/gif"),
    (8, b"WEBP", "image/webp"),
    (0, b"BM", "image/bmp"),
    (0, b"II*\0", "image/tiff"),
    (0, b"MM\0*", "image/tiff"),
    (0, b"II\xBC", "image/vnd.ms-photo"),
    (40, b" EMF", "image/emf"),
    (0, b"\xD7\xCD\xC6\x9A", "image/wmf"),
    (0, b"\x01\0\x09\0", "image/wmf"),
    (0, b"\x02\0\x09\0", "image/wmf"),
];

/// Whether `bytes` hold `sig` at offset `off`.
fn at(bytes: &[u8], off: usize, sig: &[u8]) -> bool {
    bytes.get(off..off + sig.len()) == Some(sig)
}

/// The MIME type of `bytes` from their content; the only source of an imported asset's type.
#[must_use]
pub fn sniff(bytes: &[u8]) -> &'static str {
    let at = |off: usize, sig: &[u8]| at(bytes, off, sig);
    let found = MAGIC.iter().find(|(off, sig, mime)| {
        at(*off, sig)
            && match *mime {
                "image/webp" => at(0, b"RIFF"),
                "image/emf" => at(0, b"\x01\0\0\0"),
                _ => true,
            }
    });
    match found {
        Some((_, _, mime)) => mime,
        None if is_svg(bytes) => SVG,
        None => OCTET_STREAM,
    }
}

/// The extensions whose kind is binary: magic bytes that disagree make a `TypeMismatch`.
const BINARY_EXTENSIONS: [&str; 8] = ["pdf", "docx", "pptx", "xlsx", "ods", "doc", "ppt", "xls"];
/// The extensions whose kind is text: a binary magic under one of them makes a `TypeMismatch`.
const TEXT_EXTENSIONS: [&str; 3] = ["md", "markdown", "txt"];
/// The image extensions, [`sniff`]'s kinds and the usual others: text or a document under one of
/// them makes a `TypeMismatch`, never a text document.
const IMAGE_EXTENSIONS: [&str; 16] = [
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "jxr", "wdp", "emf", "wmf", "svg", "heic", "heif",
    "avif",
];

/// Why [`sniff_document`] refuses bytes, read from their magic and their name alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SniffRefusal {
    /// An OLE2 file: a pre-2007 Office binary (`.doc`, `.ppt`, `.xls`) or an encrypted OOXML one.
    Legacy,
    /// A known extension whose magic disagrees.
    TypeMismatch,
    /// An empty file, a zip of another kind, or neither a known magic nor UTF-8 text.
    Unsupported,
}

/// The kind of document `bytes` hold, from their magic bytes, the name's extension choosing among
/// zip subtypes and between Markdown and text.
///
/// Reads the bytes for their magic only (and for UTF-8 validity): nothing is parsed, no zip is
/// opened; the extraction worker checks the zip subtype again. `name` may be a path: only its
/// last segment counts. A PDF may open with a preamble: `%PDF-` at a line start in the first KiB
/// counts under a `.pdf` name, or for binary bytes under no known extension, never in text.
///
/// # Errors
///
/// - [`SniffRefusal::Legacy`]: an OLE2 file (`.doc`, `.ppt`, `.xls`, encrypted OOXML).
/// - [`SniffRefusal::TypeMismatch`]: a known extension whose magic disagrees (a `.pdf` that
///   starts with `PK`, a `.md` that starts with `%PDF-`, a `.docx` or a `.png` holding text).
/// - [`SniffRefusal::Unsupported`]: an empty file, an image under an image name, a zip of
///   another kind, or bytes that are neither a known magic nor UTF-8 text without NUL.
pub fn sniff_document(bytes: &[u8], name: &str) -> Result<DocumentType, SniffRefusal> {
    let last = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let ext = last.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
    let image = IMAGE_EXTENSIONS.contains(&ext.as_str());
    let binary = image || BINARY_EXTENSIONS.contains(&ext.as_str());
    let known = binary || TEXT_EXTENSIONS.contains(&ext.as_str());
    // An image under an image name is no document, but its name does not lie: not a mismatch.
    if bytes.is_empty() || (image && sniff(bytes) != OCTET_STREAM) {
        return Err(SniffRefusal::Unsupported);
    }
    let text = !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok();
    if at(bytes, 0, b"%PDF-") || ((ext == "pdf" || (!known && !text)) && pdf_after_preamble(bytes)) {
        return if known && ext != "pdf" { Err(SniffRefusal::TypeMismatch) } else { Ok(DocumentType::Pdf) };
    }
    if at(bytes, 0, b"PK\x03\x04") {
        return match ext.as_str() {
            "docx" => Ok(DocumentType::Docx),
            "pptx" => Ok(DocumentType::Pptx),
            "xlsx" => Ok(DocumentType::Xlsx),
            "ods" => Ok(DocumentType::Ods),
            _ if known => Err(SniffRefusal::TypeMismatch),
            _ => Err(SniffRefusal::Unsupported),
        };
    }
    if at(bytes, 0, b"\xD0\xCF\x11\xE0") {
        return Err(SniffRefusal::Legacy);
    }
    if text {
        return match ext.as_str() {
            _ if binary => Err(SniffRefusal::TypeMismatch),
            "md" | "markdown" => Ok(DocumentType::Markdown),
            _ => Ok(DocumentType::Text),
        };
    }
    Err(if binary { SniffRefusal::TypeMismatch } else { SniffRefusal::Unsupported })
}

/// How far a PDF header may follow a preamble, as readers accept.
const PDF_PREAMBLE: usize = 1024;

/// Whether `bytes` hold `%PDF-` at a line start within their first [`PDF_PREAMBLE`] bytes.
fn pdf_after_preamble(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(PDF_PREAMBLE)];
    head.windows(6).any(|w| matches!(w[0], b'\n' | b'\r') && &w[1..] == b"%PDF-")
}

/// Width and height a raster Typst draws (png, jpeg, gif, webp) states in its header.
///
/// Read without decoding or allocating, to refuse a huge image before rendering decompresses it;
/// `None` when the header is truncated or inconsistent, or `mime` is none of them.
#[must_use]
pub fn dimensions(bytes: &[u8], mime: &str) -> Option<(u32, u32)> {
    let le16 = |at: usize| bytes.get(at..at + 2).map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])));
    let le24 = |at: usize| bytes.get(at..at + 3).map(|b| u32::from_le_bytes([b[0], b[1], b[2], 0]));
    let be32 = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    match mime {
        "image/png" => (bytes.get(12..16) == Some(b"IHDR")).then_some((be32(16)?, be32(20)?)),
        "image/gif" => Some((le16(6)?, le16(8)?)),
        "image/webp" => match bytes.get(12..16)? {
            b"VP8 " => (bytes.get(23..26) == Some(b"\x9d\x01\x2a")).then_some((le16(26)? & 0x3FFF, le16(28)? & 0x3FFF)),
            b"VP8L" => {
                let bits = u32::from_le_bytes(bytes.get(21..25)?.try_into().ok()?);
                (bytes.get(20) == Some(&0x2F)).then_some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
            }
            b"VP8X" => Some((le24(24)? + 1, le24(27)? + 1)),
            _ => None,
        },
        "image/jpeg" => jpeg_dimensions(bytes),
        _ => None,
    }
}

/// Dimensions in a JPEG's first `SOFn` segment, earlier segments skipped by their length.
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let be16 = |at: usize| bytes.get(at..at + 2).map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])));
    let mut at = 2;
    loop {
        // `FF` fill bytes are allowed before a marker.
        while bytes.get(at + 1) == Some(&0xFF) && bytes.get(at) == Some(&0xFF) {
            at += 1;
        }
        if bytes.get(at) != Some(&0xFF) {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            // Markers without a segment: TEM, RSTn.
            0x01 | 0xD0..=0xD7 => at += 2,
            // SOF0 to SOF15, except DHT (C4), JPG (C8) and DAC (CC).
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                let (h, w) = (be16(at + 5)?, be16(at + 7)?);
                return Some((u32::try_from(w).ok()?, u32::try_from(h).ok()?));
            }
            // Start of scan or end of image before any SOF.
            0xD9 | 0xDA => return None,
            _ => at = at.checked_add(2)?.checked_add(be16(at + 2)?)?,
        }
    }
}

/// The rest of `b` after the first occurrence of `end`.
fn after<'b>(b: &'b [u8], end: &[u8]) -> Option<&'b [u8]> {
    b.windows(end.len()).position(|w| w == end).and_then(|p| b.get(p + end.len()..))
}

/// The rest after a DOCTYPE (its internal `[…]` subset included).
fn after_doctype(b: &[u8]) -> Option<&[u8]> {
    let close = b.iter().position(|c| *c == b'>')?;
    // `[` is looked for before `>` only: each DOCTYPE reads only what it consumes (a linear read).
    match b.get(..close)?.iter().position(|c| *c == b'[') {
        Some(open) => after(b.get(open..)?, b"]").and_then(|r| after(r, b">")),
        None => b.get(close + 1..),
    }
}

/// An XML name without prefix: a letter or `_`, then letters, digits, `_`, `-`, `.`.
fn is_ncname(n: &[u8]) -> bool {
    n.first().is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && n.iter().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}

/// A `<svg` (or `<prefix:svg`) root after an optional BOM, declaration, comments, instructions and DOCTYPE.
fn is_svg(bytes: &[u8]) -> bool {
    let mut rest = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    loop {
        rest = rest.trim_ascii_start();
        let next = if let Some(r) = rest.strip_prefix(b"<?") {
            after(r, b"?>")
        } else if let Some(r) = rest.strip_prefix(b"<!--") {
            after(r, b"-->")
        } else if let Some(r) = rest.strip_prefix(b"<!DOCTYPE") {
            after_doctype(r)
        } else {
            break;
        };
        match next {
            Some(r) => rest = r,
            None => return false,
        }
    }
    let Some(tag) = rest.strip_prefix(b"<") else { return false };
    let end = tag.iter().position(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/')).unwrap_or(tag.len());
    let (name, delimited) = tag.split_at(end);
    // A prefix is allowed: `sanitize` then checks the SVG namespace by its URI.
    !delimited.is_empty() && (name == b"svg" || name.strip_suffix(b":svg").is_some_and(is_ncname))
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod media_tests;
