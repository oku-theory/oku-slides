//! Le témoin `office2pdf` : la présentation en PDF par le crate du même nom
//! (Rust pur, sur Typst 0.15), sans LibreOffice.
//!
//! Les polices fournies passent avant toutes les autres ; celles du système
//! restent un repli (le crate les cherche toujours, hors WASM). Les slides
//! masquées sont rendues : l'import les reprend visibles, une page par slide.
//! Les polices embarquées du `.pptx` (`ppt/fonts/` et toute part `.fntdata`)
//! sont ôtées avant la conversion : `office2pdf` les écrirait dans le dossier
//! temporaire du système, et un processus de rendu tué les y laisserait ;
//! l'appelant les fournit déjà en mémoire. Surtout, `office2pdf` 0.8.1 nomme
//! ce fichier d'après le `typeface` de `<p:embeddedFont><p:font>` sans refuser `../`
//! ni un chemin absolu : une police embarquée y serait une écriture
//! arbitraire (t-898).

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
        ensure_no_font_for_office2pdf(&pptx).map_err(render_err)?;
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
///
/// L'archive est toujours réécrite : `office2pdf` la relit avec `zip` 0.6, qui
/// ignore le nom Unicode (extra field 0x7075) que `zip` 9 préfère à celui de
/// l'en-tête ; réécrite, chaque part ne porte plus que le nom que ce filtre a vu.
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

/// Refuse `pptx` si `zip` 0.6, le lecteur d'`office2pdf`, y voit encore une police.
///
/// Les deux lecteurs `zip` peuvent lire une archive différemment ; ce contrôle
/// vaut pour celle qu'`office2pdf` lira, quelle que soit la différence.
fn ensure_no_font_for_office2pdf(pptx: &[u8]) -> Result<(), String> {
    let archive = zip_office2pdf::ZipArchive::new(Cursor::new(pptx)).map_err(|e| e.to_string())?;
    match archive.file_names().find(|n| is_font(n)) {
        Some(name) => Err(format!("an embedded font is left for office2pdf: {name}")),
        None => Ok(()),
    }
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
