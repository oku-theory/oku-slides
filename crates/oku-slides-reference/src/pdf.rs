//! Le PDF d'un témoin et la rastérisation de ses pages par `hayro`.

use std::sync::Arc;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings};
use snafu::ensure;

use crate::error::{NoPageSnafu, PdfSnafu, SizeSnafu};
use crate::{Raster, ReferenceError};

/// Côté maximal, en pixels, d'une page rastérisée.
pub const MAX_RASTER_SIDE: u32 = 4096;

/// Le PDF rendu par un témoin, lu une fois.
pub struct ReferencePdf {
    pdf: Pdf,
}

impl std::fmt::Debug for ReferencePdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferencePdf").field("pages", &self.page_count()).finish()
    }
}

impl ReferencePdf {
    /// Lit `bytes` ; un PDF illisible ou sans page est une erreur.
    pub fn new(bytes: Vec<u8>) -> Result<Self, ReferenceError> {
        let pdf = Pdf::new(Arc::new(bytes))
            .map_err(|e| ReferenceError::from(PdfSnafu { message: format!("{e:?}") }.build()))?;
        ensure!(!pdf.pages().is_empty(), PdfSnafu { message: "no page" });
        Ok(Self { pdf })
    }

    /// Nombre de pages.
    pub fn page_count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// La page `index` sur fond blanc, à `width` pixels de large (la hauteur
    /// suit le ratio de la page, arrondie).
    pub fn raster(&self, index: usize, width: u32) -> Result<Raster, ReferenceError> {
        let page = self.pdf.pages().get(index).ok_or_else(|| NoPageSnafu { index }.build())?;
        let (w, h) = page.render_dimensions();
        let scale = width as f32 / w;
        let height = (h * scale).round();
        let fits = |side: f32| side.is_finite() && side >= 1.0 && side <= MAX_RASTER_SIDE as f32;
        ensure!(fits(width as f32) && fits(height), SizeSnafu { width, height: height.max(0.0) as u32 });
        let height = height as u32;
        let settings = RenderSettings {
            x_scale: scale,
            y_scale: scale,
            width: u16::try_from(width).ok(),
            height: u16::try_from(height).ok(),
            bg_color: WHITE,
        };
        let pixmap = hayro::render(page, &RenderCache::new(), &InterpreterSettings::default(), &settings);
        // Fond blanc opaque : les composantes prémultipliées sont les composantes droites.
        Raster::new(width, height, pixmap.data_as_u8_slice().to_vec())
    }
}

#[cfg(test)]
#[path = "pdf_tests.rs"]
mod pdf_tests;
