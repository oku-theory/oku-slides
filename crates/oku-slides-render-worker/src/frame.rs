//! Pipe frames: the header length (u32 big-endian), the JSON header, the payload length
//! (u32 big-endian), the raw payload.
//!
//! Bounds per part and per kind ([`FrameLimits`]), drawn from `max_frame_mib`: a header
//! (JSON: deck, pack, layout) at most an eighth, an SVG at most a quarter, a font, an asset, a
//! `.pptx`, a document, a PNG or a PDF the whole size, an extraction its Markdown and image
//! bounds within it; the other replies have no payload.

use std::io::{Read, Write};

use oku_slides_core::knowledge::ExtractLimits;
use oku_slides_core::num::size_usize;
use serde::Serialize;
use snafu::{ResultExt, ensure};

use crate::error::{FrameError, IoSnafu, JsonSnafu, TooLargeSnafu, TruncatedSnafu};
use crate::protocol::Request;

/// A frame header: at most `max / HEADER_SHARE`.
const HEADER_SHARE: usize = 8;
/// An SVG payload: at most `max / SVG_SHARE`.
const SVG_SHARE: usize = 4;

/// The bounds of a frame, in bytes: its header and its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameLimits {
    pub(crate) header: usize,
    pub(crate) payload: usize,
}

impl FrameLimits {
    /// The same bound `max` for both parts.
    pub(crate) fn uniform(max: usize) -> Self {
        Self { header: max, payload: max }
    }

    /// A request from the host: the header at most `max / 8`, the payload (font, asset, `.pptx`) `max`.
    pub(crate) fn requests(max: usize) -> Self {
        Self { header: max / HEADER_SHARE, payload: max }
    }

    /// The reply to `req`: the header at most `max / 8`; the payload at most `max / 4` for an
    /// SVG, `max` for a PNG (of a page or a reference) or a PDF, an extraction's Markdown and
    /// image bounds within `max`, nothing otherwise.
    pub(crate) fn reply_to(req: &Request, max: usize) -> Self {
        let payload = match req {
            Request::Svg { .. } => max / SVG_SHARE,
            Request::Png { .. } | Request::Pdf { .. } | Request::ReferencePng { .. } => max,
            Request::ExtractDocument { limits, .. } => extract_payload(limits, max),
            _ => 0,
        };
        Self { header: max / HEADER_SHARE, payload }
    }
}

/// The payload bound of an extraction reply: its Markdown and image bounds, at most `max`.
pub(crate) fn extract_payload(limits: &ExtractLimits, max: usize) -> usize {
    max.min(limits.max_text_bytes.saturating_add(size_usize(limits.max_image_bytes)))
}

/// A frame read: its JSON header, not interpreted, and its raw payload.
#[derive(Debug)]
pub(crate) struct Frame {
    pub(crate) header: Vec<u8>,
    pub(crate) payload: Vec<u8>,
}

/// Writes a frame (the header serialized as JSON, then the payload) and flushes the buffer.
///
/// Each part is bounded by `limits`: past it, `TooLarge` before any byte is written (the
/// stream stays usable).
pub(crate) fn write_frame(
    w: &mut impl Write,
    header: &impl Serialize,
    payload: &[u8],
    limits: FrameLimits,
) -> Result<(), FrameError> {
    let header = serde_json::to_vec(header).context(JsonSnafu)?;
    let mut lens = [0u32; 2];
    let parts = [(&header[..], limits.header, false), (payload, limits.payload, true)];
    for (len, (part, max, payload)) in lens.iter_mut().zip(parts) {
        let max = max.min(u32::MAX as usize);
        ensure!(part.len() <= max, TooLargeSnafu { len: part.len(), max, payload });
        *len = u32::try_from(part.len()).map_err(|_| TooLargeSnafu { len: part.len(), max, payload }.build())?;
    }
    for (len, part) in lens.iter().zip([&header[..], payload]) {
        w.write_all(&len.to_be_bytes()).context(IoSnafu)?;
        w.write_all(part).context(IoSnafu)?;
    }
    w.flush().context(IoSnafu)
}

/// Reads a frame; `None` at a clean end of the stream, between two frames.
///
/// Each part is bounded by `limits` before anything is allocated.
pub(crate) fn read_frame(r: &mut impl Read, limits: FrameLimits) -> Result<Option<Frame>, FrameError> {
    read_frame_with(r, || limits)
}

/// [`read_frame`], its bounds asked of `limits` after the first byte: those of the request
/// sent meanwhile, for a reader that was already waiting.
pub(crate) fn read_frame_with(
    r: &mut impl Read,
    limits: impl FnOnce() -> FrameLimits,
) -> Result<Option<Frame>, FrameError> {
    let mut len = [0u8; 4];
    // A clean end: no byte before the frame. A signal (EINTR) is not one.
    let first = loop {
        match r.read(&mut len[..1]) {
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            other => break other.context(IoSnafu)?,
        }
    };
    match first {
        0 => return Ok(None),
        _ => read_exact(r, &mut len[1..])?,
    }
    let limits = limits();
    let header = read_part(r, u32::from_be_bytes(len) as usize, limits.header, false)?;
    read_exact(r, &mut len)?;
    let payload = read_part(r, u32::from_be_bytes(len) as usize, limits.payload, true)?;
    Ok(Some(Frame { header, payload }))
}

/// `payload`: the part is the payload (otherwise the header).
fn read_part(r: &mut impl Read, len: usize, max: usize, payload: bool) -> Result<Vec<u8>, FrameError> {
    ensure!(len <= max, TooLargeSnafu { len, max, payload });
    let mut buf = vec![0u8; len];
    read_exact(r, &mut buf)?;
    Ok(buf)
}

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<(), FrameError> {
    match r.read_exact(buf) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Err(TruncatedSnafu.build()),
        Err(e) => Err(e).context(IoSnafu),
    }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod frame_tests;
