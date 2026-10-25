//! Les limites connues des témoins (office2pdf 0.8.1, LibreOffice 26.8), lues sur l'item source d'une région.
//!
//! Là, son écart au rendu Oku est surtout celui du témoin.
//!
//! office2pdf. L'ajustement reprend les deux cas où office2pdf rétrécit lui-même un texte
//! `normAutofit` dont PowerPoint n'a enregistré ni `fontScale` ni
//! `lnSpcReduction` (`single_line_fit_paragraph`, `wrapped_fit_paragraph` de
//! son `render/typst_gen.rs`, Apache-2.0) ; PowerPoint, lui, le garde à 100 %.
//! Une liste à puces ou un texte de plusieurs paragraphes ancré en haut n'est
//! pas ajusté : son écart reste celui d'Oku.
//!
//! La police (t-904) : du `defaultTextStyle` de la présentation, office2pdf ne
//! lit que la taille (`parse_default_text_size_pt`, `parser/pptx_package.rs:348`) ;
//! une forme ordinaire n'hérite que de l'`otherStyle` du master
//! (`parser/pptx_slides.rs:471`). Un run dont seule la présentation nomme la
//! police (`+mn-lt`, Calibri) reste sans famille (`font_family: None`), et son
//! Typst n'en écrit aucune (`#set text(size: 16pt)`, kitchen p. 2) : le texte
//! sort dans la police de repli de Typst, plus étroite. PowerPoint, lui, le
//! dessine dans la police du corps du thème que nomme `defaultTextStyle`.
//!
//! LibreOffice (t-905, mesuré le 2026-10-18 sur alto, kitchen et libreoffice).
//! Sa table de substitution (`VCL.xcu`, `main.xcd`) donne Carlito pour
//! Calibri, mais rien pour « Calibri Light », la police des titres des thèmes
//! Office : il les dessine dans un repli à empattement (titres de kitchen et
//! libreoffice, 14,7 à 29 contre 3,4 à 5,7 sous office2pdf), PowerPoint et Oku
//! (Carlito) sans. Un même `drawing1.xml` de SmartArt, sans style ni `rPr`,
//! est dessiné comme Oku quand le `defaultTextStyle` de la présentation nomme
//! la couleur et la police du texte (kitchen p. 3 : 1,7 et 3,5), mais en blanc
//! et dans sa police par défaut quand aucune couche ne les nomme (libreoffice
//! p. 3 : 21 ; t-936, mesuré en ajoutant ces défauts : 2,6). Ses autres écarts ne se lisent
//! pas dans la source et ne sont pas marqués : il rétrécit un `normAutofit`
//! nu seulement s'il déborde (alto p. 8, listes des piliers 2 et 3), ce que
//! seule une mesure du texte dirait. Ses graphiques, eux, suivent l'OOXML (axes
//! `delete`, pas de légende) : leur écart est celui d'Oku.

use serde::Serialize;

use crate::source::{Anchor, Autofit, Bullet, Item, ItemKind, Paragraph, Spacing, TextBox};
use oku_slides_core::num::round_i64;

/// Taille d'un run sans taille chez le témoin, plancher du plus grand corps.
const WITNESS_DEFAULT_SIZE: f64 = 12.0;

/// Le témoin de la référence, dont les limites mesurées rendent des régions suspectes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Witness {
    /// office2pdf : ajustement, interlettrage, graphiques, SmartArt, police des défauts.
    Office2Pdf,
    /// LibreOffice (`soffice`) : police sans substitut, style du texte des SmartArt (t-905).
    LibreOffice,
}

/// Familles qu'Oku remplace et LibreOffice non, en minuscules.
///
/// Celles de [`SUBSTITUTES`](crate::fonts::SUBSTITUTES) absentes de sa table de substitution, mesuré dans `VCL.xcu` (`main.xcd` de LibreOffice 26.8) : il remplace toutes les autres. Une
/// famille ajoutée à [`SUBSTITUTES`](crate::fonts::SUBSTITUTES) est donc supposée remplacée, jusqu'à mesure contraire.
const LIBREOFFICE_UNSUBSTITUTED: [&str; 1] = ["calibri light"];

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
    /// Texte dont la police n'est nommée que par les défauts, ou par aucune couche (t-904).
    ///
    /// Le témoin le dessine dans sa police de repli, pas dans celle du thème.
    DefaultFont,
    /// Texte dans une famille qu'Oku remplace et que le témoin ne sait pas remplacer (t-905).
    ///
    /// « Calibri Light » : LibreOffice la dessine dans un repli à empattement.
    NoSubstitute,
    /// Texte d'un SmartArt dessiné : le témoin le dessine dans son propre style (t-905).
    ///
    /// Seulement si aucune couche, défauts de la présentation compris, ne nomme sa couleur
    /// ou sa police latine (t-936).
    SmartArtStyle,
}

impl WitnessLimit {
    /// Nom de la limite, argument `limit` du message `import-region-suspect`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Autofit => "autofit",
            Self::Tracking => "tracking",
            Self::Chart => "chart",
            Self::SmartArt => "smart_art",
            Self::DefaultFont => "default_font",
            Self::NoSubstitute => "no_substitute",
            Self::SmartArtStyle => "smart_art_style",
        }
    }
}

/// La limite de `witness` sur `item`, enfant d'un SmartArt dessiné si `in_smartart`.
pub(super) fn witness_limit(item: &Item, in_smartart: bool, witness: Witness) -> Option<WitnessLimit> {
    match witness {
        Witness::Office2Pdf => office2pdf_limit(item, in_smartart),
        Witness::LibreOffice => libreoffice_limit(item, in_smartart),
    }
}

/// La limite de LibreOffice sur `item`.
///
/// Dans un SmartArt dessiné, il garde le style d'un texte qui nomme sa couleur et sa police
/// (kitchen p. 3 : 1,7 et 3,5) ; sinon il y met le sien, blanc (libreoffice p. 3 : 21, puis 2,6
/// avec un `defaultTextStyle` qui nomme les deux, 8 avec la couleur seule ; mesuré le 2026-10-18).
fn libreoffice_limit(item: &Item, in_smartart: bool) -> Option<WitnessLimit> {
    let text = item.text()?;
    let written = || text.paragraphs.iter().flat_map(|p| &p.runs).filter(|r| !r.text.trim().is_empty());
    if in_smartart && written().any(|r| r.style.style_from_reader) {
        return Some(WitnessLimit::SmartArtStyle);
    }
    let unmapped = written().any(|r| !libreoffice_substitutes(&r.style.font.value));
    unmapped.then_some(WitnessLimit::NoSubstitute)
}

/// LibreOffice remplace-t-il `family` comme Oku, ou Oku la garde-t-il telle quelle ?
///
/// Faux pour une famille de [`LIBREOFFICE_UNSUBSTITUTED`], comparée en minuscules.
fn libreoffice_substitutes(family: &str) -> bool {
    !LIBREOFFICE_UNSUBSTITUTED.contains(&family.to_lowercase().as_str())
}

/// La limite d'office2pdf sur `item`.
fn office2pdf_limit(item: &Item, in_smartart: bool) -> Option<WitnessLimit> {
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
    let runs = || text.paragraphs.iter().flat_map(|p| &p.runs);
    if runs().any(|r| r.style.char_spacing != 0.0) {
        return Some(WitnessLimit::Tracking);
    }
    let defaulted = runs().any(|r| r.style.font_from_defaults && !r.text.trim().is_empty());
    defaulted.then_some(WitnessLimit::DefaultFont)
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
    let key = |size: f64| round_i64(size * 100.0);
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
