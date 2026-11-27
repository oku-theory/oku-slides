//! PDF: each page's text as `## Page N`, a page without text said so, its images after it.
//! A PDF with no text on any page is a scan, refused rather than imported empty.
//!
//! Bounded at parse time: lopdf decodes object streams within the part limit; the page tree
//! is walked without recursion, each node once (a loop is damage), and only its first
//! [`MAX_PAGES`] pages are read, the rest stated as cut; each page's content and forms are
//! walked by `pdf_content` within an eighth of the part limit, room for the parsed content
//! being many times the size of its bytes; every image is decoded through a size cap that
//! stops inflating before allocating past it.
//!
//! What pages and images decode counts against one total, `max_total_bytes`, fonts and forms
//! read whole decoded once for the document (t-1760). Pages stop being walked once that
//! total or the text budget is spent, the pages left cut and stated.
//!
//! Strict on the page tree: a node without `/Type`, or one met twice (a loop, a shared
//! subtree), makes the document damaged rather than guessed at.

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use lopdf::{DecompressError, Document, Error, LoadOptions, Object, ObjectId, ParseError, Stream};
use oku_slides_core::knowledge::{Cut, CutKind, ExtractLimits, ExtractRefusal};
use oku_slides_core::num::count_u64;
use oku_slides_core::num::size_usize;
use oku_slides_import::ReadLimits;

use crate::doc::{Doc, Fetch, Inline, Kind};
use crate::images::Skip;
use crate::markdown::{Bound, clean, escape_text};
use crate::pdf_content::{Cache, Failure, walk};
use oku_slides_core::knowledge::{Extracted, MAX_TITLE_CHARS};

/// Most pages read; the rest are cut, and the cut stated.
const MAX_PAGES: usize = 2000;
/// What a page without any text says in its place.
const BLANK_PAGE: &str = "*(no text on this page)*";
/// The part limit divided by this bounds one page's decoded content: lopdf's parsed
/// operations take many times the bytes they are read from.
const TEXT_SHARE: usize = 8;

/// Extracts the PDF `bytes`.
pub(crate) fn extract(
    bytes: &[u8],
    limits: &ExtractLimits,
    read: &ReadLimits,
    token: &str,
) -> Result<Extracted, ExtractRefusal> {
    let part = size_usize(read.max_part_bytes);
    let options = LoadOptions { max_decompressed_size: Some(part), ..LoadOptions::default() };
    let pdf = Document::load_mem_with_options(bytes, options).map_err(|e| load_refusal(&e))?;
    if pdf.is_encrypted() || pdf.was_encrypted() {
        return Err(ExtractRefusal::Encrypted);
    }
    let (pages, beyond) = page_ids(&pdf)?;
    let pixels = limits.max_image_pixels.min(read.max_image_pixels);
    let mut reader = Reader {
        pdf: &pdf,
        part,
        pixels,
        left: Cell::new(size_usize(read.max_total_bytes)),
        cache: Cache::default(),
        doc: Doc::new(limits, pixels, token),
        any_text: false,
        counts: Counts::default(),
    };
    // One Pages cut, one bound: a walk stopped by the budget or the read total names that
    // bound for every page left, those past `MAX_PAGES` included (accepted, t-1760).
    let (mut beyond, mut bound) = (beyond, Bound::Limit(MAX_PAGES));
    for (n, (i, &id)) in (1..).zip(pages.iter().enumerate()) {
        let stop = if reader.doc.room() == 0 {
            Bound::Budget
        } else if reader.left.get() == 0 {
            Bound::ReadLimit
        } else {
            reader.page(n, id);
            continue;
        };
        beyond += pages.len() - i;
        bound = stop;
        break;
    }
    if !reader.any_text {
        return Err(reader.counts.no_text(beyond));
    }
    let skipped = reader.counts.skipped();
    let skipped: Vec<&str> = skipped.iter().map(String::as_str).collect();
    let cut = Cut { what: CutKind::Pages, count: count_u64(beyond) };
    let mut out = reader.doc.finish(cut, bound, &skipped, limits)?;
    out.title = title(&pdf);
    Ok(out)
}

/// The refusal for a document lopdf could not load.
fn load_refusal(err: &Error) -> ExtractRefusal {
    match err {
        Error::Parse(ParseError::InvalidFileHeader) => ExtractRefusal::TypeMismatch,
        Error::Decompress(DecompressError::MemoryLimitExceeded { .. }) => ExtractRefusal::TooLarge,
        Error::InvalidPassword | Error::Decryption(_) | Error::UnsupportedSecurityHandler(_) => {
            ExtractRefusal::Encrypted
        }
        // Every other load error means bytes that do not parse as a PDF: syntax, cross
        // references, object streams, reference chains, I/O on the buffer.
        _damage => ExtractRefusal::Damaged,
    }
}

