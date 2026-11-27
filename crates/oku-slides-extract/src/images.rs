//! The images of a text document (DOCX, PPTX, PDF): each read once, kept within the count,
//! byte and pixel limits, deduplicated by bytes, cited by placeholder, the rest listed by reason.

use std::borrow::Cow;
use std::collections::BTreeSet;

use oku_slides_core::knowledge::{Cut, CutKind, ExtractLimits, ExtractedImage, RASTERS};
use oku_slides_core::media::{dimensions, sniff};
use oku_slides_core::num::count_u64;

use crate::markdown::{Bound, cut_line, escape_cell};

/// Why an image was not imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Skip {
    /// Past `max_images`.
    Count,
    /// Past `max_image_bytes`.
    Bytes,
    /// Past the pixel limit.
    Pixels,
    /// A media type other than [`RASTERS`].
    Format(String),
    /// Bytes missing or a header that does not read.
    Unreadable,
    /// A part past the archive's read limits (its own size, or what the archive has left).
    ReadLimit,
    /// Cited in text left out for the budget.
    Budget,
}

/// The images of a document: each read at most once, checked against `max_images`, the
/// image byte limit and the pixel limit before it is kept, the rest counted by reason.
#[derive(Debug)]
pub(crate) struct Images<'l> {
    limits: &'l ExtractLimits,
    pixels: u64,
    token: &'l str,
    kept: Vec<(String, ExtractedImage)>,
    /// Keys whose bytes equal a kept image's, and that image's index.
    copies: Vec<(String, usize)>,
    bytes: u64,
    /// Images not imported, in order: key and reason.
    skipped: Vec<(String, Skip)>,
    seen: BTreeSet<String>,
}

/// A point to come back to with [`Images::rollback`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct Mark {
    kept: usize,
    copies: usize,
    skipped: usize,
}

impl<'l> Images<'l> {
    /// No image yet; `pixels` is the pixel limit of one image.
    pub(crate) fn new(limits: &'l ExtractLimits, pixels: u64, token: &'l str) -> Self {
        Self {
            limits,
            pixels,
            token,
            kept: Vec::new(),
            copies: Vec::new(),
            bytes: 0,
            skipped: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    /// The placeholder of image `key` (`![alt](oku-image-<token>-<i>)`), or `None` when it is
    /// not imported. A new image is checked against `max_images` and the byte limit before
    /// `read` runs, then its type and size are read from its header before it is kept. Bytes
    /// equal to a kept image's link that image (D4): a copy takes nothing of the limits, but
    /// one met once they are reached is not read, so not recognized: it is listed past the
    /// count or byte limit, as reading it to compare would spend the read limits.
    pub(crate) fn cite<'b>(
        &mut self,
        key: &str,
        alt: &str,
        read: impl FnOnce() -> Result<Cow<'b, [u8]>, Skip>,
    ) -> Option<String> {
        let known = self.kept.iter().position(|(k, _)| k == key);
        if let Some(i) = known.or_else(|| self.copies.iter().find(|(k, _)| k == key).map(|&(_, i)| i)) {
            return Some(self.placeholder(i, alt));
        }
        if self.seen.contains(key) {
            return None;
        }
        let image = self.check(read).and_then(|image| {
            if let Some(i) = self.kept.iter().position(|(_, kept)| kept.bytes == image.bytes) {
                return Ok(Err(i));
            }
            if self.bytes.saturating_add(count_u64(image.bytes.len())) > self.limits.max_image_bytes {
                return Err(Skip::Bytes);
            }
            Ok(Ok(image))
        });
        self.seen.insert(key.to_owned());
        match image {
            Ok(Ok(image)) => {
                self.bytes += count_u64(image.bytes.len());
                self.kept.push((key.to_owned(), image));
                Some(self.placeholder(self.kept.len() - 1, alt))
            }
            Ok(Err(i)) => {
                self.copies.push((key.to_owned(), i));
                Some(self.placeholder(i, alt))
            }
            Err(why) => {
                self.skipped.push((key.to_owned(), why));
                None
            }
        }
    }

    pub(crate) fn check<'b>(&self, read: impl FnOnce() -> Result<Cow<'b, [u8]>, Skip>) -> Result<ExtractedImage, Skip> {
        if self.kept.len() >= self.limits.max_images {
            return Err(Skip::Count);
        }
        if self.bytes >= self.limits.max_image_bytes {
            return Err(Skip::Bytes);
        }
        let bytes = read()?;
        let mime = sniff(&bytes);
        if !RASTERS.contains(&mime) {
            return Err(Skip::Format(mime.to_owned()));
        }
        let (w, h) = dimensions(&bytes, mime).ok_or(Skip::Unreadable)?;
        if u64::from(w) * u64::from(h) > self.pixels {
            return Err(Skip::Pixels);
        }
        Ok(ExtractedImage { mime: mime.to_owned(), bytes: bytes.into_owned() })
    }

