//! Recadrage de l'aperçu d'un widget d'élément sur ce qu'il dessine.
//!
//! La boîte de l'élément d'aperçu est toute la grille : un petit widget y
//! serait un point. Deux PNG de la slide, avec et sans l'élément, sont
//! comparés dans cette boîte ; les pixels qui diffèrent bornent le dessin.

use oku_slides_render::RenderError;

use crate::widget_preview::PreviewBox;

/// Pixels par point des PNG comparés : 4 pt de précision suffisent.
pub(crate) const CROP_PIXEL_PER_PT: f32 = 0.25;

/// Une image décodée en octets par pixel (8 bits par canal).
struct Pixels {
    width: usize,
    height: usize,
    channels: usize,
    data: Vec<u8>,
}

impl Pixels {
    /// Décode un PNG ; `None` s'il est illisible.
    fn decode(bytes: &[u8]) -> Option<Self> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().ok()?;
        let mut data = vec![0; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut data).ok()?;
        let (width, height) = (usize::try_from(info.width).ok()?, usize::try_from(info.height).ok()?);
        let channels = info.color_type.samples();
        data.truncate(width * height * channels);
        (data.len() == width * height * channels).then_some(Self { width, height, channels, data })
    }

    /// Les octets du pixel `(x, y)`.
    fn at(&self, x: usize, y: usize) -> &[u8] {
        let i = (y * self.width + x) * self.channels;
        &self.data[i..i + self.channels]
    }
}

/// Les pixels de `within` (pt) qui changent de `bare` à `with`, en pt.
///
/// `None` si rien ne change (widget vide) ou si un PNG est illisible ou
/// d'une autre taille que l'autre : l'appelant garde `within`.
pub(crate) fn drawn_box(with: &[u8], bare: &[u8], within: PreviewBox, pixel_per_pt: f32) -> Option<PreviewBox> {
    let (drawn, blank) = (Pixels::decode(with)?, Pixels::decode(bare)?);
    if (drawn.width, drawn.height, drawn.channels) != (blank.width, blank.height, blank.channels) {
        return None;
    }
    let scale = f64::from(pixel_per_pt);
    // Bornes en pixels, rognées à l'image : le début arrondi en dessous, la
    // fin au-dessus (un dernier pixel couvert en partie compte).
    let px = |pt: f64, max: usize, round: fn(f64) -> f64| -> usize {
        let v = round(pt * scale).clamp(0.0, oku_slides_core::num::count_f64(max));
        // `v` est entier, positif et au plus `max` : la conversion est exacte.
        oku_slides_core::num::trunc_usize(v)
    };
    let x0 = px(within.x, drawn.width, f64::floor);
    let x1 = px(within.x + within.w, drawn.width, f64::ceil).max(x0);
    let y0 = px(within.y, drawn.height, f64::floor);
    let y1 = px(within.y + within.h, drawn.height, f64::ceil).max(y0);
    // Égalité exacte des octets : les deux PNG viennent du même rendu, à
    // l'élément près ; aucun pixel changé donne `None` (l'appelant garde `within`).
    let mut found: Option<(usize, usize, usize, usize)> = None;
    for y in y0..y1 {
        for x in x0..x1 {
            if drawn.at(x, y) != blank.at(x, y) {
                found = Some(match found {
                    None => (x, y, x, y),
                    Some((x_min, y_min, x_max, y_max)) => (x_min.min(x), y_min.min(y), x_max.max(x), y_max.max(y)),
                });
            }
        }
    }
    let (x_min, y_min, x_max, y_max) = found?;
    // Un pixel couvre `1 / scale` pt : la boîte va jusqu'au bord du dernier.
    let pt = |p: usize| oku_slides_core::num::count_f64(p) / scale;
    let (left, top) = (pt(x_min).max(within.x), pt(y_min).max(within.y));
    let (right, bottom) = (pt(x_max + 1).min(within.x + within.w), pt(y_max + 1).min(within.y + within.h));
    Some(PreviewBox { x: left, y: top, w: right - left, h: bottom - top })
}

/// La boîte de recadrage : le dessin de l'élément, sinon `cell`.
///
/// Au mieux : un PNG qui échoue (délai, mémoire, rendu périmé) garde `cell`.
pub(crate) fn crop_box(
    cell: PreviewBox,
    with: impl FnOnce() -> Result<Vec<u8>, RenderError>,
    bare: impl FnOnce() -> Result<Vec<u8>, RenderError>,
) -> PreviewBox {
    match with().and_then(|w| bare().map(|b| (w, b))) {
        Ok((w, b)) => drawn_box(&w, &b, cell, CROP_PIXEL_PER_PT).unwrap_or(cell),
        Err(e) => {
            tracing::debug!(error = %e, "aperçu non recadré, gardé sur la cellule");
            cell
        }
    }
}

#[cfg(test)]
#[path = "preview_crop_tests.rs"]
mod preview_crop_tests;