/// The first [`MAX_PAGES`] page ids in reading order, and how many pages follow them.
///
/// The tree is walked with an explicit stack, each node visited once: a node met twice (a
/// loop or a shared subtree) or a kid that is neither a page nor a page tree node is damage.
fn page_ids(pdf: &Document) -> Result<(Vec<ObjectId>, usize), ExtractRefusal> {
    let damaged = |_| ExtractRefusal::Damaged;
    let root = pdf.catalog().and_then(|c| c.get(b"Pages")).and_then(Object::as_reference).map_err(damaged)?;
    let mut seen = BTreeSet::new();
    let mut stack = vec![root];
    let (mut pages, mut past) = (Vec::new(), 0_usize);
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            return Err(ExtractRefusal::Damaged);
        }
        let node = pdf.get_dictionary(id).map_err(damaged)?;
        match node.get_type() {
            Ok(b"Pages") => {
                let kids = node.get(b"Kids").and_then(Object::as_array).map_err(damaged)?;
                for kid in kids.iter().rev() {
                    stack.push(kid.as_reference().map_err(damaged)?);
                }
            }
            Ok(b"Page") if pages.len() < MAX_PAGES => pages.push(id),
            Ok(b"Page") => past += 1,
            // A kid of another type, or without one: the tree is not a page tree.
            Ok(_) | Err(_) => return Err(ExtractRefusal::Damaged),
        }
    }
    Ok((pages, past))
}

/// The Info dictionary's `Title`, raw, cut to [`MAX_TITLE_CHARS`] characters.
fn title(pdf: &Document) -> Option<String> {
    let info = pdf.trailer.get(b"Info").ok()?;
    let (_, info) = pdf.dereference(info).ok()?;
    let (_, title) = pdf.dereference(info.as_dict().ok()?.get(b"Title").ok()?).ok()?;
    let text: String = lopdf::decode_text_string(title).ok()?.chars().take(MAX_TITLE_CHARS).collect();
    (!text.is_empty()).then_some(text)
}

/// What the pages read left out or could not read, for the closing notice and the refusal.
#[derive(Debug, Default)]
struct Counts {
    /// Pages whose text did not read, in part or whole.
    unreadable: usize,
    /// Pages whose content decodes past the text limit.
    over: usize,
    /// Forms drawn but not read.
    forms: usize,
    /// Inline images, not imported.
    inline_images: usize,
    /// Drawn objects that do not resolve to a readable `XObject`.
    broken: usize,
}

impl Counts {
    /// The refusal of a document none of whose pages read had text, `beyond` pages past
    /// the cap: too large when content or pages were left out for size, damaged when some
    /// did not read, unsupported when only forms were not read, else a scan.
    fn no_text(&self, beyond: usize) -> ExtractRefusal {
        if self.over > 0 || beyond > 0 {
            ExtractRefusal::TooLarge
        } else if self.unreadable > 0 || self.broken > 0 {
            ExtractRefusal::Damaged
        } else if self.forms > 0 {
            ExtractRefusal::Unsupported
        } else {
            ExtractRefusal::NoTextLayer
        }
    }

    /// The content skipped on purpose or unreadable, for the closing notice.
    fn skipped(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.unreadable > 0 {
            out.push(format!("text of {} unreadable pages", self.unreadable));
        }
        if self.over > 0 {
            out.push(format!("text of {} pages past the read limits", self.over));
        }
        if self.forms > 0 {
            out.push(format!("{} embedded forms not read", self.forms));
        }
        if self.inline_images > 0 {
            out.push(format!("{} inline images", self.inline_images));
        }
        if self.broken > 0 {
            out.push(format!("{} drawn objects that do not resolve", self.broken));
        }
        out
    }
}

/// The walk over the pages kept, writing into one [`Doc`].
struct Reader<'p, 'l> {
    pdf: &'p Document,
    /// Most bytes one image may decode to.
    part: usize,
    /// The pixel limit of one image.
    pixels: u64,
    /// Bytes pages and images may still decode, of `max_total_bytes`.
    left: Cell<usize>,
    /// Fonts and forms read whole, shared by the pages.
    cache: Cache<'p>,
    doc: Doc<'l>,
    /// Whether any page had text.
    any_text: bool,
    counts: Counts,
}

