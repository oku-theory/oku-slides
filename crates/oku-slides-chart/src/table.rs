//! Tableaux : options d'un widget `host(table)`, données éditées comme celles d'un graphique.
//!
//! Le tableau partage [`Dataset`] et [`DataOp`] avec les graphiques : la
//! ligne 0 est celle des noms de colonnes ; `header-row` décide seulement
//! de son affichage en en-tête.

use oku_slides_i18n::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{ResultExt, ensure};

use crate::data_ops::{DataOp, all_unique, valid_id, validate_ids};
use crate::error::{InconsistentSnafu, InvalidOptionSnafu, Result};
use crate::option::{Cell, Dataset};
use crate::patch::merge_patch;

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
    /// Data; row 0 holds the column names.
    pub dataset: Dataset,
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
/// Fails if the JSON leaves a table's schema, or if the table is empty,
/// ragged, too large or carries invalid identifiers.
pub fn parse_table(v: &Value) -> Result<TableOption> {
    let mut option = TableOption::deserialize(v).context(InvalidOptionSnafu)?;
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
fn tidy(option: &mut TableOption) {
    let width = option.dataset.source.first().map_or(0, Vec::len);
    option.align.truncate(width);
    if option.column_widths.len() != width {
        option.column_widths.clear();
    }
    if option.row_heights.len() != option.dataset.source.len() {
        option.row_heights.clear();
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

fn text_too_long(c: &Cell) -> bool {
    matches!(c, Cell::Text(s) if s.chars().count() > MAX_TABLE_CELL_CHARS)
}

/// Refuse un tableau trop grand, vide, irrégulier ou aux identifiants incohérents.
pub(crate) fn validate_table(option: &TableOption) -> Result<()> {
    let rows = &option.dataset.source;
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
        rows.iter().flatten().all(|c| !matches!(c, Cell::Number(n) if !n.is_finite())),
        InconsistentSnafu { reason: Message::new("chart-table-not-finite") }
    );
    ensure!(
        option.chart.as_ref().is_none_or(Value::is_object),
        InconsistentSnafu { reason: Message::new("chart-table-chart-option") }
    );
    // Each list's length is compared before its values are walked.
    let weights = |w: &[f64], count: usize| w.is_empty() || valid_weights(w, count);
    ensure!(
        weights(&option.column_widths, width) && weights(&option.row_heights, rows.len()),
        InconsistentSnafu { reason: Message::new("chart-table-geometry") }
    );
    ensure!(
        option.chart_columns.len() <= MAX_TABLE_COLUMNS
            && option.chart_columns.iter().all(|id| valid_id(id))
            && all_unique(&option.chart_columns),
        InconsistentSnafu { reason: Message::new("chart-table-chart-columns") }
    );
    validate_ids(&option.dataset)
}

/// Refuse, avant tout clonage, une op qui ferait dépasser les limites.
fn check_growth(option: &TableOption, change: &DataOp) -> Result<()> {
    let rows = option.dataset.source.len();
    let width = option.dataset.source.first().map_or(0, Vec::len);
    let over = match change {
        DataOp::InsertRow { .. } => rows >= MAX_TABLE_ROWS,
        DataOp::InsertColumn { .. } => width >= MAX_TABLE_COLUMNS,
        DataOp::Replace { source } => {
            source.len() > MAX_TABLE_ROWS || source.iter().any(|r| r.len() > MAX_TABLE_COLUMNS)
        }
        _ => false,
    };
    ensure!(!over, InconsistentSnafu { reason: too_large() });
    Ok(())
}

/// `set_data` on a table: applies `change`, realigns `align` and the weights, validates.
///
/// An inserted column or row takes the mean weight; a replaced table keeps the
/// weights only along a dimension whose count is unchanged.
///
/// # Errors
///
/// Fails if the operation is impossible or would exceed the table limits.
pub fn table_data(option: &TableOption, change: &DataOp) -> Result<TableOption> {
    check_growth(option, change)?;
    let mut next = option.clone();
    change.apply(&mut next.dataset)?;
    match change {
        DataOp::InsertColumn { at, .. } if *at < next.align.len() => next.align.insert(*at, ColumnAlign::Auto),
        DataOp::RemoveColumn { at } if *at < next.align.len() => {
            next.align.remove(*at);
        }
        DataOp::Replace { source } => next.align.truncate(source.first().map_or(0, Vec::len)),
        _ => {}
    }
    realign_weights(&mut next, option, change);
    validate_table(&next)?;
    Ok(next)
}

/// Keeps `column-widths` and `row-heights` of `next` aligned with its data after `change` on `before`.
fn realign_weights(next: &mut TableOption, before: &TableOption, change: &DataOp) {
    let mean = |w: &[f64]| {
        let (sum, count) = w.iter().fold((0.0, 0.0), |(s, c), x| (s + x, c + 1.0));
        sum / f64::max(count, 1.0)
    };
    match change {
        DataOp::InsertColumn { at, .. } if !next.column_widths.is_empty() => {
            let m = mean(&next.column_widths);
            next.column_widths.insert((*at).min(next.column_widths.len()), m);
        }
        DataOp::RemoveColumn { at } if *at < next.column_widths.len() => {
            next.column_widths.remove(*at);
        }
        DataOp::InsertRow { at, .. } if !next.row_heights.is_empty() => {
            let m = mean(&next.row_heights);
            next.row_heights.insert((*at).min(next.row_heights.len()), m);
        }
        DataOp::RemoveRow { at } if *at < next.row_heights.len() => {
            next.row_heights.remove(*at);
        }
        DataOp::Replace { .. } => {
            let width = |o: &TableOption| o.dataset.source.first().map_or(0, Vec::len);
            if width(next) != width(before) {
                next.column_widths.clear();
            }
            if next.dataset.source.len() != before.dataset.source.len() {
                next.row_heights.clear();
            }
        }
        _ => {}
    }
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
