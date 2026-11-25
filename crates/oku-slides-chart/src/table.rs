//! Tables: the options of a `host(table)` widget, showing deck data like a chart.
//!
//! A table shares [`Dataset`] with charts as its render input: row 0 holds the
//! column names; `header-row` only decides whether it shows as a header.

use oku_slides_i18n::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{ResultExt, ensure};

use crate::error::{InconsistentSnafu, InvalidOptionSnafu, Result};
use crate::ids::{all_unique, validate_ids};
use crate::patch::merge_patch;
use crate::refs::{check_columns, check_ref_null};
use oku_slides_data::{DataCell, DataId, Dataset, valid_id};

/// Lignes au plus, ligne d'en-tête comprise.
pub const MAX_TABLE_ROWS: usize = 100;
/// Colonnes au plus.
pub const MAX_TABLE_COLUMNS: usize = 20;
/// Caractères au plus dans une cellule de texte.
pub const MAX_TABLE_CELL_CHARS: usize = 1000;
/// Largest column or row weight, so their sum stays finite.
pub const MAX_TABLE_WEIGHT: f64 = 1e6;

/// Are `w` a list of `count` weights, each finite, positive and at most [`MAX_TABLE_WEIGHT`]?
#[must_use]
pub fn valid_weights(w: &[f64], count: usize) -> bool {
    w.len() == count && w.iter().all(|x| x.is_finite() && *x > 0.0 && *x <= MAX_TABLE_WEIGHT)
}

/// Options of a table: its data and their display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TableOption {
    /// The deck data the table reads, by id; never next to inline data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<DataId>,
    /// Column ids of `data` the table reads, in order; all of them if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
    /// Inline data; row 0 holds the column names. Absent when `data` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset: Option<Dataset>,
    /// Row 0 shown as a header.
    #[serde(default = "default_true")]
    pub header_row: bool,
    /// Column 0 shown as a header (row labels).
    #[serde(default)]
    pub header_column: bool,
    /// Every other data row on a background.
    #[serde(default)]
    pub striped: bool,
    /// Alignment of each column; a column without an entry is `auto`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub align: Vec<ColumnAlign>,
    /// Options of the original chart, without `dataset`, kept to restore it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart: Option<Value>,
    /// Column identifiers of the original chart (`dataset.columns`).
    ///
    /// Series i reads column i + 1 there. Identifiers are distinct; empty without `chart`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chart_columns: Vec<String>,
    /// Width of each column as a relative weight; empty: sized on the content.
    ///
    /// One weight per column when given (see [`valid_weights`]). A list whose
    /// length no longer matches, as a merge of concurrent edits leaves it, is dropped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_widths: Vec<f64>,
    /// Least height of each row, as a weight; empty: the content's.
    ///
    /// The weight is a share of the table height. One weight per row (header
    /// included) when given, dropped like `column-widths` when its length no
    /// longer matches; a row taller than its weight keeps its content's height.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_heights: Vec<f64>,
}

fn default_true() -> bool {
    true
}

impl TableOption {
    /// Does the table carry its own data (`dataset`)?
    #[must_use]
    pub fn has_inline(&self) -> bool {
        self.dataset.is_some()
    }
}

/// Refuses a table that has neither a data ref nor inline data.
///
/// # Errors
///
/// Fails with `chart-table-empty` when both `data` and `dataset` are absent.
pub fn require_data_in_table(option: &TableOption) -> Result<()> {
    ensure!(
        option.data.is_some() || option.dataset.is_some(),
        InconsistentSnafu { reason: Message::new("chart-table-empty") }
    );
    Ok(())
}

/// Alignment of a table column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ColumnAlign {
    /// At the end for a column of numbers, at the start otherwise.
    Auto,
    /// At the start of the line (left in a left-to-right script).
    Start,
    /// Centered.
    Center,
    /// At the end of the line (right in a left-to-right script).
    End,
}

/// Reads and validates a table's options.
///
/// # Errors
///
/// Fails if the JSON leaves a table's schema, sets `data` to `null`, has
/// invalid `columns`, or if the inline table is empty, ragged, too large or
/// carries invalid identifiers. Options without any data are left to the
/// caller ([`require_data_in_table`]).
pub fn parse_table(v: &Value) -> Result<TableOption> {
    check_ref_null(v)?;
    let mut option = TableOption::deserialize(v).context(InvalidOptionSnafu)?;
    check_columns(option.data.as_ref(), option.columns.as_deref())?;
    tidy(&mut option);
    validate_table(&option)?;
    Ok(option)
}

