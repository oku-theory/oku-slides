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

/// Tronque `align` à la largeur du tableau (une entrée de trop n'a pas de
/// colonne) et retire le `dataset` du graphique gardé (les données sont
/// celles du tableau : un `dataset` gardé serait périmé). Sans graphique
/// gardé, ses identifiants de colonnes n'ont plus d'objet.
fn tidy(option: &mut TableOption) {
    option.align.truncate(option.dataset.source.first().map_or(0, Vec::len));
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
    // Longueur d'abord : une liste énorme est refusée sans être parcourue.
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

/// `set_data` on a table: applies `change`, realigns `align`, validates.
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
    validate_table(&next)?;
    Ok(next)
}

/// Merge patch (RFC 7396) on the options outside `dataset`; returns the inverse patch.
///
/// # Errors
///
/// Fails if the patch touches `dataset`, is not an object or makes the table invalid.
pub fn patch_table(option: &TableOption, patch: &Value) -> Result<(TableOption, Value)> {
    let (mut next, inverse) = merge_patch(option, patch)?;
    tidy(&mut next);
    validate_table(&next)?;
    Ok((next, inverse))
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
