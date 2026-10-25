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
    /// Reads `bytes`; an unreadable PDF or one without pages is an error.
    ///
    /// # Errors
    ///
    /// Fails (`Pdf`) if `bytes` is not a readable PDF or has no page.
    pub fn new(bytes: Vec<u8>) -> Result<Self, ReferenceError> {
        let pdf = Pdf::new(Arc::new(bytes))
            .map_err(|e| ReferenceError::from(PdfSnafu { message: format!("{e:?}") }.build()))?;
        ensure!(!pdf.pages().is_empty(), PdfSnafu { message: "no page" });
        Ok(Self { pdf })
    }

    /// Nombre de pages.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// Page `index` on a white background, `width` pixels wide.
    ///
    /// The height follows the page ratio, rounded.
    ///
    /// # Errors
    ///
    /// Fails if page `index` does not exist (`NoPage`), or if a side of the image
    /// is zero or exceeds [`MAX_RASTER_SIDE`] (`Size`).
    pub fn raster(&self, index: usize, width: u32) -> Result<Raster, ReferenceError> {
        let page = self.pdf.pages().get(index).ok_or_else(|| NoPageSnafu { index }.build())?;
        let (w, h) = page.render_dimensions();
        let scale = side_f32(width) / w;
        let height = (h * scale).round();
        let fits = |side: f32| side.is_finite() && side >= 1.0 && side <= side_f32(MAX_RASTER_SIDE);
        ensure!(fits(side_f32(width)) && fits(height), SizeSnafu { width, height: pixels(height) });
        let height = pixels(height);
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

/// A side in pixels, as `f32` for the scale computation.
#[expect(
    clippy::cast_precision_loss,
    reason = "exact up to 2^24; a larger side exceeds MAX_RASTER_SIDE and is rejected"
)]
fn side_f32(px: u32) -> f32 {
    px as f32
}

/// A rounded side in pixels; negative or NaN gives 0, too large gives `u32::MAX`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "intended saturating conversion: called on an already rounded side, then bounded by Size"
)]
fn pixels(side: f32) -> u32 {
    side as u32
}

#[cfg(test)]
#[path = "pdf_tests.rs"]
mod pdf_tests;
