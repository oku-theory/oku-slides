//! A zip archive read under [`ReadLimits`], and bounded XML: the generic part of an OOXML or
//! ODF reader, shared by the PPTX reader and the knowledge document extractor.

use std::cell::Cell;
use std::io::{Cursor, Read};

use oku_slides_core::num::count_u64;
use roxmltree::Document;

use crate::ReadLimits;
use crate::error::{ImportError, io_error, protected, too_large, utf8_error, zip_error};

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
pub(crate) const EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
pub(crate) const ZIP64_LOCATOR: [u8; 4] = [0x50, 0x4b, 0x06, 0x07];

/// A zip archive opened on its bytes; every read counts against the limits.
///
/// The archive is cloned on each read (metadata in an `Arc`, reader over `&[u8]`) rather than
/// kept in a `RefCell`: `BoundedZip<'b>` stays covariant in `'b`.
#[derive(Debug)]
pub struct BoundedZip<'b> {
    zip: zip::ZipArchive<Cursor<&'b [u8]>>,
    limits: ReadLimits,
    read_total: Cell<u64>,
    /// A read passed a limit: it is lost, and nothing more is decompressed.
    exhausted: Cell<bool>,
}

/// Number of entries announced by the end of central directory (Zip64 included), without
/// indexing the archive.
pub(crate) fn declared_entries(bytes: &[u8]) -> Option<u64> {
    let from = bytes.len().saturating_sub(65_557);
    let at = bytes.get(from..)?.windows(4).rposition(|w| w == EOCD)? + from;
    let le16 = |o: usize| bytes.get(o..o + 2).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes);
    let entries = le16(at + 10)?;
    if entries != 0xFFFF {
        return Some(u64::from(entries));
    }
    // Zip64: the locator precedes the EOCD and points to the Zip64 record.
    let loc = at.checked_sub(20)?;
    if bytes.get(loc..loc + 4)? != ZIP64_LOCATOR {
        return None;
    }
    let off = usize::try_from(u64::from_le_bytes(bytes.get(loc + 8..loc + 16)?.try_into().ok()?)).ok()?;
    Some(u64::from_le_bytes(bytes.get(off.checked_add(32)?..off.checked_add(40)?)?.try_into().ok()?))
}

/// `%XX` decoded (relationship targets are URIs).
pub(crate) fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| char::from(c).to_digit(16);
        match (b[i], b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(high), Some(low)) => {
                out.push(u8::try_from(high * 16 + low).unwrap_or(b'?'));
                i += 3;
            }
            (c, _, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Path of a relationship target relative to the part `base_part`, percent-decoded (`..` past
/// the root are ignored); an empty `base_part` resolves from the package root.
#[must_use]
pub fn resolve_path(base_part: &str, target: &str) -> String {
    let target = percent_decode(target);
    if let Some(abs) = target.strip_prefix('/') {
        return abs.to_owned();
    }
    let mut segs: Vec<&str> = base_part.split('/').collect();
    segs.pop();
    for s in target.split('/') {
        match s {
            ".." => {
                segs.pop();
            }
            "." | "" => {}
            s => segs.push(s),
        }
    }
    segs.join("/")
}

impl<'b> BoundedZip<'b> {
    /// Opens the archive: the input size, a compound file (encrypted or legacy Office) and the
    /// entry count announced by its end are checked before the central directory is indexed.
    ///
    /// # Errors
    ///
    /// `TooLarge` past `max_input_bytes` or `max_parts`, `Protected` for a compound file,
    /// `Unreadable` when the bytes are not a zip.
    pub fn open(bytes: &'b [u8], limits: &ReadLimits) -> Result<Self, ImportError> {
        if count_u64(bytes.len()) > limits.max_input_bytes {
            return Err(too_large("input"));
        }
        if bytes.starts_with(&CFB_MAGIC) {
            return Err(protected());
        }
        if declared_entries(bytes).is_some_and(|n| n > count_u64(limits.max_parts)) {
            return Err(too_large("entries"));
        }
        let zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| zip_error("archive", e))?;
        if zip.len() > limits.max_parts {
            return Err(too_large("entries"));
        }
        Ok(Self { zip, limits: *limits, read_total: Cell::new(0), exhausted: Cell::new(false) })
    }

    /// The read limits.
    #[must_use]
    pub fn limits(&self) -> &ReadLimits {
        &self.limits
    }

    /// Bytes decompressed so far, failed reads included.
    #[must_use]
    pub fn decompressed(&self) -> u64 {
        self.read_total.get()
    }

    /// True if the part exists (nothing is read).
    #[must_use]
    pub fn exists(&self, part: &str) -> bool {
        self.zip.index_for_name(part).is_some()
    }

    /// Bytes of a part, bounded by the limits.
    ///
    /// A read never decompresses more than `min(max_part_bytes, remaining budget) + 1` bytes,
    /// which count in the total even when it fails; after a limit is passed, every read fails
    /// without decompressing (a bomb referenced a thousand times is decompressed once).
    ///
    /// # Errors
    ///
    /// `TooLarge` past a limit, `Unreadable` for a missing or corrupt entry.
    pub fn read(&self, part: &str) -> Result<Vec<u8>, ImportError> {
        if self.exhausted.get() {
            return Err(too_large("total"));
        }
        let mut zip = self.zip.clone();
        let file = zip.by_name(part).map_err(|e| zip_error(part, e))?;
        let cap = self.limits.max_part_bytes;
        let remaining = self.limits.max_total_bytes.saturating_sub(self.read_total.get());
        let mut out = Vec::new();
        // On error, `read_to_end` keeps in `out` what was already decompressed.
        let read = file.take(cap.min(remaining).saturating_add(1)).read_to_end(&mut out);
        let n = count_u64(out.len());
        self.read_total.set(self.read_total.get().saturating_add(n));
        read.map_err(|e| io_error(part, e))?;
        let over = if n > cap {
            Some(too_large(part))
        } else if n > remaining {
            Some(too_large("total"))
        } else {
            None
        };
        if let Some(e) = over {
            self.exhausted.set(true);
            return Err(e);
        }
        Ok(out)
    }

    /// UTF-8 text of a part, its BOM removed.
    ///
    /// # Errors
    ///
    /// As [`Self::read`], and `Unreadable` when the part is not UTF-8.
    pub fn text(&self, part: &str) -> Result<String, ImportError> {
        let s = String::from_utf8(self.read(part)?).map_err(|e| utf8_error(part, e))?;
        Ok(s.strip_prefix('\u{feff}').map(str::to_owned).unwrap_or(s))
    }
}

/// Parses an XML part: no DTD, at most `nodes` nodes, 256 levels and 1024 attributes per element.
///
/// # Errors
///
/// `TooLarge` past `nodes`, `Unreadable` for malformed, too deep or DTD-bearing XML.
pub fn parse_xml<'i>(text: &'i str, part: &str, nodes: u32) -> Result<Document<'i>, ImportError> {
    crate::pptx::xml::parse(text, part, nodes)
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod bounded_tests;
