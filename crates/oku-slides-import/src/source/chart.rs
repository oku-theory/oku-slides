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
    /// Abscisses propres d'une série de nuage de points (`xVal` numérique), au rang de ses valeurs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<Vec<Option<f64>>>,
    /// Format des valeurs (`formatCode` du cache, `0%`, `#,##0 €`…), s'il n'est pas `General`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Axes drawn by side: bottom or top (`x`), left or right (`y`).
///
/// A side is drawn when one of its axes is not deleted (`c:delete`); a side without axis is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Axes {
    /// The horizontal axis, at the bottom or the top.
    pub x: bool,
    /// The vertical axis, on the left or the right.
    pub y: bool,
}

/// The value axis as written: gridlines and scale (t-1384).
///
/// Bounds are finite with `min` below `max`, `major_unit` finite above 0; the
/// reader drops the others.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct ValueAxis {
    /// Major gridlines drawn across the plot (`c:majorGridlines`).
    pub gridlines: bool,
    /// Lower bound (`c:scaling/c:min`); automatic if absent.
    pub min: Option<f64>,
    /// Upper bound (`c:scaling/c:max`); automatic if absent.
    pub max: Option<f64>,
    /// Step between ticks (`c:majorUnit`); automatic if absent.
    pub major_unit: Option<f64>,
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
    /// Libellés des catégories (les X textuels d'un nuage de points, ceux de la première série).
    pub categories: Vec<String>,
    /// Séries (256 au plus).
    pub series: Vec<Series>,
    /// A legend is drawn: the chart has a `c:legend` (t-954).
    pub legend: bool,
    /// The axes drawn (t-954).
    pub axes: Axes,
    /// The value axis (the vertical one of a scatter): gridlines and scale (t-1384).
    pub value_axis: ValueAxis,
    /// Bars: gap between categories in % of a bar's width (`c:gapWidth`, 150 if absent, 0 to 500).
    pub gap_width: Option<u16>,
}