impl Reader<'_, '_> {
    /// Writes page `id`, number `n` from 1: its heading, its text or the blank notice, then
    /// the images it draws.
    fn page(&mut self, n: u32, id: ObjectId) {
        let heading = format!("## Page {n}");
        self.doc.block(&heading, 0, 0);
        let walked = walk(self.pdf, id, (self.part / TEXT_SHARE).min(self.left.get()), &mut self.cache);
        self.left.set(self.left.get().saturating_sub(walked.decoded));
        match walked.failure {
            Some(Failure::Over) => self.counts.over += 1,
            Some(Failure::Unreadable) => self.counts.unreadable += 1,
            None => {}
        }
        self.counts.forms += walked.forms_unread;
        self.counts.inline_images += walked.inline_images;
        self.counts.broken += walked.broken;
        let mut wrote = false;
        for para in clean(&walked.text).split("\n\n") {
            let para = para.trim();
            if para.is_empty() {
                continue;
            }
            self.doc.block(&escape_text(para), count_u64(para.len()), 0);
            wrote = true;
        }
        if wrote {
            self.any_text = true;
        } else {
            self.doc.block(BLANK_PAGE, 0, 0);
        }
        self.images(&walked.images);
    }

    /// Cites the image `XObject`s `found`, read through the document's image limits.
    fn images(&mut self, found: &[ObjectId]) {
        if found.is_empty() {
            return;
        }
        let key = |i: ObjectId| format!("{} {}", i.0, i.1);
        let keys: BTreeMap<String, ObjectId> = found.iter().map(|&i| (key(i), i)).collect();
        let mut inline = Inline::new(self.doc.room());
        for &i in found {
            inline.image(&key(i), "");
        }
        let (pdf, part, pixels, left) = (self.pdf, self.part, self.pixels, &self.left);
        let fetch = |key: &str| -> Result<Cow<'_, [u8]>, Skip> {
            let id = keys.get(key).ok_or(Skip::Unreadable)?;
            let stream = pdf.get_object(*id).and_then(Object::as_stream).map_err(|_| Skip::Unreadable)?;
            image_bytes(pdf, stream, part, pixels, left)
        };
        let fetch: Fetch<'_, '_> = &fetch;
        self.doc.paragraph(&inline, Kind::Para, fetch);
    }
}

/// Entries whose image would show other pixels than its samples: a mask, a soft mask, a
/// decode array other than the identity; skipped rather than shown wrong (D7).
const ALTERING: [(&[u8], &str); 3] = [(b"Mask", "a mask"), (b"SMask", "a soft mask"), (b"Decode", "a decode array")];

/// Whether `Decode` array `decode` maps every component to itself: `[0 1 0 1 …]`.
fn identity_decode(decode: &[Object]) -> bool {
    let at = |o: &Object, want: f32| o.as_float().is_ok_and(|v| (v - want).abs() < f32::EPSILON);
    !decode.is_empty() && decode.chunks(2).all(|pair| matches!(pair, [lo, hi] if at(lo, 0.0) && at(hi, 1.0)))
}

/// The bytes of image `stream` as a raster file: a `DCTDecode` stream in RGB or gray as the
/// JPEG it is (an RGB or gray ICC profile included), a Flate-coded 8-bit RGB or gray one as PNG decoded within what `left` has left
/// of the document's total; an image mask, a masked image or one with a decode array is
/// listed with that reason (an identity decode array alters nothing), any other by its filter
/// or color space.
fn image_bytes<'s>(
    pdf: &Document,
    stream: &'s Stream,
    part: usize,
    pixels: u64,
    left: &Cell<usize>,
) -> Result<Cow<'s, [u8]>, Skip> {
    let entry = |key: &[u8]| stream.dict.get(key).and_then(|o| pdf.dereference(o)).map(|(_, o)| o);
    if entry(b"ImageMask").and_then(Object::as_bool).unwrap_or(false) {
        return Err(Skip::Format("PDF image mask".to_owned()));
    }
    let altering = |key: &[u8]| match entry(key) {
        Ok(Object::Array(decode)) if key == b"Decode" => !identity_decode(decode),
        _present => stream.dict.has(key),
    };
    if let Some((_, what)) = ALTERING.iter().find(|(key, _)| altering(key)) {
        return Err(Skip::Format(format!("PDF image with {what}")));
    }
    let filters = stream.filters().map_err(|_| Skip::Format("PDF image without filter".to_owned()))?;
    match filters.as_slice() {
        [b"DCTDecode"] => color(pdf, entry(b"ColorSpace")).map(|_| Cow::Borrowed(stream.content.as_slice())),
        [b"FlateDecode"] => flate_png(pdf, stream, part, pixels, left).map(Cow::Owned),
        [one] => Err(Skip::Format(format!("PDF {}", String::from_utf8_lossy(one)))),
        chain => Err(Skip::Format(format!("PDF chain of {} filters", chain.len()))),
    }
}

