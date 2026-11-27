//! Turns an imported document into sanitized Markdown for the deck knowledge base.
//!
//! Pure and bounded: every read goes through [`ReadLimits`] and [`ExtractLimits`], every text
//! through one sanitizer (`markdown`). The session runs it in a worker process; see
//! `ARCHITECTURE.md`.

mod archive;
mod doc;
mod docx;
mod images;
mod markdown;
mod ods;
mod pdf;
mod pdf_content;
mod pptx;
mod sheet;
mod text;
mod xlsx;

use oku_slides_core::knowledge::{Cut, DocumentType, ExtractLimits, ExtractRefusal, Extracted};
use oku_slides_core::num::count_u64;
use oku_slides_import::ReadLimits;
use snafu::{Backtrace, ResultExt, Snafu};

/// The extraction itself failed, whatever the document: a fault of the worker, not a refusal.
///
/// Its message is not shown as is: the caller reports it as its worker's failure.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ExtractError {
    /// The extraction thread could not be started.
    #[snafu(display("could not start the extraction thread"))]
    Spawn {
        /// Why the system refused the thread.
        source: std::io::Error,
        /// Where it happened.
        backtrace: Backtrace,
    },
}

/// Extracts the document `bytes` of kind `kind` into an [`Extracted`] (`oku_slides_core::knowledge`).
///
/// Markdown, text, XLSX, ODS, DOCX, PPTX and PDF are read. Image `i` is cited as
/// `![alt](oku-image-<token>-<i>)`, `token` being the session's unpredictable placeholder name.
///
/// The reading runs on its own thread with an [`EXTRACT_STACK`] stack, under `catch_unwind`:
/// a parser that panics on hostile bytes refuses the document as `Damaged`.
///
/// The outer result is the worker's, the inner one the document's: `Ok(Err(refusal))` says
/// why this document cannot be imported.
///
/// # Errors
///
/// [`ExtractError::Spawn`] when the extraction thread cannot be started.
pub fn extract(
    bytes: &[u8],
    kind: DocumentType,
    limits: &ExtractLimits,
    read: &ReadLimits,
    token: &str,
) -> Result<Result<Extracted, ExtractRefusal>, ExtractError> {
    std::thread::scope(|scope| {
        let reading = std::thread::Builder::new()
            .name("oku-extract".to_owned())
            .stack_size(EXTRACT_STACK)
            .spawn_scoped(scope, || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| extract_on(bytes, kind, limits, read, token)))
            })
            .context(SpawnSnafu)?;
        Ok(match reading.join() {
            Ok(Ok(result)) => result,
            // The reader panicked: caught on its thread, or past the catch while unwinding.
            Ok(Err(_)) | Err(_) => Err(ExtractRefusal::Damaged),
        })
    })
}

/// The stack of the extraction thread: room for the parsers' bounded nesting.
const EXTRACT_STACK: usize = 8 << 20;

/// [`extract`] on the current thread.
fn extract_on(
    bytes: &[u8],
    kind: DocumentType,
    limits: &ExtractLimits,
    read: &ReadLimits,
    token: &str,
) -> Result<Extracted, ExtractRefusal> {
    #[cfg(test)]
    panic_probe(bytes);
    if count_u64(bytes.len()) > read.max_input_bytes {
        return Err(ExtractRefusal::TooLarge);
    }
    match kind {
        DocumentType::Markdown => text::markdown(bytes, limits),
        DocumentType::Text => text::plain(bytes, limits),
        DocumentType::Xlsx => xlsx::extract(bytes, limits, read),
        DocumentType::Ods => ods::extract(bytes, limits, read),
        DocumentType::Docx => docx::extract(bytes, limits, read, token),
        DocumentType::Pptx => pptx::extract(bytes, limits, read, token),
        DocumentType::Pdf => pdf::extract(bytes, limits, read, token),
    }
}

/// `body` between `open` and `close`, fitted to `max_text_bytes`: the text cut, if any, joins
/// `cuts`; `dropped` bytes the reader already left out join the text cut.
fn finish(
    open: &str,
    body: &str,
    close: &str,
    limits: &ExtractLimits,
    mut cuts: Vec<Cut>,
    dropped: u64,
) -> Result<Extracted, ExtractRefusal> {
    let (markdown, cut) =
        markdown::fit(open, body, close, limits.max_text_bytes, dropped).ok_or(ExtractRefusal::TooLarge)?;
    if let Some(cut) = cut {
        sheet::add_cut(&mut cuts, cut);
    }
    Ok(Extracted { title: None, markdown, images: Vec::new(), cuts })
}

/// Bytes on which `extract` panics in tests, to prove a panic becomes `Damaged`.
#[cfg(test)]
const PANIC_PROBE: &[u8] = b"oku-slides-extract panic probe";

/// Panics on [`PANIC_PROBE`], standing for a parser that panics on hostile bytes.
#[cfg(test)]
fn panic_probe(bytes: &[u8]) {
    assert!(bytes != PANIC_PROBE, "panic probe");
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
