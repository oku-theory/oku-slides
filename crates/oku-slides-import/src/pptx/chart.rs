//! Graphiques (`c:chartSpace`) : type, empilement, séries lues dans le cache (bornées).

use roxmltree::Node;

use crate::collect::{clip, clip_to};
use crate::pptx::bounds::{GAP_WIDTH, attr_in};
use crate::pptx::color::parse_color;
use crate::pptx::context::PartCtx;
use crate::pptx::xml::{
    attr_bool, attr_f64, attr_i64, attr_str, attr_u32, child, children, descendant, elements, text_of,
};
use crate::source::{Axes, Chart, ChartKind, Grouping, Series, ValueAxis, WarningKind};

/// Nombre maximal de points lus par cache.
pub(crate) const MAX_CHART_POINTS: usize = 10_000;
/// Nombre maximal de séries par graphique.
pub(crate) const MAX_SERIES: usize = 256;
/// Cases vides admises au-delà des points présents (fin de série sans valeur).
pub(crate) const MAX_TRAILING_GAP: usize = 64;
/// Lignes gardées d'un titre de graphique.
pub(crate) const MAX_TITLE_LINES: usize = 8;
/// Caractères lus par ligne de titre, avant `clip` : borne la copie d'un titre géant.
const MAX_TITLE_CHARS: usize = 1024;
/// Caractères gardés d'un `formatCode`.
const MAX_FORMAT_CHARS: usize = 64;
/// Axes read per chart: twice the four of a combo chart (two per type), for leeway.
const MAX_AXES: usize = 8;
/// `c:gapWidth` of a bar chart that does not write it (`CT_GapAmount` default).
const DEFAULT_GAP_WIDTH: u16 = 150;

/// Pourquoi un graphique n'est pas lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChartSkip {
    /// Type non géré (`bubbleChart`, `stockChart`, `surfaceChart`…).
    Unsupported(String),
    /// Structure absente (`plotArea`, aucun type).
    Malformed(&'static str),
}

/// Valeurs d'un cache, de longueur proportionnelle aux points présents.
fn cache(n: Node<'_, '_>) -> Vec<Option<String>> {
    let Some(c) = n
        .descendants()
        .find(|d| matches!(d.tag_name().name(), "strCache" | "numCache" | "strLit" | "numLit" | "multiLvlStrCache"))
    else {
        return Vec::new();
    };
    // Catégories à plusieurs niveaux : le premier `lvl` est la feuille, un libellé par point.
    let points = if c.tag_name().name() == "multiLvlStrCache" { child(c, "lvl") } else { Some(c) };
    let declared =
        child(c, "ptCount").and_then(|p| attr_u32(p, "val")).map_or(0, |v| usize::try_from(v).unwrap_or(usize::MAX));
    let pts: Vec<(usize, Option<String>)> = points
        .into_iter()
        .flat_map(|p| children(p, "pt"))
        .take(MAX_CHART_POINTS)
        .filter_map(|pt| {
            Some((usize::try_from(attr_u32(pt, "idx")?).ok()?, child(pt, "v").and_then(|v| v.text()).map(clip)))
        })
        .collect();
    let present = pts.iter().map(|(i, _)| i.saturating_add(1)).max().unwrap_or(0);
    let len = declared.max(present).min(pts.len().saturating_add(MAX_TRAILING_GAP)).min(MAX_CHART_POINTS);
    let mut out = vec![None; len];
    for (i, v) in pts {
        if let Some(slot) = out.get_mut(i) {
            *slot = v;
        }
    }
    out
}

/// Valeurs numériques du cache de `n` ; une case absente ou non numérique est `None`.
fn numbers(n: Node<'_, '_>) -> Vec<Option<f64>> {
    cache(n).into_iter().map(|x| x.and_then(|t| t.trim().parse::<f64>().ok().filter(|f| f.is_finite()))).collect()
}

/// Vrai si `n` porte un cache numérique (`numRef`, `numLit`).
fn is_numeric(n: Node<'_, '_>) -> bool {
    n.descendants().any(|d| matches!(d.tag_name().name(), "numCache" | "numLit"))
}

/// `formatCode` du cache numérique de `n`, sauf `General` (le format par défaut).
fn format_code(n: Node<'_, '_>) -> Option<String> {
    let c = n.descendants().find(|d| matches!(d.tag_name().name(), "numCache" | "numLit"))?;
    let code = child(c, "formatCode")?.text()?.trim();
    (!code.is_empty() && !code.eq_ignore_ascii_case("General")).then(|| clip_to(code, MAX_FORMAT_CHARS))
}

