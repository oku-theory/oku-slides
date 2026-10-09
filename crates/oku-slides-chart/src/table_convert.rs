//! Graphique ↔ tableau : les mêmes données, l'autre widget.

use oku_slides_i18n::Message;
use serde_json::json;
use snafu::ResultExt;

use crate::canonical::sync_series_with_columns;
use crate::error::{DataOpSnafu, InvalidOptionSnafu, Result};
use crate::labels::ChartLabels;
use crate::model::normalize;
use crate::option::{Cell, ChartOption, Dataset};
use crate::table::{TableOption, validate_table};

/// Le tableau des données d'un graphique, lignes courtes complétées.
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
    };
    validate_table(&table)?;
    Ok(table)
}

/// Le graphique d'un tableau : celui d'origine s'il convient encore, sinon des barres.
pub fn table_to_chart(table: &TableOption, labels: &ChartLabels) -> Result<ChartOption> {
    restored_chart(table, labels).map_or_else(|| bar_chart(table, labels), Ok)
}

/// Le graphique d'origine sur les données actuelles ; `None` s'il n'en reste pas ou ne convient plus.
fn restored_chart(table: &TableOption, labels: &ChartLabels) -> Option<ChartOption> {
    let mut v = table.chart.clone()?;
    v.as_object_mut()?.insert("dataset".into(), serde_json::to_value(&table.dataset).ok()?);
    let mut chart: ChartOption = serde_json::from_value(v).ok()?;
    sync_series_with_columns(&mut chart);
    normalize(&chart, labels).ok()?;
    Some(chart)
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
