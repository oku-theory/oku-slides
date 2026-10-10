//! Graphiques : type et données lues dans le cache du XML.

use serde::Serialize;

use super::style::Color;

/// Type de graphique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    /// Barres verticales.
    Column,
    /// Barres horizontales.
    Bar,
    /// Courbes.
    Line,
    /// Secteurs.
    Pie,
    /// Anneau.
    Doughnut,
    /// Aires.
    Area,
    /// Nuage de points.
    Scatter,
    /// Radar.
    Radar,
}

/// Empilement des séries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    /// Côte à côte.
    #[default]
    Clustered,
    /// Empilées.
    Stacked,
    /// Empilées à 100 %.
    PercentStacked,
    /// Sans empilement (courbes, aires).
    Standard,
}

/// Une série.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Series {
    /// Nom.
    pub name: String,
    /// Valeurs ; absente ou non numérique = `None`.
    pub values: Vec<Option<f64>>,
    /// Couleur explicite de la série.
    pub color: Option<Color>,
}

/// Graphique.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chart {
    /// Type (le premier, pour un graphique combiné).
    pub kind: ChartKind,
    /// Empilement.
    pub grouping: Grouping,
    /// Titre.
    pub title: Option<String>,
    /// Libellés des catégories (les X d'un nuage de points, en texte).
    pub categories: Vec<String>,
    /// Séries (256 au plus).
    pub series: Vec<Series>,
}
