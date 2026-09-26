//! Changer le type d'un graphique : ses données passent par un tableau
//! neutre (une colonne de libellés, des colonnes de valeurs nommées) que
//! chaque type sait lire et écrire. Le résultat est canonique.
//!
//! Barres, lignes et radar partagent la même forme de tableau ; les secteurs
//! n'en gardent que la première colonne de valeurs ; un nuage de points prend
//! les libellés comme abscisses (numérotées 1, 2… s'ils ne sont pas des nombres).

use serde_json::json;
use snafu::ResultExt;

use crate::error::{InvalidOptionSnafu, Result};
use crate::labels::ChartLabels;
use crate::model::{ChartKind, normalize};
use crate::option::{Cell, ChartOption, Dataset, SeriesType};

/// Tableau neutre : libellés des lignes et colonnes de valeurs.
struct Table {
    label: String,
    rows: Vec<String>,
    columns: Vec<(String, Vec<Option<f64>>)>,
}

/// `option` redessiné en `kind`, sous forme canonique ; titre et légende gardés.
pub fn convert(option: &ChartOption, kind: SeriesType, labels: &ChartLabels) -> Result<ChartOption> {
    let table = table(option, labels)?;
    let series_count = table.columns.len().max(1);
    let (source, series, axes) = match kind {
        SeriesType::Bar | SeriesType::Line => {
            let series = match shared_stack(option) {
                Some(stack) => json!({ "type": kind, "stack": stack }),
                None => json!({ "type": kind }),
            };
            (grid(&table, table.columns.len()), vec![series; series_count], true)
        }
        SeriesType::Radar => (grid(&table, table.columns.len()), vec![json!({ "type": "radar" })], false),
        SeriesType::Pie => (grid(&table, 1), vec![json!({ "type": "pie" })], false),
        SeriesType::Scatter => {
            let series =
                (0..series_count).map(|i| json!({ "type": "scatter", "encode": { "x": 2 * i, "y": 2 * i + 1 } }));
            (scatter_grid(&table, labels), series.collect(), false)
        }
    };
    let mut skeleton = json!({ "series": series });
    if axes {
        skeleton["xAxis"] = json!({ "type": "category" });
        skeleton["yAxis"] = json!({ "type": "value" });
    }
    let mut out: ChartOption = serde_json::from_value(skeleton).context(InvalidOptionSnafu)?;
    out.title.clone_from(&option.title);
    out.legend.clone_from(&option.legend);
    out.dataset = Some(Dataset { source });
    normalize(&out, labels)?;
    Ok(out)
}

fn table(option: &ChartOption, labels: &ChartLabels) -> Result<Table> {
    Ok(match normalize(option, labels)?.kind {
        ChartKind::Cartesian(c) => Table {
            label: c.category_label,
            rows: c.categories,
            columns: c.series.into_iter().map(|s| (s.name, s.values)).collect(),
        },
        ChartKind::Radar(r) => Table {
            label: r.indicator_label,
            rows: r.indicators.into_iter().map(|(name, _)| name).collect(),
            columns: r.series.into_iter().map(|s| (s.name, s.values)).collect(),
        },
        ChartKind::Pie(p) => {
            let (rows, values) = p.slices.into_iter().unzip();
            let name = if p.name.is_empty() { labels.value().to_owned() } else { p.name };
            Table { label: p.name_label, rows, columns: vec![(name, values)] }
        }
        ChartKind::Scatter(s) => {
            // Les abscisses de toutes les séries, triées, deviennent les lignes.
            let mut xs: Vec<f64> = s.series.iter().flat_map(|s| s.points.iter().map(|(x, _)| *x)).collect();
            xs.sort_by(f64::total_cmp);
            xs.dedup();
            let columns = s
                .series
                .into_iter()
                .map(|s| {
                    let values = xs.iter().map(|x| s.points.iter().find(|(px, _)| px == x).map(|(_, y)| *y)).collect();
                    (s.name, values)
                })
                .collect();
            Table { label: labels.category().to_owned(), rows: xs.iter().map(|x| number(*x)).collect(), columns }
        }
    })
}

/// Le groupe d'empilement commun à toutes les séries, s'il y en a un :
/// des barres empilées deviennent des lignes empilées, et inversement.
fn shared_stack(option: &ChartOption) -> Option<&str> {
    let first = option.series.first()?.stack.as_deref()?;
    option.series.iter().all(|s| s.stack.as_deref() == Some(first)).then_some(first)
}

/// En-têtes puis une ligne par libellé, avec les `width` premières colonnes.
fn grid(table: &Table, width: usize) -> Vec<Vec<Cell>> {
    let mut header = vec![Cell::Text(table.label.clone())];
    header.extend(table.columns.iter().take(width).map(|(name, _)| Cell::Text(name.clone())));
    let rows = table.rows.iter().enumerate().map(|(i, row)| {
        let mut cells = vec![Cell::Text(row.clone())];
        cells.extend(table.columns.iter().take(width).map(|(_, v)| cell(v.get(i).copied().flatten())));
        cells
    });
    std::iter::once(header).chain(rows).collect()
}

/// Une paire de colonnes (x, y) par série ; une valeur absente n'est pas un point.
fn scatter_grid(table: &Table, labels: &ChartLabels) -> Vec<Vec<Cell>> {
    let parsed: Option<Vec<f64>> = table.rows.iter().map(|r| r.trim().replace(',', ".").parse().ok()).collect();
    let xs = parsed.unwrap_or_else(|| (1..=table.rows.len()).map(|i| i as f64).collect());
    let header = table.columns.iter().flat_map(|(name, _)| [Cell::Text(labels.x_of(name)), Cell::Text(name.clone())]);
    let rows = xs.iter().enumerate().map(|(i, x)| {
        table
            .columns
            .iter()
            .flat_map(|(_, v)| match v.get(i).copied().flatten() {
                Some(y) => [Cell::Number(*x), Cell::Number(y)],
                None => [Cell::Null, Cell::Null],
            })
            .collect()
    });
    std::iter::once(header.collect()).chain(rows).collect()
}

fn cell(v: Option<f64>) -> Cell {
    v.map_or(Cell::Null, Cell::Number)
}

/// `3.0` → `3`, `2.5` → `2.5`.
fn number(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 { format!("{}", x as i64) } else { x.to_string() }
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod convert_tests;
