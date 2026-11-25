//! Chart ↔ table: the same data, the other widget.

use oku_slides_i18n::Message;
use serde_json::json;
use snafu::ResultExt;

use crate::canonical::{fresh_series, is_canonical_cartesian, sync_series_with_columns};
use crate::error::{InconsistentSnafu, InvalidOptionSnafu, Result};
use crate::labels::ChartLabels;
use crate::model::normalize;
use crate::option::{ChartOption, Series};
use crate::table::{TableOption, validate_table};
use oku_slides_data::{DataCell, Dataset};

/// The data table of a chart, short rows padded.
///
/// # Errors
///
/// Fails if the chart has no `dataset` or if its data exceed the
/// limits of a table.
pub fn chart_to_table(chart: &ChartOption) -> Result<TableOption> {
    let dataset =
        chart.dataset.as_ref().ok_or_else(|| InconsistentSnafu { reason: Message::new("chart-no-dataset") }.build())?;
    let width = dataset.source.iter().map(Vec::len).max().unwrap_or(0);
    let source = dataset
        .source
        .iter()
        .map(|r| {
            let mut r = r.clone();
            r.resize(width, DataCell::Null);
            r
        })
        .collect();
    // A padded row 0 changes the width: its column ids are dropped.
    let columns = if dataset.columns.len() == width { dataset.columns.clone() } else { Vec::new() };
    // On the way back, each series finds its column again by its id.
    let chart_columns = columns.clone();
    let mut saved = serde_json::to_value(chart).context(InvalidOptionSnafu)?;
    if let Some(o) = saved.as_object_mut() {
        // The data, their ref and projection move to the table itself.
        for key in ["dataset", "data", "columns"] {
            o.remove(key);
        }
    }
    let table = TableOption {
        data: chart.data.clone(),
        columns: chart.columns.clone(),
        dataset: Some(Dataset { source, ids: dataset.ids.clone(), columns }),
        header_row: true,
        header_column: true,
        striped: false,
        align: Vec::new(),
        chart: Some(saved),
        chart_columns,
        column_widths: Vec::new(),
        row_heights: Vec::new(),
    };
    validate_table(&table)?;
    Ok(table)
}

/// The chart of a table: the original one if it still fits, otherwise bars.
///
/// # Errors
///
/// Fails if the table has no `dataset` or does not give a consistent bar chart.
pub fn table_to_chart(table: &TableOption, labels: &ChartLabels) -> Result<ChartOption> {
    let dataset =
        table.dataset.as_ref().ok_or_else(|| InconsistentSnafu { reason: Message::new("chart-no-dataset") }.build())?;
    let mut chart = restored_chart(table, dataset, labels).map_or_else(|| bar_chart(dataset, labels), Ok)?;
    chart.data.clone_from(&table.data);
    chart.columns.clone_from(&table.columns);
    Ok(chart)
}

/// The original chart on the current data; `None` if there is none or it no longer fits.
fn restored_chart(table: &TableOption, dataset: &Dataset, labels: &ChartLabels) -> Option<ChartOption> {
    let mut v = table.chart.clone()?;
    v.as_object_mut()?.insert("dataset".into(), serde_json::to_value(dataset).ok()?);
    let mut chart: ChartOption = serde_json::from_value(v).ok()?;
    if let Some(series) = series_by_column(&chart, &table.chart_columns, &dataset.columns) {
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
            // Model: the previous neighbour, else the next kept series, else the first.
            None => fresh_series(out.last().or_else(|| ids[i..].iter().find_map(kept)).or(chart.series.first())?),
        };
        out.push(series);
    }
    Some(out)
}

/// The bar chart of a table: one series per value column.
fn bar_chart(dataset: &Dataset, labels: &ChartLabels) -> Result<ChartOption> {
    let v = json!({ "dataset": dataset, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] });
    let mut chart: ChartOption = serde_json::from_value(v).context(InvalidOptionSnafu)?;
    sync_series_with_columns(&mut chart);
    normalize(&chart, labels)?;
    Ok(chart)
}

#[cfg(test)]
#[path = "table_convert_tests.rs"]
mod table_convert_tests;
