//! The extraction of a knowledge document, in the worker (t-1755).
//!
//! Rules:
//! - The worker reads the document of `ExtractDocument` with `oku_slides_extract::extract`
//!   (its own thread, a panic refused as `Damaged`). A refusal answers `ExtractRefused`; the
//!   thread failing to start answers `Error Setup`, a fault of the worker, never a refusal.
//! - `Extracted` carries the Markdown, then each image, in its payload; the header says their
//!   lengths. Images stop at the first one past the frame (`max_frame`) or past
//!   `max_image_bytes`: the rest are dropped and counted in the `Images` cut. Markdown alone
//!   past the frame is refused `TooLarge`.
//! - The host checks the reply before trusting it: the lengths add up to the payload exactly,
//!   the Markdown is UTF-8 within `max_text_bytes` (the extractor's own bound, notices
//!   included) and clean (`is_clean_text`), the title within `MAX_TITLE_CHARS`, one cut per
//!   kind, the images within the limits, each the `RASTERS` type its bytes sniff to with a
//!   readable size within `max_image_pixels`; otherwise `Frame`. Each part is copied out alone.
//! - No pack is blamed and nothing is kept: the session runs each import in a child worker
//!   built for it, under the `[knowledge]` bounds.
//! - Debug builds only: `OKU_SLIDES_TEST_EXTRACT_SLEEP_MS` sleeps and
//!   `OKU_SLIDES_TEST_EXTRACT_ALLOC_MIB` allocates gradually before extracting, the tests'
//!   timeout and memory hooks; release builds never read them.

use oku_slides_core::knowledge::{
    Cut, CutKind, DocumentType, ExtractLimits, ExtractRefusal, Extracted, ExtractedImage, MAX_TITLE_CHARS, RASTERS,
    is_clean_text,
};
use oku_slides_core::media::{dimensions, sniff};
use oku_slides_core::num::{count_u64, size_usize};
use oku_slides_import::ReadLimits;
use oku_slides_render::{RenderError, WireError};

use crate::client::{RenderWorker, unexpected};
use crate::protocol::{ImageMeta, Request, Response};
use crate::serve::Reply;

/// Answers `ExtractDocument`: the extraction of `document`, fitted to a `max_frame` payload.
pub(crate) fn answer(
    document: &[u8],
    kind: DocumentType,
    limits: &ExtractLimits,
    read: &ReadLimits,
    token: &str,
    max_frame: usize,
) -> Result<Reply, WireError> {
    let _held = test_hooks();
    match oku_slides_extract::extract(document, kind, limits, read, token) {
        Ok(Ok(extracted)) => {
            Ok(reply_of(extracted, limits.max_image_bytes, crate::frame::extract_payload(limits, max_frame)))
        }
        Ok(Err(refusal)) => Ok((Response::ExtractRefused { refusal }, Vec::new())),
        Err(e) => Err(WireError::Setup { message: format!("extract: {e}") }),
    }
}

/// The reply carrying `extracted`: its Markdown, then the images that fit both `max_frame` and
/// `max_image_bytes`, in order; the others are counted in the `Images` cut.
///
/// It stops at the first image that does not fit: placeholders cite images by position, so
/// none may be skipped. The extractor already keeps images within `max_image_bytes`, so only
/// the frame cap stops it in practice; the byte check guards a broken extractor.
pub(crate) fn reply_of(extracted: Extracted, max_image_bytes: u64, max_frame: usize) -> Reply {
    let Extracted { title, markdown, images, mut cuts } = extracted;
    if markdown.len() > max_frame {
        return (Response::ExtractRefused { refusal: ExtractRefusal::TooLarge }, Vec::new());
    }
    let markdown_len = markdown.len();
    let mut payload = markdown.into_bytes();
    let mut metas = Vec::new();
    let max_image_bytes = size_usize(max_image_bytes);
    let mut image_bytes = 0usize;
    let total = images.len();
    for image in images {
        let len = image.bytes.len();
        let bytes = image_bytes.saturating_add(len);
        if payload.len().saturating_add(len) > max_frame || bytes > max_image_bytes {
            break;
        }
        image_bytes = bytes;
        payload.extend_from_slice(&image.bytes);
        metas.push(ImageMeta { mime: image.mime, len });
    }
    let dropped = total - metas.len();
    if dropped > 0 {
        add_images_cut(&mut cuts, count_u64(dropped));
    }
    (Response::Extracted { title, markdown_len, images: metas, cut: cuts }, payload)
}

/// Adds `count` dropped images to the `Images` cut of `cuts`, created if missing.
fn add_images_cut(cuts: &mut Vec<Cut>, count: u64) {
    match cuts.iter_mut().find(|c| c.what == CutKind::Images) {
        Some(cut) => cut.count = cut.count.saturating_add(count),
        None => cuts.push(Cut { what: CutKind::Images, count }),
    }
}