/// The PNG color type and component count of color space `space`: `DeviceRGB`, `DeviceGray`
/// or an ICC profile of 3 or 1 components; any other skipped with its name or component count.
/// A CMYK image (an inverted Adobe JPEG among them) would show inverted or wrong colors.
fn color(pdf: &Document, space: lopdf::Result<&Object>) -> Result<(png::ColorType, u64), Skip> {
    match space.map_err(|_| Skip::Unreadable)? {
        Object::Name(name) => match name.as_slice() {
            b"DeviceRGB" => Ok((png::ColorType::Rgb, 3)),
            b"DeviceGray" => Ok((png::ColorType::Grayscale, 1)),
            other => Err(Skip::Format(format!("PDF {} image", String::from_utf8_lossy(other)))),
        },
        Object::Array(space) => match space.as_slice() {
            [Object::Name(family), profile] if family == b"ICCBased" => {
                let n = pdf
                    .dereference(profile)
                    .and_then(|(_, o)| o.as_stream())
                    .and_then(|s| s.dict.get(b"N"))
                    .and_then(|n| pdf.dereference(n))
                    .and_then(|(_, n)| n.as_i64())
                    .map_err(|_| Skip::Unreadable)?;
                match n {
                    3 => Ok((png::ColorType::Rgb, 3)),
                    1 => Ok((png::ColorType::Grayscale, 1)),
                    other => Err(Skip::Format(format!("PDF ICC image of {other} components"))),
                }
            }
            _indexed_or_other => Err(Skip::Format("PDF image in an indexed or special color space".to_owned())),
        },
        Object::Null
        | Object::Boolean(_)
        | Object::Integer(_)
        | Object::Real(_)
        | Object::String(..)
        | Object::Dictionary(_)
        | Object::Stream(_)
        | Object::Reference(_) => Err(Skip::Unreadable),
    }
}

/// A Flate-coded image decoded within its own size and written as PNG: 8 bits per component,
/// `DeviceRGB` or `DeviceGray`, no predictor; its size is checked against the pixel and part
/// limits and what `left` has left before anything is decoded. Its entries may be indirect.
fn flate_png(pdf: &Document, stream: &Stream, part: usize, pixels: u64, left: &Cell<usize>) -> Result<Vec<u8>, Skip> {
    let dict = &stream.dict;
    let entry = |key: &[u8]| dict.get(key).and_then(|o| pdf.dereference(o)).map(|(_, o)| o);
    let int = |key: &[u8]| entry(key).and_then(Object::as_i64).map_err(|_| Skip::Unreadable);
    let (w, h, bits) = (int(b"Width")?, int(b"Height")?, int(b"BitsPerComponent")?);
    let (color, components) = color(pdf, entry(b"ColorSpace"))?;
    if bits != 8 {
        return Err(Skip::Format(format!("PDF {bits}-bit image")));
    }
    if dict.has(b"DecodeParms") {
        return Err(Skip::Format("PDF image with a predictor".to_owned()));
    }
    let (w, h) = (u32::try_from(w).map_err(|_| Skip::Pixels)?, u32::try_from(h).map_err(|_| Skip::Pixels)?);
    let area = u64::from(w).checked_mul(u64::from(h)).ok_or(Skip::Pixels)?;
    if area == 0 {
        return Err(Skip::Unreadable);
    }
    if area > pixels {
        return Err(Skip::Pixels);
    }
    let raw = area.checked_mul(components).ok_or(Skip::Pixels)?;
    let raw = usize::try_from(raw).map_err(|_| Skip::ReadLimit)?;
    if raw > part || raw > left.get() {
        return Err(Skip::ReadLimit);
    }
    // Counted before inflating: an image that fails to inflate costs as much.
    left.set(left.get() - raw);
    let data = stream.decompressed_content_with_limit(raw).map_err(|e| match e {
        Error::Decompress(DecompressError::MemoryLimitExceeded { .. }) => Skip::ReadLimit,
        // Any other error: the stream does not inflate.
        _unreadable => Skip::Unreadable,
    })?;
    if data.len() != raw {
        return Err(Skip::Unreadable);
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_| Skip::Unreadable)?;
    writer.write_image_data(&data).map_err(|_| Skip::Unreadable)?;
    writer.finish().map_err(|_| Skip::Unreadable)?;
    Ok(out)
}

#[cfg(test)]
#[path = "pdf_tests.rs"]
mod pdf_tests;
