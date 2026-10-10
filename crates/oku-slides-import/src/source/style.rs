//! Couleurs, provenance, remplissages et traits.

use serde::{Serialize, Serializer};

/// Couleur sRGB 8 bits avec opacité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    /// Rouge, 0..255.
    pub r: u8,
    /// Vert, 0..255.
    pub g: u8,
    /// Bleu, 0..255.
    pub b: u8,
    /// Opacité, 0 (transparent) à 255 (opaque).
    pub a: u8,
}

impl Rgba {
    /// Couleur opaque.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// `#rrggbb`, ou `#rrggbbaa` si elle n'est pas opaque.
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

/// Modificateur DrawingML appliqué à une couleur de thème ; valeurs en fraction (0.75 = 75 %).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "op", content = "value")]
pub enum ColorMod {
    /// Luminance multipliée (HSL).
    LumMod(f64),
    /// Luminance décalée (HSL).
    LumOff(f64),
    /// Saturation multipliée (HSL).
    SatMod(f64),
    /// Éclaircie vers le blanc (sRGB linéaire).
    Tint(f64),
    /// Assombrie vers le noir (sRGB linéaire).
    Shade(f64),
    /// Opacité.
    Alpha(f64),
}

/// D'où vient une valeur stylée.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Couleur ou police du thème.
    Scheme {
        /// Nom tel qu'écrit (`tx1`, `bg1`, `accent1`… ; `major`/`minor` pour une police).
        name: String,
        /// Emplacement du thème après `clrMap` (`dk1`, `lt1`, `accent1`… ; `major`/`minor`).
        slot: String,
        /// Modificateurs appliqués, dans l'ordre ; vide = la valeur du thème telle quelle.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        mods: Vec<ColorMod>,
    },
    /// Valeur écrite telle quelle dans le fichier (éventuellement modifiée).
    Explicit,
    /// Valeur par défaut du lecteur.
    Default,
}

impl Origin {
    /// Emplacement de thème écrit sous son propre nom, sans modificateur.
    pub fn scheme(slot: &str) -> Self {
        Origin::Scheme { name: slot.to_owned(), slot: slot.to_owned(), mods: Vec::new() }
    }

    /// Emplacement du thème (après `clrMap`), si la valeur en vient.
    pub fn slot(&self) -> Option<&str> {
        match self {
            Origin::Scheme { slot, .. } => Some(slot),
            Origin::Explicit | Origin::Default => None,
        }
    }
}

/// Une valeur résolue et sa provenance.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Styled<T> {
    /// Valeur résolue.
    pub value: T,
    /// Provenance.
    pub origin: Origin,
}

impl<T> Styled<T> {
    /// Valeur écrite telle quelle.
    pub fn explicit(value: T) -> Self {
        Self { value, origin: Origin::Explicit }
    }

    /// Valeur de l'emplacement `slot` du thème, écrit sous ce nom.
    pub fn scheme(value: T, slot: &str) -> Self {
        Self { value, origin: Origin::scheme(slot) }
    }

    /// Valeur du thème écrite `name`, lue dans `slot` après `clrMap`, modifiée par `mods`.
    pub fn mapped(value: T, name: &str, slot: &str, mods: Vec<ColorMod>) -> Self {
        Self { value, origin: Origin::Scheme { name: name.to_owned(), slot: slot.to_owned(), mods } }
    }

    /// Valeur par défaut du lecteur.
    pub fn default_value(value: T) -> Self {
        Self { value, origin: Origin::Default }
    }
}

/// Couleur résolue avec sa provenance.
pub type Color = Styled<Rgba>;

/// Arrêt de dégradé.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GradientStop {
    /// Position, 0..1.
    pub pos: f64,
    /// Couleur à cette position.
    pub color: Color,
}

/// Remplissage d'une forme, d'un fond ou d'une cellule.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Fill {
    /// Rien n'est peint.
    None,
    /// Couleur unie.
    Solid {
        /// Couleur.
        color: Color,
    },
    /// Dégradé linéaire.
    Gradient {
        /// Angle en degrés (0 = de gauche à droite).
        angle: f64,
        /// Arrêts (64 au plus).
        stops: Vec<GradientStop>,
    },
    /// Image étirée.
    Picture {
        /// Identifiant d'asset.
        asset: String,
    },
}

impl Fill {
    /// Vrai si rien n'est peint.
    pub fn is_none(&self) -> bool {
        matches!(self, Fill::None)
    }
}

/// Motif d'un trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Dash {
    /// Continu.
    Solid,
    /// Tirets.
    Dash,
    /// Points.
    Dot,
    /// Tiret-point.
    DashDot,
    /// Tirets longs.
    LongDash,
}

/// Trait (contour, bordure, connecteur).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stroke {
    /// Épaisseur en pt.
    pub width: f64,
    /// Couleur.
    pub color: Color,
    /// Motif.
    pub dash: Dash,
}

#[cfg(test)]
#[path = "style_tests.rs"]
mod style_tests;
