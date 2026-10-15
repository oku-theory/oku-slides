//! La page Oku d'une source : 1920 × 1080 pt, quelle que soit la page source.
//!
//! Le thème généré garde la page du rendu ; la source y est mise à l'échelle
//! uniformément (`min(1920 / w, 1080 / h)`) et centrée. Une source 16:9
//! (960 × 540 pt, celle de PowerPoint) la remplit exactement : échelle 2,
//! aucun décalage. Une autre (4:3, A4…) laisse deux bandes, et la perte
//! [`LossKind::PageFitted`] le dit. Les marges du thème généré sont nulles :
//! la zone de corps est la page, les cadres sont en pt page.
//!
//! Les longueurs produites sont arrondies au centième de pt : les plans et
//! leurs instantanés restent stables d'une plateforme à l'autre.

use crate::report::LossKind;
use crate::source::{Frame, Size};

/// Largeur de la page Oku, en pt.
pub(crate) const PAGE_W: f64 = 1920.0;
/// Hauteur de la page Oku, en pt.
pub(crate) const PAGE_H: f64 = 1080.0;

/// Décalage de centrage sous lequel la source remplit la page (pt page).
const FITTED_MIN: f64 = 0.5;

/// Arrondi au centième ; `-0` devient `0`.
pub(crate) fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0 + 0.0
}

/// Transposition d'une page source sur la page Oku.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Page {
    /// Taille de la page source, en pt.
    source: Size,
    /// pt page par pt source.
    scale: f64,
    /// Décalage horizontal de centrage, en pt page.
    dx: f64,
    /// Décalage vertical de centrage, en pt page.
    dy: f64,
}

impl Page {
    /// La transposition de la page source `size`. Une taille non finie ou
    /// nulle (le lecteur la borne à `[1, 10⁶]`) vaut la page Oku.
    pub(crate) fn new(size: Size) -> Self {
        let side = |v: f64, page: f64| if v.is_finite() && v > 0.0 { v } else { page };
        let (w, h) = (side(size.w, PAGE_W), side(size.h, PAGE_H));
        let scale = (PAGE_W / w).min(PAGE_H / h);
        Self { source: Size { w, h }, scale, dx: (PAGE_W - w * scale) / 2.0, dy: (PAGE_H - h * scale) / 2.0 }
    }

    /// Une longueur source (taille de police, trait, rayon) en pt page.
    pub(crate) fn len(&self, pt: f64) -> f64 {
        round2(pt * self.scale)
    }

    /// Le cadre Oku d'une boîte source (rotation et miroirs ne s'y écrivent pas).
    pub(crate) fn frame(&self, f: &Frame) -> oku_slides_core::Frame {
        oku_slides_core::Frame {
            x: round2(self.dx + f.x * self.scale),
            y: round2(self.dy + f.y * self.scale),
            w: self.len(f.w),
            h: self.len(f.h),
        }
    }

    /// La page source sur la page Oku, en pt page : là où le rendu Oku se
    /// compare au rendu de référence.
    pub(crate) fn source_rect(&self) -> oku_slides_core::Frame {
        self.frame(&Frame::new(0.0, 0.0, self.source.w, self.source.h))
    }

    /// La boîte source que la page Oku entière recouvre (bandes de centrage
    /// comprises) : celle du fond propre d'une slide.
    pub(crate) fn cover(&self) -> Frame {
        Frame::new(-self.dx / self.scale, -self.dy / self.scale, PAGE_W / self.scale, PAGE_H / self.scale)
    }

    /// La perte à signaler si la source ne remplit pas la page.
    pub(crate) fn loss(&self) -> Option<LossKind> {
        (self.dx >= FITTED_MIN || self.dy >= FITTED_MIN)
            .then(|| LossKind::PageFitted { width: pt_u32(self.source.w), height: pt_u32(self.source.h) })
    }
}

/// Une taille de page en pt entiers (`as` sature ; le lecteur la borne à `[1, 10⁶]`).
fn pt_u32(v: f64) -> u32 {
    v.round() as u32
}

#[cfg(test)]
#[path = "page_tests.rs"]
mod page_tests;
