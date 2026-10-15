//! Couleurs dans l'espace OKLab et leur écart ΔE, partagés par le retrofit et
//! les métriques.
//!
//! Module neutre : il ne dépend que de [`Rgba`] de la représentation source.

use crate::source::Rgba;

/// Une couleur dans l'espace OKLab (Björn Ottosson, 2020).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Oklab {
    /// Clarté, 0 (noir) à 1 (blanc).
    pub l: f64,
    /// Axe vert (négatif) – rouge (positif).
    pub a: f64,
    /// Axe bleu (négatif) – jaune (positif).
    pub b: f64,
}

impl Oklab {
    /// Chroma : distance à l'axe des gris.
    pub(crate) fn chroma(&self) -> f64 {
        self.a.hypot(self.b)
    }
}

/// Composante sRGB 8 bits → linéaire.
pub(crate) fn linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// `c` en OKLab (opacité ignorée).
pub(crate) fn oklab(c: Rgba) -> Oklab {
    let (r, g, b) = (linear(c.r), linear(c.g), linear(c.b));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    Oklab {
        l: 0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        a: 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        b: 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    }
}

/// Écart entre deux couleurs : distance euclidienne OKLab, ×100.
pub(crate) fn delta_e(x: Oklab, y: Oklab) -> f64 {
    100.0 * ((x.l - y.l).powi(2) + (x.a - y.a).powi(2) + (x.b - y.b).powi(2)).sqrt()
}

#[cfg(test)]
#[path = "oklab_tests.rs"]
mod oklab_tests;
