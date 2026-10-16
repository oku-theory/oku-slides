//! Graphique source → options canoniques d'un widget `chart` (`host(chart)`).
//!
//! Forme canonique (`oku_slides_chart::canonicalize`) : toutes les données
//! dans `dataset`, des séries nues qui lisent chacune leur colonne. Aucune
//! couleur n'est émise : la palette du thème cible décide.

use oku_slides_core::dataset_row_budget;
use serde_json::{Value, json};

use crate::report::LossKind;
use crate::source::{self, ChartKind, Grouping};

/// Groupe d'empilement des séries empilées.
const STACK: &str = "total";
/// Rayons d'un anneau.
const DOUGHNUT: [&str; 2] = ["40%", "70%"];

/// Une valeur : un nombre fini, sinon une cellule vide.
fn value(v: Option<f64>) -> Value {
    v.filter(|v| v.is_finite()).map_or(Value::Null, Value::from)
}

/// Les libellés des catégories, complétés `1`, `2`… jusqu'à la plus longue série.
fn categories(c: &source::Chart) -> Vec<String> {
    let rows = c.series.iter().map(|s| s.values.len()).max().unwrap_or(0).max(c.categories.len());
    (0..rows).map(|i| c.categories.get(i).cloned().unwrap_or_else(|| (i + 1).to_string())).collect()
}

/// `dataset.source` : en-tête (catégorie vide puis noms des séries), une ligne par catégorie.
fn grid(c: &source::Chart, series: &[source::Series]) -> Vec<Vec<Value>> {
    let mut header = vec![Value::from("")];
    header.extend(series.iter().map(|s| Value::from(s.name.as_str())));
    let rows = categories(c).into_iter().enumerate().map(|(i, cat)| {
        let mut row = vec![Value::from(cat)];
        row.extend(series.iter().map(|s| value(s.values.get(i).copied().flatten())));
        row
    });
    std::iter::once(header).chain(rows).collect()
}

/// Une paire de colonnes (x, y) par série ; les abscisses sont celles de la
/// série (`x`), sinon les catégories lues comme nombres, sinon `1`, `2`… ; une
/// valeur ou une abscisse absente n'est pas un point.
fn scatter_grid(c: &source::Chart) -> Vec<Vec<Value>> {
    let cats = categories(c);
    let parsed: Option<Vec<f64>> =
        cats.iter().map(|s| s.trim().parse::<f64>().ok().filter(|x| x.is_finite())).collect();
    let shared: Vec<Option<f64>> =
        parsed.unwrap_or_else(|| (1..=cats.len()).map(|i| i as f64).collect()).into_iter().map(Some).collect();
    let xs: Vec<&[Option<f64>]> = c.series.iter().map(|s| s.x.as_deref().unwrap_or(&shared)).collect();
    let rows = xs.iter().map(|x| x.len()).max().unwrap_or(0).max(cats.len());
    let header = c.series.iter().flat_map(|s| [Value::from("X"), Value::from(s.name.as_str())]).collect();
    let rows = (0..rows).map(|i| {
        c.series
            .iter()
            .zip(&xs)
            .flat_map(|(s, x)| match (value(x.get(i).copied().flatten()), value(s.values.get(i).copied().flatten())) {
                (Value::Null, _) | (_, Value::Null) => [Value::Null, Value::Null],
                (x, y) => [x, y],
            })
            .collect()
    });
    std::iter::once(header).chain(rows).collect()
}

/// Les options canoniques du graphique `c` et ses pertes ; `None` sans série.
///
/// | source | options |
/// |---|---|
/// | `Column` | barres, catégories en X |
/// | `Bar` | barres, catégories en Y |
/// | `Line` | lignes |
/// | `Area` | lignes avec `areaStyle` |
/// | `Pie` | secteurs, première série seule (les autres : perte `SeriesDropped`) |
/// | `Doughnut` | secteurs, rayons `40%` et `70%` |
/// | `Scatter` | une paire de colonnes (x, y) par série |
/// | `Radar` | radar, axes = catégories |
///
/// `Stacked` pose `stack` sur les séries cartésiennes ; `PercentStacked` aussi,
/// avec la perte `PercentStacked`. Le titre devient `title.text`. Un format des
/// nombres (`0%`, monnaie) se perd (`ChartFormat`) : le modèle n'en a pas.
/// `dataset.source` est coupé au budget de relecture du cœur
/// ([`dataset_row_budget`], en-tête compris), avec la perte `ChartTruncated`.
pub(crate) fn chart(c: &source::Chart) -> Option<(Value, Vec<LossKind>)> {
    if c.series.is_empty() {
        return None;
    }
    let mut losses = Vec::new();
    let cartesian = |kind: &str, area: bool| {
        let mut s = json!({ "type": kind });
        if matches!(c.grouping, Grouping::Stacked | Grouping::PercentStacked) {
            s["stack"] = json!(STACK);
        }
        if area {
            s["areaStyle"] = json!({});
        }
        vec![s; c.series.len()]
    };
    let category = json!({ "type": "category" });
    let value_axis = json!({ "type": "value" });
    let mut option = match c.kind {
        ChartKind::Column | ChartKind::Line | ChartKind::Area => {
            let kind = if c.kind == ChartKind::Column { "bar" } else { "line" };
            json!({
                "dataset": { "source": grid(c, &c.series) },
                "xAxis": category, "yAxis": value_axis,
                "series": cartesian(kind, c.kind == ChartKind::Area),
            })
        }
        ChartKind::Bar => json!({
            "dataset": { "source": grid(c, &c.series) },
            "xAxis": value_axis, "yAxis": category,
            "series": cartesian("bar", false),
        }),
        ChartKind::Pie | ChartKind::Doughnut => {
            let mut s = json!({ "type": "pie" });
            if c.kind == ChartKind::Doughnut {
                s["radius"] = json!(DOUGHNUT);
            }
            // `series` n'est pas vide (voir plus haut) ; au plus 256 séries (lot 1).
            let dropped = u32::try_from(c.series.len() - 1).unwrap_or(u32::MAX);
            if dropped > 0 {
                losses.push(LossKind::SeriesDropped { kept: 1, dropped });
            }
            json!({ "dataset": { "source": grid(c, &c.series[..1]) }, "series": [s] })
        }
        ChartKind::Scatter => {
            let series: Vec<Value> = (0..c.series.len())
                .map(|i| json!({ "type": "scatter", "encode": { "x": 2 * i, "y": 2 * i + 1 } }))
                .collect();
            json!({ "dataset": { "source": scatter_grid(c) }, "series": series })
        }
        ChartKind::Radar => json!({ "dataset": { "source": grid(c, &c.series) }, "series": [{ "type": "radar" }] }),
    };
    let stacked = matches!(c.kind, ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area);
    if stacked && c.grouping == Grouping::PercentStacked {
        losses.push(LossKind::PercentStacked);
    }
    // Le modèle de graphique n'a pas de format des nombres : le premier format écrit se perd.
    if let Some(format) = c.series.iter().find_map(|s| s.format.clone()) {
        losses.push(LossKind::ChartFormat(format));
    }
    if let Some(title) = c.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        option["title"] = json!({ "text": title });
    }
    if let Some(rows) = option.pointer_mut("/dataset/source").and_then(Value::as_array_mut) {
        let budget = dataset_row_budget(rows.first().and_then(Value::as_array).map_or(0, Vec::len));
        if rows.len() > budget {
            rows.truncate(budget);
            losses.push(LossKind::ChartTruncated);
        }
    }
    Some((option, losses))
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod chart_tests;