    /// Counts image `key` as not imported for `why`, once.
    pub(crate) fn skip(&mut self, key: &str, why: Skip) {
        if self.seen.insert(key.to_owned()) {
            self.skipped.push((key.to_owned(), why));
        }
    }

    pub(crate) fn placeholder(&self, i: usize, alt: &str) -> String {
        format!("![{}](oku-image-{}-{i})", escape_cell(alt), self.token)
    }

    /// The current point.
    pub(crate) fn mark(&self) -> Mark {
        Mark { kept: self.kept.len(), copies: self.copies.len(), skipped: self.skipped.len() }
    }

    /// Forgets the images met since `mark` (their block was left out).
    pub(crate) fn rollback(&mut self, mark: Mark) {
        for (key, _) in self.copies.drain(mark.copies..) {
            self.seen.remove(&key);
        }
        for (key, image) in self.kept.drain(mark.kept..) {
            self.bytes -= count_u64(image.bytes.len());
            self.seen.remove(&key);
        }
        for (key, _) in self.skipped.drain(mark.skipped..) {
            self.seen.remove(&key);
        }
    }

    /// One notice line per reason an image was not imported, and their cut.
    pub(crate) fn notices(&self) -> (String, Option<Cut>) {
        let mut groups: Vec<(&Skip, u64)> = Vec::new();
        for (_, why) in &self.skipped {
            match groups.iter_mut().find(|(w, _)| *w == why) {
                Some((_, n)) => *n += 1,
                None => groups.push((why, 1)),
            }
        }
        let mut notices = String::new();
        for (why, count) in &groups {
            let bound = match why {
                Skip::Count => Bound::Limit(self.limits.max_images),
                Skip::Bytes => Bound::ImageBytes(self.limits.max_image_bytes),
                Skip::Pixels => Bound::Pixels(self.pixels),
                Skip::Format(mime) => Bound::Format(mime),
                Skip::Unreadable => Bound::Unreadable,
                Skip::ReadLimit => Bound::ReadLimit,
                Skip::Budget => Bound::Budget,
            };
            notices.push_str(&cut_line(Cut { what: CutKind::Images, count: *count }, bound));
        }
        let total = count_u64(self.skipped.len());
        (notices, (total > 0).then_some(Cut { what: CutKind::Images, count: total }))
    }

    /// The first kept image whose placeholder `markdown` lacks (its text cut to fit), the
    /// placeholders found in one scan.
    pub(crate) fn first_uncited(&self, markdown: &str) -> Option<usize> {
        let prefix = format!("(oku-image-{}-", self.token);
        let mut cited = vec![false; self.kept.len()];
        for (at, _) in markdown.match_indices(&prefix) {
            let rest = &markdown[at + prefix.len()..];
            let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            if rest[digits..].starts_with(')')
                && let Ok(i) = rest[..digits].parse::<usize>()
                && let Some(slot) = cited.get_mut(i)
            {
                *slot = true;
            }
        }
        cited.iter().position(|c| !c)
    }

    /// Counts the kept images from `from` on as cited in text left out for the budget; their
    /// copies go with them, listed for the same reason.
    pub(crate) fn demote(&mut self, from: usize) {
        for (key, image) in self.kept.drain(from..) {
            self.bytes -= count_u64(image.bytes.len());
            self.skipped.push((key, Skip::Budget));
        }
        let (gone, copies): (Vec<_>, Vec<_>) = self.copies.drain(..).partition(|&(_, i)| i >= from);
        self.copies = copies;
        self.skipped.extend(gone.into_iter().map(|(key, _)| (key, Skip::Budget)));
    }

    /// The images kept, in citation order.
    pub(crate) fn into_kept(self) -> Vec<ExtractedImage> {
        self.kept.into_iter().map(|(_, image)| image).collect()
    }
}

#[cfg(test)]
#[path = "images_tests.rs"]
mod images_tests;
