//! Texte résolu : boîte, paragraphes, runs, styles par niveau.

use serde::Serialize;

use super::lang::Script;
use super::style::{Color, Styled};

/// Marges intérieures d'une boîte de texte, en pt.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Insets {
    /// Gauche.
    pub l: f64,
    /// Haut.
    pub t: f64,
    /// Droite.
    pub r: f64,
    /// Bas.
    pub b: f64,
}

impl Default for Insets {
    /// Défauts OOXML : 7,2 pt à gauche et à droite, 3,6 pt en haut et en bas.
    fn default() -> Self {
        Self { l: 7.2, t: 3.6, r: 7.2, b: 3.6 }
    }
}

/// Ancrage vertical du texte dans sa boîte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    /// En haut.
    #[default]
    Top,
    /// Au milieu.
    Middle,
    /// En bas.
    Bottom,
}

/// Ajustement automatique du texte à sa boîte.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Autofit {
    /// Aucun.
    #[default]
    None,
    /// Le texte rétrécit.
    Shrink {
        /// Échelle de police (fraction).
        font_scale: f64,
        /// Réduction d'interligne (fraction).
        line_reduction: f64,
    },
    /// La boîte grandit avec le texte.
    Resize,
}

/// Alignement horizontal d'un paragraphe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    /// À gauche.
    #[default]
    Left,
    /// Centré.
    Center,
    /// À droite.
    Right,
    /// Justifié.
    Justify,
}

/// Interligne ou espacement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// Proportion de la hauteur de ligne (1.0 = simple).
    Percent(f64),
    /// Valeur absolue en pt.
    Points(f64),
}

/// Puce d'un paragraphe.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Bullet {
    /// Pas de puce.
    #[default]
    None,
    /// Caractère.
    Char {
        /// Le caractère (8 au plus).
        char: String,
        /// Couleur ; absente = celle du texte.
        color: Option<Color>,
        /// Taille relative au texte (1.0 = même taille).
        size_pct: f64,
        /// Police ; absente = celle du texte.
        font: Option<String>,
    },
    /// Numérotation automatique.
    AutoNum {
        /// Schéma OOXML (`arabicPeriod`, `alphaLcParenR`…).
        scheme: String,
        /// Premier numéro.
        start: u32,
    },
}

/// Majuscules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Caps {
    /// Telles qu'écrites.
    #[default]
    None,
    /// Toutes en capitales.
    All,
    /// Petites capitales.
    Small,
}

/// Style résolu d'un run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunStyle {
    /// Famille (128 caractères au plus) et provenance (`major`/`minor` pour le thème).
    pub font: Styled<String>,
    /// Famille des écritures d'Asie de l'Est (`ea`) et provenance (`major-ea`/`minor-ea`) ;
    /// `None` : aucune n'est donnée.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_ea: Option<Styled<String>>,
    /// Famille des écritures complexes (`cs` : arabe, hébreu, indiennes, thaï…) et
    /// provenance (`major-cs`/`minor-cs`) ; `None` : aucune n'est donnée.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_cs: Option<Styled<String>>,
    /// Langue (`lang`), étiquette BCP 47 telle qu'écrite (32 caractères au plus).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Taille en pt.
    pub size: f64,
    /// Gras (`b`).
    pub bold: bool,
    /// Graisse numérique, 100 à 900 : celle que nomme la famille (`Light` 300, `Semibold`
    /// 600…), sinon 400 ; au moins 700 si gras.
    pub weight: u16,
    /// Italique.
    pub italic: bool,
    /// Souligné.
    pub underline: bool,
    /// Barré.
    pub strike: bool,
    /// Couleur du texte.
    pub color: Color,
    /// Surlignage.
    pub highlight: Option<Color>,
    /// Décalage de ligne de base en % (positif = exposant).
    pub baseline: i32,
    /// Majuscules.
    pub caps: Caps,
    /// Espacement entre caractères, en pt (`spc`).
    pub char_spacing: f64,
    /// Lien hypertexte externe (`http`, `https`, `mailto` seulement).
    pub link: Option<String>,
}

impl RunStyle {
    /// La famille du run pour l'écriture `script` : `ea` ou `cs` si donnée, sinon la latine.
    pub fn family(&self, script: Script) -> &str {
        let other = match script {
            Script::Latin => None,
            Script::EastAsian => self.font_ea.as_ref(),
            Script::Complex => self.font_cs.as_ref(),
        };
        other.map_or(self.font.value.as_str(), |f| f.value.as_str())
    }

    /// Gras (`b`) ou graisse d'au moins 600 (demi-gras) : le gras d'un projecteur sans graisse
    /// numérique.
    pub fn strong(&self) -> bool {
        self.bold || self.weight >= 600
    }
}

/// Champ du numéro de slide ([`Run::field`]) : recalculé par le deck.
pub(crate) const SLIDE_NUMBER: &str = "slidenum";

/// Morceau de texte au style uniforme ; un saut de ligne `a:br` est un run `"\n"`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Run {
    /// Texte (valeur en cache pour un champ).
    pub text: String,
    /// Champ dynamique ([`SLIDE_NUMBER`], `datetime1`…).
    pub field: Option<String>,
    /// Style résolu.
    pub style: RunStyle,
}

/// Style résolu d'un paragraphe.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParaStyle {
    /// Alignement.
    pub align: Align,
    /// Retrait de première ligne en pt (négatif = suspendu).
    pub indent: f64,
    /// Marge gauche en pt.
    pub margin_left: f64,
    /// Interligne.
    pub line_spacing: Spacing,
    /// Espace avant.
    pub space_before: Spacing,
    /// Espace après.
    pub space_after: Spacing,
    /// Puce.
    pub bullet: Bullet,
    /// Écrit de droite à gauche (`rtl`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rtl: bool,
}

/// Paragraphe résolu.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Paragraph {
    /// Niveau 0..8 (`lvl`).
    pub level: u8,
    /// Style du paragraphe.
    pub style: ParaStyle,
    /// Runs, dans l'ordre.
    pub runs: Vec<Run>,
}

/// Style que prend un texte saisi à un niveau donné d'un placeholder.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LevelStyle {
    /// Style de paragraphe.
    pub para: ParaStyle,
    /// Style de run.
    pub run: RunStyle,
}

/// Boîte de texte résolue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextBox {
    /// Paragraphes, dans l'ordre.
    pub paragraphs: Vec<Paragraph>,
    /// Marges intérieures.
    pub insets: Insets,
    /// Ancrage vertical.
    pub anchor: Anchor,
    /// Ajustement automatique.
    pub autofit: Autofit,
    /// Retour à la ligne automatique.
    pub wrap: bool,
    /// Colonnes de texte, 1..16.
    pub columns: u32,
}

impl Default for TextBox {
    /// Boîte vide : une colonne, retour à la ligne, marges OOXML.
    fn default() -> Self {
        Self {
            paragraphs: Vec::new(),
            insets: Insets::default(),
            anchor: Anchor::Top,
            autofit: Autofit::None,
            wrap: true,
            columns: 1,
        }
    }
}