fn series_name(ser: Node<'_, '_>) -> String {
    let Some(tx) = child(ser, "tx") else { return String::new() };
    if let Some(v) = child(tx, "v") {
        return clip(v.text().unwrap_or_default());
    }
    clip(&cache(tx).into_iter().flatten().collect::<Vec<_>>().join(" "))
}

/// Lit un `c:chartSpace` ; `item` désigne le cadre dans les avertissements.
pub(crate) fn parse_chart(root: Node<'_, '_>, cx: &PartCtx<'_>, item: &str) -> Result<Chart, ChartSkip> {
    let plot = descendant(root, "plotArea").ok_or(ChartSkip::Malformed("plotArea"))?;
    let types: Vec<Node<'_, '_>> = elements(plot).filter(|n| n.tag_name().name().ends_with("Chart")).take(8).collect();
    let first = *types.first().ok_or(ChartSkip::Malformed("chart type"))?;
    let name = first.tag_name().name();
    if types.len() > 1 {
        cx.warn(Some(item), WarningKind::UnsupportedChart(clip(&format!("combo: {name}"))));
    }
    let kind = match name {
        "barChart" | "bar3DChart" => {
            if child(first, "barDir").and_then(|d| attr_str(d, "val")) == Some("bar") {
                ChartKind::Bar
            } else {
                ChartKind::Column
            }
        }
        "lineChart" | "line3DChart" => ChartKind::Line,
        "pieChart" | "pie3DChart" | "ofPieChart" => ChartKind::Pie,
        "doughnutChart" => ChartKind::Doughnut,
        "areaChart" | "area3DChart" => ChartKind::Area,
        "scatterChart" => ChartKind::Scatter,
        "radarChart" => ChartKind::Radar,
        other => return Err(ChartSkip::Unsupported(clip(other))),
    };
    let grouping = match child(first, "grouping").and_then(|g| attr_str(g, "val")) {
        Some("stacked") => Grouping::Stacked,
        Some("percentStacked") => Grouping::PercentStacked,
        Some("standard") => Grouping::Standard,
        Some("clustered") => Grouping::Clustered,
        _ if matches!(kind, ChartKind::Bar | ChartKind::Column) => Grouping::Clustered,
        _ => Grouping::Standard,
    };
    let sers: Vec<Node<'_, '_>> = children(first, "ser").take(MAX_SERIES).collect();
    let cat = sers.first().and_then(|s| child(*s, "cat").or_else(|| child(*s, "xVal")));
    if cat.and_then(|c| descendant(c, "multiLvlStrCache")).is_some_and(|m| children(m, "lvl").nth(1).is_some()) {
        cx.warn(Some(item), WarningKind::Approximated("multiLvlStrRef".into()));
    }
    let categories = cat.map(|c| cache(c).into_iter().map(Option::unwrap_or_default).collect()).unwrap_or_default();
    let series = sers
        .iter()
        .map(|s| {
            let values = child(*s, "val").or_else(|| child(*s, "yVal"));
            // Un nuage de points a des X par série ; numériques, ils ne passent pas par les catégories.
            let x = (kind == ChartKind::Scatter).then(|| child(*s, "xVal")).flatten().filter(|x| is_numeric(*x));
            Series {
                name: series_name(*s),
                values: values.map(numbers).unwrap_or_default(),
                color: child(*s, "spPr").and_then(|p| child(p, "solidFill")).and_then(|f| parse_color(f, &cx.colors)),
                x: x.map(numbers),
                format: values.and_then(format_code),
            }
        })
        .collect();
    let legend = descendant(root, "chart").and_then(|c| child(c, "legend")).is_some();
    let (axes, value) = axes(plot, first, kind == ChartKind::Bar, cx.shared.mso2007);
    let value_axis = value.map(|v| value_axis(v, cx, item)).unwrap_or_default();
    let gap_width = matches!(kind, ChartKind::Bar | ChartKind::Column).then(|| gap_width(first, cx, item));
    let title = descendant(root, "chart")
        .and_then(|c| child(c, "title"))
        .map(|t| {
            text_of(t, MAX_TITLE_LINES, MAX_TITLE_CHARS)
                .split('\n')
                .take(MAX_TITLE_LINES)
                .map(clip)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|t| !t.trim().is_empty());
    Ok(Chart { kind, grouping, title, categories, series, legend, axes, value_axis, gap_width })
}

/// `c:gapWidth` of the bar chart `first`, 150 if absent, clamped to 0 to 500 with `Approximated`.
fn gap_width(first: Node<'_, '_>, cx: &PartCtx<'_>, item: &str) -> u16 {
    let Some(g) = child(first, "gapWidth") else { return DEFAULT_GAP_WIDTH };
    if attr_i64(g, "val").is_some_and(|v| !(GAP_WIDTH.0..=GAP_WIDTH.1).contains(&v)) {
        cx.warn(Some(item), WarningKind::Approximated("gapWidth".into()));
    }
    attr_in(g, "val", GAP_WIDTH).and_then(|v| u16::try_from(v).ok()).unwrap_or(DEFAULT_GAP_WIDTH)
}

/// Gridlines and scale of the value axis `ax`.
///
/// Bounds that are not finite, or not in order, are dropped with
/// `Approximated`, and so is a `majorUnit` that is not above 0. A log scale
/// (`c:logBase`) and a reversed one (`c:orientation` `maxMin`) are drawn
/// linear and in order, with `Approximated`.
fn value_axis(ax: Node<'_, '_>, cx: &PartCtx<'_>, item: &str) -> ValueAxis {
    let scaling = child(ax, "scaling");
    let bound = |name: &str| scaling.and_then(|s| child(s, name)).and_then(|b| attr_f64(b, "val"));
    let (mut min, mut max) = (bound("min").filter(|v| v.is_finite()), bound("max").filter(|v| v.is_finite()));
    if let (Some(lo), Some(hi)) = (min, max)
        && lo >= hi
    {
        cx.warn(Some(item), WarningKind::Approximated("c:scaling".into()));
        (min, max) = (None, None);
    }
    let unit = child(ax, "majorUnit").and_then(|u| attr_f64(u, "val"));
    let major_unit = unit.filter(|u| u.is_finite() && *u > 0.0);
    if unit.is_some() && major_unit.is_none() {
        cx.warn(Some(item), WarningKind::Approximated("majorUnit".into()));
    }
    if scaling.and_then(|s| child(s, "logBase")).is_some() {
        cx.warn(Some(item), WarningKind::Approximated("logBase".into()));
    }
    if scaling.and_then(|s| child(s, "orientation")).and_then(|o| attr_str(o, "val")) == Some("maxMin") {
        cx.warn(Some(item), WarningKind::Approximated("orientation".into()));
    }
    ValueAxis { gridlines: child(ax, "majorGridlines").is_some(), min, max, major_unit }
}

/// The axes of the chart type `first` drawn, by side, and its value axis; `horizontal`: bars along y.
///
/// Only the axes `first` names by `c:axId` count (a combo's other axes do not).
/// A category axis (`c:catAx`, `c:dateAx`) is on the category side, x or y
/// for horizontal bars, and a value axis on the other; the two value axes of
/// a scatter are placed by `c:axPos` (`b`, `t`: x), else in order, the
/// vertical one being its value axis. An axis is deleted by `c:delete`, a
/// bare one too (`CT_Boolean` defaults to true), and when the element is
/// absent, as LibreOffice and python-pptx read it; in a file of PowerPoint
/// 2007 (`mso2007`), whose schema defaulted to false, both mean shown, as
/// LibreOffice's oox reads it (`mbDeleted = !bMSO2007Doc`, t-1385). A side
/// is drawn when one of its axes is; a side without axis is not, as both
/// witnesses draw it (t-954).
fn axes<'a, 'i>(
    plot: Node<'a, 'i>,
    first: Node<'a, 'i>,
    horizontal: bool,
    mso2007: bool,
) -> (Axes, Option<Node<'a, 'i>>) {
    let ids: Vec<&str> = children(first, "axId").filter_map(|a| attr_str(a, "val")).take(MAX_AXES).collect();
    let named = |n: &Node<'_, '_>| child(*n, "axId").and_then(|a| attr_str(a, "val")).is_some_and(|v| ids.contains(&v));
    let read: Vec<Node<'_, '_>> = elements(plot)
        .filter(|n| matches!(n.tag_name().name(), "catAx" | "dateAx" | "valAx") && named(n))
        .take(MAX_AXES)
        .collect();
    let has_category = read.iter().any(|n| n.tag_name().name() != "valAx");
    let (mut x, mut y) = (false, false);
    let mut value = None;
    for (rank, ax) in read.into_iter().enumerate() {
        let category = ax.tag_name().name() != "valAx";
        let vertical = if category {
            horizontal
        } else if has_category {
            !horizontal
        } else {
            match child(ax, "axPos").and_then(|p| attr_str(p, "val")) {
                Some("l" | "r") => true,
                Some("b" | "t") => false,
                _ => rank > 0,
            }
        };
        if !category && (has_category || vertical) && value.is_none() {
            value = Some(ax);
        }
        let deleted = child(ax, "delete").map_or(!mso2007, |d| attr_bool(d, "val").unwrap_or(!mso2007));
        let side = if vertical { &mut y } else { &mut x };
        *side |= !deleted;
    }
    (Axes { x, y }, value)
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod chart_tests;
