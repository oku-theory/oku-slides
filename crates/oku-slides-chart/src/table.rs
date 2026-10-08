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

use crate::data_ops::DataOp;
use crate::error::{InconsistentSnafu, InvalidOptionSnafu, Result};
use crate::option::{Cell, Dataset};
use crate::patch::merge_patch;

/// Lignes au plus, ligne d'en-tête comprise.
pub const MAX_TABLE_ROWS: usize = 100;
/// Colonnes au plus.
pub const MAX_TABLE_COLUMNS: usize = 20;
/// Caractères au plus dans une cellule de texte.
pub const MAX_TABLE_CELL_CHARS: usize = 1000;

/// Options d'un tableau : ses données et leur affichage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TableOption {
    /// Données ; ligne 0 = noms de colonnes.
    pub dataset: Dataset,
    /// Ligne 0 affichée en en-tête.
    #[serde(default = "default_true")]
    pub header_row: bool,
    /// Colonne 0 affichée en en-tête (libellés des lignes).
    #[serde(default)]
    pub header_column: bool,
    /// Une ligne de données sur deux sur fond.
    #[serde(default)]
    pub striped: bool,
    /// Alignement de chaque colonne ; une colonne sans entrée vaut `auto`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub align: Vec<ColumnAlign>,
}

fn default_true() -> bool {
    true
}

/// Alignement d'une colonne de tableau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ColumnAlign {
    /// À la fin pour une colonne de nombres, au début sinon.
    Auto,
    /// Au début de la ligne (à gauche en français).
    Start,
    /// Centré.
    Center,
    /// À la fin de la ligne (à droite en français).
    End,
}

/// Lit et valide les options d'un tableau.
pub fn parse_table(v: &Value) -> Result<TableOption> {
    let option = TableOption::deserialize(v).context(InvalidOptionSnafu)?;
    validate_table(&option)?;
    Ok(option)
}

fn too_large() -> Message {
    Message::new("chart-table-too-large").arg("rows", MAX_TABLE_ROWS).arg("columns", MAX_TABLE_COLUMNS)
}

fn text_too_long(c: &Cell) -> bool {
    matches!(c, Cell::Text(s) if s.chars().count() > MAX_TABLE_CELL_CHARS)
}

/// Refuse un tableau trop grand, vide, irrégulier ou mal aligné.
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
        option.align.len() <= width,
        InconsistentSnafu { reason: Message::new("chart-table-align").arg("width", width) }
    );
    Ok(())
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

/// `set_data` sur un tableau : applique `change`, recale `align`, valide.
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

/// Merge patch (RFC 7396) sur les options hors `dataset` ; rend le patch inverse.
pub fn patch_table(option: &TableOption, patch: &Value) -> Result<(TableOption, Value)> {
    let (next, inverse) = merge_patch(option, patch)?;
    validate_table(&next)?;
    Ok((next, inverse))
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
