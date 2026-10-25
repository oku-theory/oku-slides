//! Graphique ↔ tableau : les mêmes données, l'autre widget.

use oku_slides_i18n::Message;
use serde_json::json;
use snafu::ResultExt;

use crate::canonical::{fresh_series, is_canonical_cartesian, sync_series_with_columns};
use crate::error::{DataOpSnafu, InvalidOptionSnafu, Result};
use crate::labels::ChartLabels;
use crate::model::normalize;
use crate::option::{Cell, ChartOption, Dataset, Series};
use crate::table::{TableOption, validate_table};

/// The data table of a chart, short rows padded.
///
/// # Errors
///
/// Fails if the chart has no `dataset` or if its data exceed the
/// limits of a table.
pub fn chart_to_table(chart: &ChartOption) -> Result<TableOption> {
    let dataset =
        chart.dataset.as_ref().ok_or_else(|| DataOpSnafu { reason: Message::new("chart-data-no-dataset") }.build())?;
    let width = dataset.source.iter().map(Vec::len).max().unwrap_or(0);
    let source = dataset
        .source
        .iter()
        .map(|r| {
            let mut r = r.clone();
            r.resize(width, Cell::Null);
            r
        })
        .collect();
    // Une ligne 0 complétée change la largeur : ses identifiants de colonnes sont tirés à neuf.
    let columns = if dataset.columns.len() == width { dataset.columns.clone() } else { Vec::new() };
    // Au retour, chaque série retrouvera sa colonne par son identifiant.
    let chart_columns = columns.clone();
    let mut saved = serde_json::to_value(chart).context(InvalidOptionSnafu)?;
    if let Some(o) = saved.as_object_mut() {
        o.remove("dataset");
    }
    let table = TableOption {
        dataset: Dataset { source, ids: dataset.ids.clone(), columns },
        header_row: true,
        header_column: true,
        striped: false,
        align: Vec::new(),
        chart: Some(saved),
        chart_columns,
    };
    validate_table(&table)?;
    Ok(table)
}

/// The chart of a table: the original one if it still fits, otherwise bars.
///
/// # Errors
///
/// Fails if the table does not give a consistent bar chart.
pub fn table_to_chart(table: &TableOption, labels: &ChartLabels) -> Result<ChartOption> {
    restored_chart(table, labels).map_or_else(|| bar_chart(table, labels), Ok)
}

/// Le graphique d'origine sur les données actuelles ; `None` s'il n'en reste pas ou ne convient plus.
fn restored_chart(table: &TableOption, labels: &ChartLabels) -> Option<ChartOption> {
    let mut v = table.chart.clone()?;
    v.as_object_mut()?.insert("dataset".into(), serde_json::to_value(&table.dataset).ok()?);
    let mut chart: ChartOption = serde_json::from_value(v).ok()?;
    if let Some(series) = series_by_column(&chart, &table.chart_columns, &table.dataset.columns) {
        chart.series = series;
    }
    sync_series_with_columns(&mut chart);
    normalize(&chart, labels).ok()?;
    Some(chart)
}

/// The kept series put back in front of their column, by identifier.
///
/// Kept series i read `saved[i + 1]`; a column without a kept series
/// gets a new one modelled on its neighbour ([`fresh_series`]). `None` (order by position) outside
/// canonical bars and lines, or if the identifiers are missing or do not
/// match the kept series.
fn series_by_column(chart: &ChartOption, saved: &[String], current: &[String]) -> Option<Vec<Series>> {
    let aligned = saved.len() == chart.series.len() + 1 && current.len() > 1;
    if !aligned || !is_canonical_cartesian(chart) {
        return None;
    }
    let ids = &current[1..];
    let kept = |id: &String| saved[1..].iter().position(|s| s == id).and_then(|i| chart.series.get(i));
    let mut out: Vec<Series> = Vec::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        let series = match kept(id) {
            Some(s) => s.clone(),
            // Modèle : la voisine d'avant, sinon la prochaine série gardée, sinon la première.
            None => fresh_series(out.last().or_else(|| ids[i..].iter().find_map(kept)).or(chart.series.first())?),
        };
        out.push(series);
    }
    Some(out)
}

/// Le graphique en barres d'un tableau : une série par colonne de valeurs.
fn bar_chart(table: &TableOption, labels: &ChartLabels) -> Result<ChartOption> {
    let v = json!({ "dataset": table.dataset, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] });
    let mut chart: ChartOption = serde_json::from_value(v).context(InvalidOptionSnafu)?;
    sync_series_with_columns(&mut chart);
    normalize(&chart, labels)?;
    Ok(chart)
}

#[cfg(test)]
#[path = "table_convert_tests.rs"]
mod table_convert_tests;
