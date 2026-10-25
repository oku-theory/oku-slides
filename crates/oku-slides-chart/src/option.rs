//! Sous-ensemble v1 des options ECharts accepté par Oku Slides.
//!
//! Ces types sont la source de vérité : ils valident les options à la
//! désérialisation (`deny_unknown_fields`) et génèrent le JSON Schema fourni
//! au LLM (voir [`crate::schema`]). Les noms de champs suivent ECharts.
//!
//! Les polices viennent du thème appliqué au rendu. Les couleurs aussi, sauf
//! `color` (palette des séries) et `series.itemStyle.color` (une série), en
//! `#rgb` ou `#rrggbb`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Options of a chart, in ECharts format (v1 subset).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChartOption {
    /// Title and subtitle, drawn by Oku Slides above the chart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// Legend; shown by default, except for a pie or a waterfall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend: Option<Legend>,
    /// Tabular data; the first row holds the column headers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset: Option<Dataset>,
    /// Horizontal axis (bars, lines, scatter, waterfall).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_axis: Option<Axis>,
    /// Vertical axis (bars, lines, scatter, waterfall).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_axis: Option<Axis>,
    /// Axes of a radar chart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radar: Option<Radar>,
    /// At least one series, all of the same family.
    ///
    /// Pie, radar and waterfall have only one.
    pub series: Vec<Series>,
    /// Palette of the series, in order (`#rgb` or `#rrggbb`); the theme's if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Vec<String>>,
}

/// Chart title and its subtitle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Title {
    /// Title text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Subtitle, smaller, under the title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtext: Option<String>,
    /// Horizontal alignment of the title and subtitle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<HAlign>,
}

/// Legend of the series, placed at the top of the chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Legend {
    /// Shows the legend; by default, except pie and waterfall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<bool>,
    /// Horizontal alignment; the legend is always placed at the top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<HAlign>,
}

/// Horizontal alignment of a title or a legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HAlign {
    /// Left-aligned.
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

/// Tabular data shared by the series, with stable identifiers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dataset {
    /// 2D table; row 0 holds the column names.
    pub source: Vec<Vec<Cell>>,
    /// Stable identifier of each row, header included.
    ///
    /// Do not supply them: the core draws them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Stable identifier of each column.
    ///
    /// Do not supply them: the core draws them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
}

/// Cell of `dataset.source`: number, text, boolean or empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Cell {
    /// Number.
    Number(f64),
    /// Text; read as a number if it looks like one (decimal comma accepted).
    Text(String),
    /// Checkbox (the Total column of a waterfall).
    Bool(bool),
    /// Empty cell.
    Null,
}

impl Cell {
    /// Numeric value: a number, or a text that looks like one.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Cell::Number(n) => Some(*n),
            Cell::Text(s) => parse_number(s),
            Cell::Bool(_) | Cell::Null => None,
        }
    }

    /// Case cochée ; toute autre cellule compte comme décochée.
    #[must_use]
    pub fn is_checked(&self) -> bool {
        matches!(self, Cell::Bool(true))
    }

    /// Displayed text: whole numbers without decimals, empty for an empty cell.
    #[must_use]
    pub fn as_label(&self) -> String {
        match self {
            Cell::Number(n) => format_number(*n),
            Cell::Text(s) => s.clone(),
            Cell::Bool(b) => b.to_string(),
            Cell::Null => String::new(),
        }
    }
}

/// Nombre saisi en texte : virgule ou point décimal, signe « - » ou « − ».
pub(crate) fn parse_number(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").replace('\u{2212}', "-").parse().ok()
}

/// Displayed number, without decimals if it is whole.
pub(crate) fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        #[expect(clippy::cast_possible_truncation, reason = "integer with absolute value below 1e15, exact in i64")]
        let entier = n as i64;
        format!("{entier}")
    } else {
        format!("{n}")
    }
}

/// Cartesian axis: categories or values, optional bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    /// `category` (labels) or `value` (numbers).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<AxisType>,
    /// Categories, if the data does not come from `dataset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<String>>,
    /// Minimum of the value axis; computed from the data if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum of the value axis; computed from the data if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// Kind of a cartesian axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AxisType {
    /// Label axis.
    Category,
    /// Numeric axis.
    Value,
}

/// Axes of a radar chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Radar {
    /// Radar axes, in the order of the values of `series.data`.
    ///
    /// Required without `dataset`. With `dataset`, column 0 names the axes and
    /// each other column gives a polygon; an indicator with the same name only
    /// sets the maximum there.
    pub indicator: Vec<Indicator>,
}

