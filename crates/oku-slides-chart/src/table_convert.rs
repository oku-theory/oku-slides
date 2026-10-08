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
    let table = TableOption {
        dataset: Dataset { source },
        header_row: true,
        header_column: true,
        striped: false,
        align: Vec::new(),
    };
    validate_table(&table)?;
    Ok(table)
}

/// Le graphique en barres d'un tableau : une série par colonne de valeurs.
pub fn table_to_chart(table: &TableOption, labels: &ChartLabels) -> Result<ChartOption> {
    let v = json!({ "dataset": table.dataset, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] });
    let mut chart: ChartOption = serde_json::from_value(v).context(InvalidOptionSnafu)?;
    sync_series_with_columns(&mut chart);
    normalize(&chart, labels)?;
    Ok(chart)
}

#[cfg(test)]
#[path = "table_convert_tests.rs"]
mod table_convert_tests;
