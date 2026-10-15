//! Le témoin `office2pdf` : la présentation en PDF par le crate du même nom
//! (Rust pur, sur Typst 0.15), sans LibreOffice.
//!
//! Les polices fournies passent avant toutes les autres ; celles du système
//! restent un repli (le crate les cherche toujours, hors WASM). Les slides
//! masquées sont rendues : l'import les reprend visibles, une page par slide.
//! Les polices embarquées du `.pptx` (`ppt/fonts/`) sont ôtées avant la
//! conversion : `office2pdf` les écrirait dans le dossier temporaire du
//! système, et un processus de rendu tué les y laisserait ; l'appelant les
//! fournit déjà en mémoire.

use std::io::Cursor;

use office2pdf::config::{ConvertOptions, Format, SlideRange};

use crate::error::RenderSnafu;
use crate::{ReferenceError, ReferencePdf, ReferenceRenderer, SlideSpan};

/// Nom et version du témoin `office2pdf`, tels que le rapport les écrit.
///
/// La version est celle que `Cargo.toml` épingle (`=0.8.1`) : un test le vérifie.
pub const WITNESS: &str = "office2pdf 0.8.1";

/// Le dossier des polices embarquées d'un `.pptx`.
const FONTS_DIR: &str = "ppt/fonts/";

/// Le témoin `office2pdf`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Office2Pdf;

impl ReferenceRenderer for Office2Pdf {
    fn witness(&self) -> &'static str {
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

/// `pptx` sans ses parts de `ppt/fonts/`, les autres copiées telles quelles.
fn without_fonts(pptx: &[u8]) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut source = zip::ZipArchive::new(Cursor::new(pptx))?;
    if !source.file_names().any(|n| n.starts_with(FONTS_DIR)) {
        return Ok(pptx.to_vec());
    }
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..source.len() {
        let entry = source.by_index_raw(i)?;
        if !entry.name().starts_with(FONTS_DIR) {
            out.raw_copy_file(entry)?;
        }
    }
    Ok(out.finish()?.into_inner())
}

#[cfg(test)]
#[path = "office2pdf_tests.rs"]
mod office2pdf_tests;
