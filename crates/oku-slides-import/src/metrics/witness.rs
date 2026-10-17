//! Les limites connues du témoin (office2pdf 0.8.1), lues sur l'item source d'une région.
//!
//! Là, son écart au rendu Oku est surtout celui du témoin.
//!
//! L'ajustement reprend les deux cas où office2pdf rétrécit lui-même un texte
//! `normAutofit` dont PowerPoint n'a enregistré ni `fontScale` ni
//! `lnSpcReduction` (`single_line_fit_paragraph`, `wrapped_fit_paragraph` de
//! son `render/typst_gen.rs`, Apache-2.0) ; PowerPoint, lui, le garde à 100 %.
//! Une liste à puces ou un texte de plusieurs paragraphes ancré en haut n'est
//! pas ajusté : son écart reste celui d'Oku.

use serde::Serialize;

use crate::source::{Anchor, Autofit, Bullet, Item, ItemKind, Paragraph, Spacing, TextBox};

/// Taille d'un run sans taille chez le témoin, plancher du plus grand corps.
const WITNESS_DEFAULT_SIZE: f64 = 12.0;

/// Le témoin de la référence : seules les limites d'office2pdf sont établies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Witness {
    /// office2pdf : ses régions [`WitnessLimit`] sont suspectes.
    Office2Pdf,
    /// Un autre témoin (LibreOffice) : aucune région n'est suspecte.
    Other,
}

/// Pourquoi le témoin dessine faux l'item d'une région (limite connue).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessLimit {
    /// Texte `normAutofit` sans échelle enregistrée, que le témoin rétrécit à sa boîte.
    ///
    /// PowerPoint le garde à 100 %.
    Autofit,
    /// Texte interlettré : le témoin le dessine plus large de 2 à 5 px.
    Tracking,
    /// Graphique : le témoin le dessine à peu près.
    Chart,
    /// Item d'un SmartArt dessiné : le témoin en rend le texte trop petit.
    SmartArt,
}

impl WitnessLimit {
    /// Nom de la limite, argument `limit` du message `import-region-suspect`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Autofit => "autofit",
            Self::Tracking => "tracking",
            Self::Chart => "chart",
            Self::SmartArt => "smart_art",
        }
    }
}

/// La limite du témoin sur `item`, enfant d'un SmartArt dessiné si `in_smartart`.
pub(super) fn witness_limit(item: &Item, in_smartart: bool) -> Option<WitnessLimit> {
    if in_smartart {
        return Some(WitnessLimit::SmartArt);
    }
    if matches!(item.kind, ItemKind::Chart(_)) {
        return Some(WitnessLimit::Chart);
    }
    let text = item.text()?;
    if self_fitted(text, item.frame.h) {
        return Some(WitnessLimit::Autofit);
    }
    let tracked = text.paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.style.char_spacing != 0.0);
    tracked.then_some(WitnessLimit::Tracking)
}

/// Le témoin rétrécit-il lui-même `text`, dans une boîte haute de `height` pt ?
fn self_fitted(text: &TextBox, height: f64) -> bool {
    // `normAutofit` nu : le lecteur le lit à 100 % et 0 %.
    let Autofit::Shrink { font_scale, line_reduction } = text.autofit else { return false };
    if font_scale < 1.0 || line_reduction > 0.0 {
        return false;
    }
    // Un texte ancré au milieu ou en bas, à la ligne, est ramené à sa boîte.
    if text.wrap && text.anchor != Anchor::Top {
        return true;
    }
    // Un seul paragraphe sans puce ni saut de ligne, dans une boîte courte.
    let [p] = text.paragraphs.as_slice() else { return false };
    let breaks = p.runs.iter().any(|r| r.text.contains(['\n', '\r', '\u{b}']));
    if p.runs.is_empty() || breaks || !matches!(p.style.bullet, Bullet::None) {
        return false;
    }
    let inner = height - text.insets.t - text.insets.b;
    let max = max_size(p);
    if inner <= 0.0 {
        return false;
    }
    if mixed_sizes(p) && inner <= max * 2.5 {
        return true;
    }
    inner <= line_height(p, max) * 2.0
}

/// Le plus grand corps du paragraphe, au moins [`WITNESS_DEFAULT_SIZE`].
fn max_size(p: &Paragraph) -> f64 {
    p.runs.iter().map(|r| r.style.size).fold(WITNESS_DEFAULT_SIZE, f64::max)
}

/// Les runs du paragraphe ont-ils plusieurs corps (au centième de pt) ?
fn mixed_sizes(p: &Paragraph) -> bool {
    let key = |size: f64| (size * 100.0).round() as i64;
    p.runs.windows(2).any(|w| key(w[0].style.size) != key(w[1].style.size))
}

/// La hauteur d'une ligne que le témoin estime, pour un corps `max`.
fn line_height(p: &Paragraph, max: f64) -> f64 {
    let default = max * 1.2;
    match p.style.line_spacing {
        Spacing::Points(points) => default.max(points),
        Spacing::Percent(factor) => default.max(max * factor),
    }
}

#[cfg(test)]
#[path = "witness_tests.rs"]
mod witness_tests;
