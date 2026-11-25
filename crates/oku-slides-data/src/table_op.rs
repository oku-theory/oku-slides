//! Id-based edits of a table: each names rows and columns by id, so concurrent edits keep.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ensure};

use crate::error::{
    Bound, CellTypeSnafu, DataError, IdInvalidSnafu, IdMissingSnafu, IdTakenSnafu, NonFiniteSnafu, TooLargeSnafu,
};
use crate::table::{CellKey, key};
use crate::{ColumnType, DataCell, IdGen, NewTable, Table, bounded, clean_label, limits, valid_id};

/// One edit of a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum TableOp {
    /// Sets one cell; an empty value clears it.
    SetCell {
        /// Row id.
        row: String,
        /// Column id.
        column: String,
        /// New value, which must fit the column's type.
        value: DataCell,
    },
    /// Renames a column.
    RenameColumn {
        /// Column id.
        column: String,
        /// New displayed name.
        name: String,
    },
    /// Inserts a row.
    InsertRow {
        /// Id of the new row; drawn if left out.
        #[serde(default)]
        id: Option<String>,
        /// Row it follows; first if left out.
        #[serde(default)]
        after: Option<String>,
        /// Its cells, by column id; the others stay empty.
        #[serde(default, deserialize_with = "bounded::btree::<_, _, _, { limits::MAX_COLUMNS }>")]
        cells: BTreeMap<String, DataCell>,
    },
    /// Removes a row and its cells.
    RemoveRow {
        /// Row id.
        row: String,
    },
    /// Inserts a column, typed from its cells.
    InsertColumn {
        /// Id of the new column; drawn if left out.
        #[serde(default)]
        id: Option<String>,
        /// Column it follows; first if left out.
        #[serde(default)]
        after: Option<String>,
        /// Displayed name.
        name: String,
        /// Its cells, by row id; the others stay empty.
        #[serde(default, deserialize_with = "bounded::btree::<_, _, _, { limits::MAX_ROWS }>")]
        cells: BTreeMap<String, DataCell>,
    },
    /// Removes a column and its cells.
    RemoveColumn {
        /// Column id.
        column: String,
    },
    /// Replaces the whole table, starting a new generation.
    Replace {
        /// The new table.
        table: NewTable,
    },
}

/// Row and column ids a batch gives for one data, kept free when ids are drawn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GivenIds {
    /// Row ids given by the batch.
    pub rows: BTreeSet<String>,
    /// Column ids given by the batch.
    pub columns: BTreeSet<String>,
}

impl GivenIds {
    /// Records the row and column ids `op` gives.
    pub fn record(&mut self, op: &TableOp) {
        match op {
            TableOp::InsertRow { id: Some(id), .. } => {
                self.rows.insert(id.clone());
            }
            TableOp::InsertColumn { id: Some(id), .. } => {
                self.columns.insert(id.clone());
            }
            TableOp::Replace { table } => self.record_table(table),
            _ => {}
        }
    }

    /// Records the row and column ids `table` gives.
    pub fn record_table(&mut self, table: &NewTable) {
        self.rows.extend(table.rows.iter().filter_map(|r| r.id.clone()));
        self.columns.extend(table.columns.iter().filter_map(|c| c.id.clone()));
    }
}

impl TableOp {
    /// Draws the ids left out, unused in `table` and not in `given`.
    pub fn fill_ids(&mut self, table: &Table, given: &GivenIds, ids: &mut IdGen<'_>) {
        match self {
            TableOp::InsertRow { id: id @ None, .. } => {
                *id = Some(ids(&|candidate| table.rows.contains(candidate) || given.rows.contains(candidate)));
            }
            TableOp::InsertColumn { id: id @ None, .. } => {
                *id = Some(ids(&|candidate| {
                    table.columns.iter().any(|c| c.id == candidate) || given.columns.contains(candidate)
                }));
            }
            TableOp::Replace { table: new } => new.fill_ids(Some(table), given, ids),
            _ => {}
        }
    }

    /// Cells the edit writes, to budget a batch.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        match self {
            TableOp::SetCell { .. } => 1,
            TableOp::InsertRow { cells, .. } | TableOp::InsertColumn { cells, .. } => cells.len(),
            TableOp::Replace { table } => table.cell_count(),
            TableOp::RenameColumn { .. } | TableOp::RemoveRow { .. } | TableOp::RemoveColumn { .. } => 0,
        }
    }
}

impl Table {
    /// Applies `op`, or leaves the table unchanged and says why not.
    ///
    /// Growing edits are refused past the bounds of a data; removals always succeed.
    ///
    /// # Errors
    ///
    /// A [`DataError`] naming the unknown row or column, the refused id or value, or the bound.
    pub fn apply(&mut self, op: &TableOp) -> Result<(), DataError> {
        match op {
            TableOp::SetCell { row, column, value } => self.set_cell(row, column, value),
            TableOp::RenameColumn { column, name } => {
                let index = self.column_index(column)?;
                self.columns[index].name = clean_label(name);
                Ok(())
            }
            TableOp::InsertRow { id, after, cells } => self.insert_row(id.as_deref(), after.as_deref(), cells),
            TableOp::RemoveRow { row } => {
                let index = self.row_index(row)?;
                self.rows.remove(index);
                self.remove_row_cells(row);
                Ok(())
            }
            TableOp::InsertColumn { id, after, name, cells } => {
                self.insert_column(id.as_deref(), after.as_deref(), name, cells)
            }
            TableOp::RemoveColumn { column } => {
                let index = self.column_index(column)?;
                self.columns.remove(index);
                self.remove_cells(|key| key.column == *column);
                Ok(())
            }
            TableOp::Replace { table } => {
                let (table, non_finite) = table.clone().into_table()?;
                ensure!(non_finite == 0, NonFiniteSnafu { count: non_finite });
                *self = table;
                Ok(())
            }
        }
    }

