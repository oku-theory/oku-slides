//! Graphiques (`c:chartSpace`) : type, empilement, séries lues dans le cache (bornées).

use roxmltree::Node;

use crate::collect::{clip, clip_to};
use crate::pptx::color::parse_color;
use crate::pptx::context::PartCtx;
use crate::pptx::xml::{attr_str, attr_u32, child, children, descendant, elements, text_of};
use crate::source::{Chart, ChartKind, Grouping, Series, WarningKind};

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
    Ok(Chart { kind, grouping, title, categories, series })
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod chart_tests;
