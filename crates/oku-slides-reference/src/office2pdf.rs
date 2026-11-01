//! The `office2pdf` witness: the presentation as PDF through the crate of that
//! name (pure Rust, on Typst 0.15), without LibreOffice.
//!
//! The given fonts come before all others; system fonts stay a fallback (the
//! crate always looks for them, outside WASM). Hidden slides are rendered: the
//! import keeps them visible, one page per slide. The `.pptx`'s embedded fonts
//! (`ppt/fonts/` and any `.fntdata` part) are stripped before the conversion:
//! `office2pdf` would write them to the system temp dir, where a killed render
//! process would leave them; the caller already passes them in memory.
//! `office2pdf` 0.8.1 also named that file after the `typeface` of
//! `<p:embeddedFont><p:font>`, refusing neither `../` nor an absolute path
//! (t-898); 0.10 names it by position, and the filter stays as defense in depth.

use std::io::Cursor;

use office2pdf::config::{ConvertOptions, Format, SlideRange};

use crate::error::RenderSnafu;
use crate::{ReferenceError, ReferencePdf, ReferenceRenderer, SlideSpan};

/// Name and version of the `office2pdf` witness, as the report writes them.
///
/// The version is the one `Cargo.toml` pins (`=0.10.0`): a test checks it.
pub const WITNESS: &str = "office2pdf 0.10.0";

/// Le dossier des polices embarquées d'un `.pptx`.
const FONTS_DIR: &str = "ppt/fonts/";

/// L'extension d'une police embarquée, la seule qu'`office2pdf` lise (`{GUID}.fntdata`).
const FONT_EXTENSION: &[u8] = b".fntdata";

/// Le témoin `office2pdf`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Office2Pdf;

impl ReferenceRenderer for Office2Pdf {
    fn witness(&self) -> &str {
        WITNESS
    }

    fn render(
        &self,
        pptx: &[u8],
        fonts: Vec<Vec<u8>>,
        slides: Option<SlideSpan>,
    ) -> Result<ReferencePdf, ReferenceError> {
        let render_err = |message: String| ReferenceError::from(RenderSnafu { message }.build());
        let pptx = without_fonts(pptx).map_err(|e| render_err(e.to_string()))?;
        ensure_no_font_left(&pptx).map_err(render_err)?;
        let options = ConvertOptions {
            include_hidden_slides: true,
            font_bytes: fonts,
            slide_range: slides.map(|s| SlideRange::new(s.first, s.last)),
            ..Default::default()
        };
        let converted =
            office2pdf::convert_bytes(&pptx, Format::Pptx, &options).map_err(|e| render_err(e.to_string()))?;
        ReferencePdf::new(converted.pdf)
    }
}

/// `pptx` réécrit sans ses polices embarquées, les autres parts copiées telles quelles.
fn without_fonts(pptx: &[u8]) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut source = zip::ZipArchive::new(Cursor::new(pptx))?;
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::with_capacity(pptx.len())));
    for i in 0..source.len() {
        let entry = source.by_index_raw(i)?;
        if !is_font(&entry.name()?) {
            out.raw_copy_file(entry)?;
        }
    }
    Ok(out.finish()?.into_inner())
}

/// Refuses `pptx` if, reread, the rewritten archive still holds a font.
///
/// An assertion that `without_fonts` left nothing, made with the zip 9 that
/// `office2pdf` shares with the workspace (a test checks it in `Cargo.lock`).
fn ensure_no_font_left(pptx: &[u8]) -> Result<(), String> {
    let archive = zip::ZipArchive::new(Cursor::new(pptx)).map_err(|e| e.to_string())?;
    for name in archive.file_names() {
        let name = name.map_err(|e| e.to_string())?;
        if is_font(&name) {
            return Err(format!("an embedded font is left for office2pdf: {name}"));
        }
    }
    Ok(())
}

/// Vrai si la part `name` est une police embarquée, où qu'elle soit dans l'archive.
///
/// Seule l'extension protège (`office2pdf` ne lit que `{GUID}.fntdata`, à la
/// casse près) ; `ppt/fonts/` est ôté pour ne rien laisser au dossier temporaire.
fn is_font(name: &str) -> bool {
    let name = name.as_bytes();
    let font_file = name.len() >= FONT_EXTENSION.len()
        && name[name.len() - FONT_EXTENSION.len()..].eq_ignore_ascii_case(FONT_EXTENSION);
    font_file || name.starts_with(FONTS_DIR.as_bytes())
}

#[cfg(test)]
#[path = "office2pdf_tests.rs"]
pub(crate) mod office2pdf_tests;