/// The extraction an `Extracted` reply carries, checked against its payload and `limits`.
pub(crate) fn split(
    title: Option<String>,
    markdown_len: usize,
    images: Vec<ImageMeta>,
    cuts: Vec<Cut>,
    mut payload: Vec<u8>,
    limits: &ExtractLimits,
) -> Result<Extracted, RenderError> {
    let total = images.iter().try_fold(markdown_len, |sum, i| sum.checked_add(i.len));
    if total != Some(payload.len()) {
        return Err(RenderError::frame(format!(
            "extraction reply lengths ({total:?}) do not match its {} payload bytes",
            payload.len()
        )));
    }
    if title.as_ref().is_some_and(|t| t.chars().nth(MAX_TITLE_CHARS).is_some()) {
        return Err(RenderError::frame(format!("extraction reply title past {MAX_TITLE_CHARS} characters")));
    }
    // One cut per kind at most: refused past the number of kinds first, so the pairwise check
    // runs over a handful.
    if cuts.len() > CutKind::ALL.len()
        || cuts.iter().enumerate().any(|(i, c)| cuts[..i].iter().any(|d| d.what == c.what))
    {
        return Err(RenderError::frame("extraction reply states a cut kind twice".to_owned()));
    }
    if markdown_len > limits.max_text_bytes {
        return Err(RenderError::frame(format!(
            "extraction reply markdown of {markdown_len} bytes past its {} byte limit",
            limits.max_text_bytes
        )));
    }
    let image_bytes = payload.len() - markdown_len;
    if images.len() > limits.max_images || image_bytes > size_usize(limits.max_image_bytes) {
        return Err(RenderError::frame(format!(
            "extraction reply past its image limits: {} images, {image_bytes} bytes",
            images.len()
        )));
    }
    // Each image is copied out on its own, never a split tail keeping the payload's capacity.
    let mut out = Vec::with_capacity(images.len());
    let mut at = markdown_len;
    for meta in images {
        let end = at + meta.len;
        let bytes = payload
            .get(at..end)
            .ok_or_else(|| RenderError::frame(format!("extraction reply image at {at}..{end} past its payload")))?;
        if !RASTERS.contains(&meta.mime.as_str()) || sniff(bytes) != meta.mime {
            return Err(RenderError::frame(format!(
                "extraction reply image of type {:?} is not that raster ({:?})",
                meta.mime,
                sniff(bytes)
            )));
        }
        match dimensions(bytes, &meta.mime) {
            Some((w, h)) if u64::from(w) * u64::from(h) <= limits.max_image_pixels => {}
            size => {
                return Err(RenderError::frame(format!(
                    "extraction reply image of size {size:?} unreadable or past {} pixels",
                    limits.max_image_pixels
                )));
            }
        }
        out.push(ExtractedImage { mime: meta.mime, bytes: bytes.to_vec() });
        at = end;
    }
    payload.truncate(markdown_len);
    payload.shrink_to_fit();
    let markdown = String::from_utf8(payload)
        .map_err(|e| RenderError::frame(format!("extraction reply markdown is not UTF-8: {e}")))?;
    if !is_clean_text(&markdown) {
        return Err(RenderError::frame("extraction reply markdown holds stripped characters".to_owned()));
    }
    Ok(Extracted { title, markdown, images: out, cuts })
}

impl RenderWorker {
    /// Extracts the document `bytes` of kind `kind` in the worker, within `limits` and `read`;
    /// `token` names its image placeholders.
    ///
    /// The outer result is the worker's (transport, timeout, memory), the inner one the
    /// document's refusal. No pack is blamed. Call it on a worker built for the extraction,
    /// never on a deck's render worker.
    ///
    /// # Errors
    ///
    /// Fails if the worker does not start, dies, exceeds its bounds, cannot start its
    /// extraction thread, or sends a reply that does not hold together.
    pub fn extract_document(
        &self,
        bytes: &[u8],
        kind: DocumentType,
        limits: &ExtractLimits,
        read: &ReadLimits,
        token: &str,
    ) -> Result<Result<Extracted, ExtractRefusal>, RenderError> {
        let mut link = self.link();
        self.prepare(&mut link)?;
        let request = Request::ExtractDocument { kind, limits: limits.clone(), read: *read, token: token.to_owned() };
        match link.call(&request, bytes)? {
            (Response::Extracted { title, markdown_len, images, cut }, payload) => {
                split(title, markdown_len, images, cut, payload, limits).map(Ok)
            }
            (Response::ExtractRefused { refusal }, _) => Ok(Err(refusal)),
            (Response::Error { error }, _) => Err(error.into_render()),
            (other, _) => Err(unexpected(&other)),
        }
    }
}

/// What the tests' hooks hold until the extraction ends.
type Held = Vec<Vec<u8>>;

/// The tests' timeout and memory hooks (debug builds only): sleep, then allocate 1 MiB every 5 ms.
#[cfg(debug_assertions)]
fn test_hooks() -> Held {
    let hook = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok());
    if let Some(ms) = hook("OKU_SLIDES_TEST_EXTRACT_SLEEP_MS") {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
    let mut held = Held::new();
    for _ in 0..hook("OKU_SLIDES_TEST_EXTRACT_ALLOC_MIB").unwrap_or(0) {
        // Non-zero bytes: every page is written, so it counts in the resident memory.
        held.push(vec![1; 1 << 20]);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    held
}

/// Release builds have no test hook.
#[cfg(not(debug_assertions))]
fn test_hooks() -> Held {
    Held::new()
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod extract_tests;
