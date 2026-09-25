//! Sous-ensemble v1 des options ECharts accepté par Slider.
//!
//! Ces types sont la source de vérité : ils valident les options à la
//! désérialisation (`deny_unknown_fields`) et génèrent le JSON Schema fourni
//! au LLM (voir [`crate::schema`]). Les noms de champs suivent ECharts.
//!
//! Les couleurs et les polices ne font pas partie du sous-ensemble : elles
//! viennent du thème appliqué au rendu.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Options d'un graphique, au format ECharts (sous-ensemble v1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChartOption {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend: Option<Legend>,
    /// Données tabulaires. Première ligne = en-têtes de colonnes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset: Option<Dataset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_axis: Option<Axis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_axis: Option<Axis>,
    /// Axes d'un graphique radar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radar: Option<Radar>,
    /// Au moins une série.
    pub series: Vec<Series>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Title {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtext: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<HAlign>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Legend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<bool>,
    /// Alignement horizontal. La légende est toujours placée en haut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<HAlign>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dataset {
    /// Tableau 2D ; la ligne 0 contient les noms de colonnes.
    pub source: Vec<Vec<Cell>>,
}

/// Cellule de `dataset.source` : nombre, texte ou vide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Cell {
    Number(f64),
    Text(String),
    Null,
}

impl Cell {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Cell::Number(n) => Some(*n),
            Cell::Text(s) => s.trim().replace(',', ".").parse().ok(),
            Cell::Null => None,
        }
    }

    pub fn as_label(&self) -> String {
        match self {
            Cell::Number(n) => format_number(*n),
            Cell::Text(s) => s.clone(),
            Cell::Null => String::new(),
        }
    }
}

pub(crate) fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{n}") }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<AxisType>,
    /// Catégories, si les données ne viennent pas de `dataset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AxisType {
    Category,
    Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Radar {
    pub indicator: Vec<Indicator>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Indicator {
    pub name: String,
    /// Maximum de l'axe ; calculé depuis les données s'il est absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SeriesType {
    Bar,
    Line,
    Pie,
    Scatter,
    Radar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Series {
    #[serde(rename = "type")]
    pub kind: SeriesType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Colonnes de `dataset` utilisées par la série (nom ou index).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encode: Option<Encode>,
    /// Données en ligne, si la série n'utilise pas `dataset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<DataItem>>,
    /// Ligne : courbe lissée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smooth: Option<bool>,
    /// Ligne ou radar : remplit l'aire. Seul `{}` est accepté.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_style: Option<AreaStyle>,
    /// Camembert : rayon, ou `[intérieur, extérieur]` pour un donut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    /// Camembert : rose de Nightingale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rose_type: Option<RoseType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Label>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Encode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<ColumnRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<ColumnRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_name: Option<ColumnRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<ColumnRef>,
}

/// Référence à une colonne de `dataset` : nom d'en-tête ou index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ColumnRef {
    Index(usize),
    Name(String),
}

/// Élément de `series.data`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DataItem {
    /// Valeur simple (barres, lignes).
    Value(f64),
    /// Point `[x, y]` (nuage de points).
    Point(Vec<f64>),
    /// Élément nommé (camembert, radar).
    Named(NamedItem),
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NamedItem {
    pub name: String,
    pub value: ItemValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ItemValue {
    Number(f64),
    Values(Vec<f64>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AreaStyle {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Label {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Radius {
    Single(Length),
    Pair([Length; 2]),
}

/// Longueur en pixels (`120`) ou en pourcentage (`"60%"`) du plus petit côté.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Length {
    Pixels(f64),
    Percent(String),
}

impl Length {
    /// Résout la longueur par rapport à une taille de référence (demi-côté).
    pub fn resolve(&self, reference: f64) -> Option<f64> {
        match self {
            Length::Pixels(p) => Some(*p),
            Length::Percent(s) => {
                let n: f64 = s.trim().strip_suffix('%')?.trim().parse().ok()?;
                Some(reference * n / 100.0)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RoseType {
    Flag(bool),
    Mode(RoseMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RoseMode {
    Radius,
    Area,
}

impl RoseType {
    pub fn enabled(&self) -> bool {
        match self {
            RoseType::Flag(b) => *b,
            RoseType::Mode(_) => true,
        }
    }
}
