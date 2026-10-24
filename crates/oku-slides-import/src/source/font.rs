//! Polices utilisées et embarquées, assets binaires.

use serde::{Serialize, Serializer};

/// Sérialise la longueur d'un contenu binaire, pas son contenu.
fn len_of<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(data.len() as u64)
}

/// Graisse et style d'une face embarquée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceStyle {
    /// Normal.
    Regular,
    /// Gras.
    Bold,
    /// Italique.
    Italic,
    /// Gras italique.
    BoldItalic,
}

/// Graisse que nomme le suffixe d'une famille (« Segoe UI Semibold » → 600, « Calibri Light »
/// → 300) ; `None` sans suffixe de graisse. Un suffixe `Italic`/`Oblique` final est ignoré.
pub(crate) fn family_weight(family: &str) -> Option<u16> {
    let words: Vec<String> = family
        .split([' ', '-', '_'])
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .filter(|w| !matches!(w.as_str(), "italic" | "oblique"))
        .collect();
    let [.., before, last] = words.as_slice() else { return None };
    let joined = format!("{before}{last}");
    let named = |w: &str| match w {
        "thin" | "hairline" => Some(100),
        "extralight" | "ultralight" => Some(200),
        "light" => Some(300),
        "medium" => Some(500),
        "semibold" | "demibold" | "demi" => Some(600),
        "bold" => Some(700),
        "extrabold" | "ultrabold" => Some(800),
        "black" | "heavy" => Some(900),
        _ => None,
    };
    // « Semi Bold », « Extra Light » : modificateur et graisse en deux mots.
    named(&joined).filter(|_| words.len() > 2).or_else(|| named(last))
}

/// Face embarquée lisible (sfnt : TrueType ou OpenType).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontFace {
    /// Graisse et style.
    pub style: FaceStyle,
    /// Graisse numérique (`usWeightClass` de `OS/2`, ramenée à 100–900) ; `None` sans `OS/2`.
    pub weight: Option<u16>,
    /// Droits d'incorporation (`fsType` OS/2) ; `None` s'ils ne se lisent pas.
    pub fs_type: Option<u16>,
    /// La présentation n'embarque que les glyphes utilisés (`saveSubsetFonts`).
    pub subset: bool,
    /// Données sfnt (octets non fiables).
    #[serde(rename = "len", serialize_with = "len_of")]
    pub data: Vec<u8>,
}

/// Famille utilisée par le texte du deck ; `faces` vide si rien n'est embarqué.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceFont {
    /// Nom de famille.
    pub family: String,
    /// Faces embarquées lisibles.
    pub faces: Vec<FontFace>,
}

/// Types MIME que le rendu sait dessiner.
const DRAWABLE: [&str; 5] = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"];

/// Vrai si le rendu sait dessiner une image de type `mime`.
pub(crate) fn is_drawable(mime: &str) -> bool {
    DRAWABLE.contains(&mime)
}

/// Contenu d'un asset (image) ; la clé de la table est son `asset_id`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Asset {
    /// Type réel, lu dans les premiers octets (`oku_slides_core::media::sniff`) ; un SVG est déjà assaini.
    pub mime: String,
    /// Octets.
    #[serde(rename = "len", serialize_with = "len_of")]
    pub bytes: Vec<u8>,
}

impl Asset {
    /// Vrai si le rendu sait dessiner cet asset (png, jpeg, gif, webp, svg).
    pub(crate) fn is_drawable(&self) -> bool {
        is_drawable(&self.mime)
    }
}

#[cfg(test)]
#[path = "font_tests.rs"]
mod font_tests;
