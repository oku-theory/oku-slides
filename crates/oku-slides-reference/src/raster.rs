//! Une image RGBA 8 bits, opaque, et son passage en PNG.

use snafu::ensure;

use crate::error::{PngSnafu, SizeSnafu};
use crate::{MAX_RASTER_SIDE, ReferenceError};

/// Une image RGBA 8 bits, ligne à ligne depuis le haut.
#[derive(Clone, PartialEq, Eq)]
pub struct Raster {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl std::fmt::Debug for Raster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Raster").field("width", &self.width).field("height", &self.height).finish_non_exhaustive()
    }
}

impl Raster {
    /// Une image de `width` × `height` pixels, `rgba` à 4 octets par pixel.
    ///
    /// Des octets d'une autre taille sont une erreur `Size`.
    pub(crate) fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ReferenceError> {
        ensure!(rgba.len() == width as usize * height as usize * 4, SizeSnafu { width, height });
        Ok(Self { width, height, rgba })
    }

    /// Largeur, en pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Hauteur, en pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Les octets RGBA.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Le rectangle `x, y, width, height` de l'image, borné à l'image.
    #[must_use]
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        let (x, y) = (x.min(self.width), y.min(self.height));
        let (width, height) = (width.min(self.width - x), height.min(self.height - y));
        let row = self.width as usize * 4;
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for r in y..y + height {
            let start = r as usize * row + x as usize * 4;
            rgba.extend_from_slice(&self.rgba[start..start + width as usize * 4]);
        }
        Self { width, height, rgba }
    }

    /// The image enlarged `factor` times, each pixel repeated as a square (nearest neighbor).
    ///
    /// The pixels stay those of the image: two images enlarged this way compare as
    /// before. A zero factor, or a side that would exceed [`MAX_RASTER_SIDE`], is a `Size` error;
    /// an empty image stays empty, with its sides enlarged.
    ///
    /// # Errors
    ///
    /// Fails (`Size`) if `factor` is zero or if an enlarged side exceeds [`MAX_RASTER_SIDE`].
    pub fn scaled(&self, factor: u32) -> Result<Self, ReferenceError> {
        let (width, height) = (self.width.saturating_mul(factor), self.height.saturating_mul(factor));
        ensure!(factor > 0, SizeSnafu { width, height });
        ensure!(width <= MAX_RASTER_SIDE && height <= MAX_RASTER_SIDE, SizeSnafu { width, height });
        if self.rgba.is_empty() {
            return Ok(Self { width, height, rgba: Vec::new() });
        }
        let (k, row) = (factor as usize, self.width as usize * 4);
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for line in self.rgba.chunks_exact(row) {
            let wide: Vec<u8> =
                line.as_chunks::<4>().0.iter().flat_map(|px| std::iter::repeat_n(*px, k)).flatten().collect();
            for _ in 0..k {
                rgba.extend_from_slice(&wide);
            }
        }
        Ok(Self { width, height, rgba })
    }

    /// Reads a PNG of at most [`MAX_RASTER_SIDE`] pixels per side.
    ///
    /// Accepts 8-bit RGB or RGBA; indexed or gray colors are expanded.
    ///
    /// # Errors
    ///
    /// Fails (`Png`) if the PNG is unreadable, of an unexpected color type or truncated,
    /// and (`Size`) if a side is zero or exceeds [`MAX_RASTER_SIDE`].
    pub fn from_png(bytes: &[u8]) -> Result<Self, ReferenceError> {
        let png_err = |e: png::DecodingError| ReferenceError::from(PngSnafu { message: e.to_string() }.build());
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().map_err(png_err)?;
        let (width, height) = (reader.info().width, reader.info().height);
        ensure!(
            (1..=MAX_RASTER_SIDE).contains(&width) && (1..=MAX_RASTER_SIDE).contains(&height),
            SizeSnafu { width, height }
        );
        let mut buf = vec![0; reader.output_buffer_size().unwrap_or_default()];
        let info = reader.next_frame(&mut buf).map_err(png_err)?;
        buf.truncate(info.buffer_size());
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf,
            png::ColorType::GrayscaleAlpha => buf.as_chunks::<2>().0.iter().flat_map(|&[g, a]| [g, g, g, a]).collect(),
            other => return Err(PngSnafu { message: format!("unexpected color type {other:?}") }.build().into()),
        };
        ensure!(rgba.len() == width as usize * height as usize * 4, PngSnafu { message: "truncated image" });
        Ok(Self { width, height, rgba })
    }

    /// The image as an 8-bit RGBA PNG.
    ///
    /// # Errors
    ///
    /// Fails (`Png`) if the encoder rejects the image, for example a byte count
    /// inconsistent with its sides.
    pub fn to_png(&self) -> Result<Vec<u8>, ReferenceError> {
        let png_err = |e: png::EncodingError| ReferenceError::from(PngSnafu { message: e.to_string() }.build());
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(png_err)?;
        writer.write_image_data(&self.rgba).map_err(png_err)?;
        writer.finish().map_err(png_err)?;
        Ok(out)
    }
}

#[cfg(test)]
#[path = "raster_tests.rs"]
mod raster_tests;
