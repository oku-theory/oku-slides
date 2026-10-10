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

/// Face embarquée lisible (sfnt : TrueType ou OpenType).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontFace {
    /// Graisse et style.
    pub style: FaceStyle,
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

/// Contenu d'un asset (image) ; la clé de la table est son `asset_id`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Asset {
    /// Type, déduit de l'extension (à vérifier avant tout usage : octets non fiables).
    pub mime: String,
    /// Octets.
    #[serde(rename = "len", serialize_with = "len_of")]
    pub bytes: Vec<u8>,
}