/// Fits the options to the data, as a merge of concurrent edits may leave them.
///
/// Truncates `align` to the table width (an extra entry has no column) and
/// drops a weight list whose length does not match. Removes the kept chart's
/// `dataset` (the data are the table's: a kept `dataset` would be stale).
/// Without a kept chart, its column identifiers have no use.
///
/// Without inline data the table's size is the data's: nothing is cut.
fn tidy(option: &mut TableOption) {
    if let Some(dataset) = &option.dataset {
        let width = dataset.source.first().map_or(0, Vec::len);
        option.align.truncate(width);
        if option.column_widths.len() != width {
            option.column_widths.clear();
        }
        if option.row_heights.len() != dataset.source.len() {
            option.row_heights.clear();
        }
    }
    if option.chart.is_none() {
        option.chart_columns.clear();
    }
    if let Some(chart) = option.chart.as_mut().and_then(Value::as_object_mut) {
        chart.remove("dataset");
    }
}

fn too_large() -> Message {
    Message::new("chart-table-too-large").arg("rows", MAX_TABLE_ROWS).arg("columns", MAX_TABLE_COLUMNS)
}

fn text_too_long(c: &DataCell) -> bool {
    matches!(c, DataCell::Text(s) if s.chars().count() > MAX_TABLE_CELL_CHARS)
}

/// Refuses a table too large, empty, ragged or with inconsistent identifiers.
///
/// Without inline data only the display options are checked.
pub(crate) fn validate_table(option: &TableOption) -> Result<()> {
    ensure!(
        option.chart.as_ref().is_none_or(Value::is_object),
        InconsistentSnafu { reason: Message::new("chart-table-chart-option") }
    );
    ensure!(
        option.chart_columns.len() <= MAX_TABLE_COLUMNS
            && option.chart_columns.iter().all(|id| valid_id(id))
            && all_unique(&option.chart_columns),
        InconsistentSnafu { reason: Message::new("chart-table-chart-columns") }
    );
    let Some(dataset) = &option.dataset else {
        let weights = |w: &[f64]| w.is_empty() || valid_weights(w, w.len());
        ensure!(
            weights(&option.column_widths) && weights(&option.row_heights),
            InconsistentSnafu { reason: Message::new("chart-table-geometry") }
        );
        return Ok(());
    };
    let rows = &dataset.source;
    let width = rows.first().map_or(0, Vec::len);
    // Tailles d'abord : un tableau énorme est refusé sans être parcouru.
    ensure!(rows.len() <= MAX_TABLE_ROWS && width <= MAX_TABLE_COLUMNS, InconsistentSnafu { reason: too_large() });
    ensure!(width > 0, InconsistentSnafu { reason: Message::new("chart-table-empty") });
    ensure!(rows.iter().all(|r| r.len() == width), InconsistentSnafu { reason: Message::new("chart-table-ragged") });
    ensure!(
        !rows.iter().flatten().any(text_too_long),
        InconsistentSnafu { reason: Message::new("chart-table-cell-too-long").arg("max", MAX_TABLE_CELL_CHARS) }
    );
    ensure!(
        rows.iter().flatten().all(|c| !matches!(c, DataCell::Number(n) if !n.is_finite())),
        InconsistentSnafu { reason: Message::new("chart-table-not-finite") }
    );
    // Each list's length is compared before its values are walked.
    let weights = |w: &[f64], count: usize| w.is_empty() || valid_weights(w, count);
    ensure!(
        weights(&option.column_widths, width) && weights(&option.row_heights, rows.len()),
        InconsistentSnafu { reason: Message::new("chart-table-geometry") }
    );
    validate_ids(dataset)
}

/// Merge patch (RFC 7396) on the options outside `dataset`; returns the inverse patch.
///
/// # Errors
///
/// Fails if the patch touches `dataset`, is not an object or makes the table invalid.
pub fn patch_table(option: &TableOption, patch: &Value) -> Result<(TableOption, Value)> {
    let (mut next, inverse) = merge_patch(option, patch)?;
    // An explicit edit with the wrong number of weights is refused, not dropped like a merge's.
    validate_table(&next)?;
    tidy(&mut next);
    validate_table(&next)?;
    Ok((next, inverse))
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