/// Radar axis: name and maximum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Indicator {
    /// Axis label.
    pub name: String,
    /// Maximum of the axis; computed from the data if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// Series type, which sets the shape of the chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SeriesType {
    /// Vertical bars, or horizontal if `yAxis` is the category axis.
    ///
    /// Horizontal bars do not mix with lines.
    Bar,
    /// Polyline (straight segments) or smoothed line.
    Line,
    /// Pie, donut or rose.
    Pie,
    /// Scatter plot of `[x, y]` points.
    Scatter,
    /// Radar (spider web).
    Radar,
    /// Waterfall, an Oku Slides extension absent from ECharts.
    ///
    /// A single series of variations on a category axis, read from `dataset`.
    /// The column after the values, if any, marks the totals: `true` draws the
    /// running sum (the row's value is ignored).
    Waterfall,
}

/// Data series and its style.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Series {
    /// Series type.
    #[serde(rename = "type")]
    pub kind: SeriesType,
    /// Name shown in the legend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Columns of `dataset` used by the series (name or index).
    ///
    /// The radar ignores it: it always reads the whole `dataset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encode: Option<Encode>,
    /// Inline data, if the series does not use `dataset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<DataItem>>,
    /// Bars or lines: the series of the same group stack.
    ///
    /// Negative values stack downward, separately.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    /// Line: smoothed curve. A single smoothed series smooths all the lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smooth: Option<bool>,
    /// Line or radar: fills the area. Only `{}` is accepted.
    ///
    /// A single series carrying it fills all the lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_style: Option<AreaStyle>,
    /// Pie: radius, or `[inner, outer]` for a donut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    /// Pie: Nightingale rose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rose_type: Option<RoseType>,
    /// Value labels: bars, lines, pie or waterfall.
    ///
    /// Shown by default on a pie or a waterfall only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Label>,
    /// Bars, lines or scatter: color of the series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_style: Option<ItemStyle>,
}

/// Style of a series' elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemStyle {
    /// Color: `#rgb` or `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Columns of `dataset` read by a series, by role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Encode {
    /// Column of the horizontal axis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<ColumnRef>,
    /// Column of the vertical axis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<ColumnRef>,
    /// Pie: column of the slice names (0 by default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_name: Option<ColumnRef>,
    /// Pie: column of the values (1 by default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<ColumnRef>,
}

/// Reference to a column of `dataset`: header name or index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ColumnRef {
    /// Column index, from 0.
    Index(usize),
    /// Column header name (row 0).
    Name(String),
}

/// Element of `series.data`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DataItem {
    /// Single value (bars, lines).
    Value(f64),
    /// Point `[x, y]` (scatter plot).
    Point(Vec<f64>),
    /// Named element (pie, radar).
    Named(NamedItem),
    /// Missing value (gap in the series).
    Null,
}

/// Named element of `series.data` (pie slice, radar profile).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NamedItem {
    /// Element name (pie legend, radar profile).
    pub name: String,
    /// Value, or one value per indicator for a radar.
    pub value: ItemValue,
}

/// Value of a named element: a number or a list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ItemValue {
    /// Single value (pie).
    Number(f64),
    /// One value per indicator (radar).
    Values(Vec<f64>),
}

/// Area fill; its mere presence enables it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AreaStyle {}

/// Value labels of a series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Label {
    /// `true` shows the value on each element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<bool>,
}

/// Radius of a pie: full or donut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Radius {
    /// Outer radius of a full pie.
    Single(Length),
    /// `[inner, outer]` radii of a donut.
    Pair([Length; 2]),
}

/// Length in points (`120`) or as a percentage (`"60%"`).
///
/// A percentage refers to half of the chart's shorter side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Length {
    /// Length in points (1 pt = 1 SVG unit).
    Pixels(f64),
    /// Percentage, for example `"60%"`.
    Percent(String),
}

impl Length {
    /// Résout la longueur par rapport à une taille de référence (demi-côté).
    #[must_use]
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

/// Nightingale rose: a boolean or an ECharts mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RoseType {
    /// `true` enables the rose.
    Flag(bool),
    /// Named mode; either one enables the rose.
    Mode(RoseMode),
}

/// ECharts rose mode; both modes render identically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RoseMode {
    /// ECharts `radius` mode; rendered like `area`.
    Radius,
    /// ECharts `area` mode; rendered like `radius`.
    Area,
}

impl RoseType {
    /// True if the rose is requested.
    #[must_use]
    pub fn enabled(&self) -> bool {
        match self {
            RoseType::Flag(b) => *b,
            RoseType::Mode(_) => true,
        }
    }
}
