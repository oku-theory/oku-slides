//! La géométrie de la comparaison d'un import hi-fi.
//!
//! La page source posée sur la page Oku, ses pixels, et les rognures d'une région.
//!
//! La page source fait [`REFERENCE_WIDTH`] pixels de large ; la page Oku est
//! rendue à la même échelle ([`Geometry::ppp`]) puis rognée à la page source.
//! Les bords d'une région sont signés : un cadre qui commence avant la page
//! (ou finit après) est borné à la page sans s'élargir ([`crop_box`]).

use oku_slides_core::Frame;
use oku_slides_import::hifi;
use oku_slides_import::metrics::Image;
use oku_slides_import::source::SourceDeck;
use oku_slides_reference::Raster;

use super::REFERENCE_WIDTH;

/// La géométrie de la comparaison : la page source sur la page Oku.
pub(crate) struct Geometry {
    /// La page source, en pt de la page Oku.
    pub(crate) rect: Frame,
    /// Pixels par pt de la page Oku : la page source y fait `REFERENCE_WIDTH` pixels.
    pub(crate) ppp: f64,
    /// Largeur de la page source, en pixels.
    pub(crate) width: u32,
    /// Pixels par pt de la page source.
    pub(crate) scale: f64,
}

impl Geometry {
    /// La géométrie de la présentation `source`.
    pub(crate) fn of(source: &SourceDeck) -> Self {
        let rect = hifi::source_rect(source);
        let ppp = f64::from(REFERENCE_WIDTH) / rect.w;
        let width = px(rect.w, ppp);
        Self { scale: f64::from(width) / source.size.w, rect, ppp, width }
    }

    /// Les bords `[gauche, haut, droite, bas]` de `frame`, en pixels de la page source.
    ///
    /// `frame` est en pt de la page Oku.
    ///
    /// Signés : un cadre qui déborde de la page source garde ses bords hors d'elle.
    pub(crate) fn edges(&self, frame: Frame) -> [f64; 4] {
        let (x, y) = ((frame.x - self.rect.x) * self.ppp, (frame.y - self.rect.y) * self.ppp);
        [x, y, x + frame.w * self.ppp, y + frame.h * self.ppp]
    }
}

/// `v` pt en pixels entiers, à `ppp` pixels par pt.
pub(crate) fn px(v: f64, ppp: f64) -> u32 {
    // Borné à zéro, puis arrondi : un cadre fini et borné de la page tient dans `u32`.
    (v * ppp).round().max(0.0) as u32
}

/// La vue de `r` que lit [`oku_slides_import::metrics::score`].
pub(crate) fn image(r: &Raster) -> Image<'_> {
    Image { width: r.width(), height: r.height(), rgba: r.rgba() }
}

/// La rognure `[x, y, w, h]` des bords `edges`, à `margin` pixels autour.
///
/// Les bords sont des pixels signés ; la rognure est bornée à une page de `width` × `height` pixels.
///
/// Bornée à la page, et d'un pixel au moins : un cadre hors de la page montre
/// son bord. `None` pour une page vide.
pub(crate) fn crop_box([left, top, right, bottom]: [f64; 4], margin: u32, width: u32, height: u32) -> Option<[u32; 4]> {
    if width == 0 || height == 0 {
        return None;
    }
    let m = f64::from(margin);
    // Arrondi puis borné à la page : la valeur tient dans `u32`.
    let at = |v: f64, max: u32| v.round().clamp(0.0, f64::from(max)) as u32;
    let (x0, y0) = (at(left - m, width - 1), at(top - m, height - 1));
    let (x1, y1) = (at(right + m, width).max(x0 + 1), at(bottom + m, height).max(y0 + 1));
    Some([x0, y0, x1 - x0, y1 - y0])
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod geometry_tests;