    fn set_cell(&mut self, row: &str, column: &str, value: &DataCell) -> Result<(), DataError> {
        self.row_index(row)?;
        let index = self.column_index(column)?;
        check_cell(value, &self.columns[index].name, self.columns[index].ty)?;
        let key = key(row, column);
        match value {
            DataCell::Null if self.base.is_none() => {
                self.cells.remove(&key);
            }
            _ => {
                ensure!(
                    self.cells.contains_key(&key) || self.cells.len() < limits::MAX_OVERLAY,
                    TooLargeSnafu { what: Bound::Overlay, max: limits::MAX_OVERLAY }
                );
                self.cells.insert(key, value.clone());
            }
        }
        Ok(())
    }

    fn insert_row(
        &mut self,
        id: Option<&str>,
        after: Option<&str>,
        cells: &BTreeMap<String, DataCell>,
    ) -> Result<(), DataError> {
        let id = new_id_checked(id, |id| self.rows.contains(id))?;
        let at = match after {
            Some(after) => self.row_index(after)? + 1,
            None => 0,
        };
        ensure!(self.rows.len() < limits::MAX_ROWS, TooLargeSnafu { what: Bound::Rows, max: limits::MAX_ROWS });
        for (column, cell) in cells {
            let index = self.column_index(column)?;
            check_cell(cell, &self.columns[index].name, self.columns[index].ty)?;
        }
        let written: Vec<_> = cells.iter().filter(|(_, cell)| !matches!(cell, DataCell::Null)).collect();
        self.check_overlay_room(written.len())?;
        self.rows.insert(at, id.to_owned());
        for (column, cell) in written {
            self.cells.insert(key(id, column), cell.clone());
        }
        Ok(())
    }

    fn insert_column(
        &mut self,
        id: Option<&str>,
        after: Option<&str>,
        name: &str,
        cells: &BTreeMap<String, DataCell>,
    ) -> Result<(), DataError> {
        let id = new_id_checked(id, |id| self.columns.iter().any(|c| c.id == id))?;
        let at = match after {
            Some(after) => self.column_index(after)? + 1,
            None => 0,
        };
        ensure!(
            self.columns.len() < limits::MAX_COLUMNS,
            TooLargeSnafu { what: Bound::Columns, max: limits::MAX_COLUMNS }
        );
        let name = clean_label(name);
        let ty = ColumnType::infer(cells.values());
        for (row, cell) in cells {
            self.row_index(row)?;
            check_cell(cell, &name, ty)?;
        }
        let written: Vec<_> = cells.iter().filter(|(_, cell)| !matches!(cell, DataCell::Null)).collect();
        self.check_overlay_room(written.len())?;
        self.columns.insert(at, crate::Column { id: id.to_owned(), name, ty });
        for (row, cell) in written {
            self.cells.insert(key(row, id), cell.clone());
        }
        Ok(())
    }

    /// Removes the overlay entries of the row `row`: keys sort by row first, so they are one
    /// range, found in O(log n).
    fn remove_row_cells(&mut self, row: &str) {
        let start = CellKey { row: row.to_owned(), column: String::new() };
        let keys: Vec<CellKey> =
            self.cells.range(start..).map(|(key, _)| key).take_while(|key| key.row == row).cloned().collect();
        for key in keys {
            self.cells.remove(&key);
        }
    }

    /// Removes the overlay entries whose key matches `drop`.
    fn remove_cells(&mut self, drop: impl Fn(&CellKey) -> bool) {
        let keys: Vec<CellKey> = self.cells.keys().filter(|key| drop(key)).cloned().collect();
        for key in keys {
            self.cells.remove(&key);
        }
    }

    /// Refuses `added` new overlay entries past `MAX_OVERLAY`.
    fn check_overlay_room(&self, added: usize) -> Result<(), DataError> {
        ensure!(
            self.cells.len().saturating_add(added) <= limits::MAX_OVERLAY,
            TooLargeSnafu { what: Bound::Overlay, max: limits::MAX_OVERLAY }
        );
        Ok(())
    }
}

/// The id of a new row or column: given, valid and not `taken`.
fn new_id_checked(id: Option<&str>, taken: impl Fn(&str) -> bool) -> Result<&str, DataError> {
    let id = id.context(IdMissingSnafu)?;
    ensure!(valid_id(id), IdInvalidSnafu { id });
    ensure!(!taken(id), IdTakenSnafu { id });
    Ok(id)
}

/// Refuses a non-finite number, a text over `MAX_TEXT_BYTES`, then a cell that does not fit
/// the column `name` of type `ty`.
pub(crate) fn check_cell(cell: &DataCell, name: &str, ty: ColumnType) -> Result<(), DataError> {
    ensure!(!is_non_finite(cell), NonFiniteSnafu { count: 1_usize });
    ensure!(
        !matches!(cell, DataCell::Text(s) if s.len() > limits::MAX_TEXT_BYTES),
        TooLargeSnafu { what: Bound::Text, max: limits::MAX_TEXT_BYTES }
    );
    ensure!(ty.fits(cell), CellTypeSnafu { column: name, ty });
    Ok(())
}

/// A NaN or infinite number.
pub(crate) fn is_non_finite(cell: &DataCell) -> bool {
    matches!(cell, DataCell::Number(n) if !n.is_finite())
}

#[cfg(test)]
#[path = "table_op_tests.rs"]
mod table_op_tests;
