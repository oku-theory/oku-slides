//! Reads of a based table: a view's dataset, and the check of a retype against the base.

use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::ops::Range;

use polars::io::SerReader;
use polars::io::parquet::read::ParquetReader;
use polars::prelude::DataFrame;

use super::cells::read_cells;
use super::{ROW_ID, check_footer, guarded, invalid};
use crate::error::ColumnTypeSnafu;
use crate::table::NAMED_ROWS;
use crate::text::cut_chars;
use crate::{BaseRef, CellRead, ColumnType, DataCell, DataError, Dataset, Table};

/// A view's dataset read over a base.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewRead {
    /// The dataset, as [`Table::to_dataset`] gives it.
    pub dataset: Dataset,
    /// Ids of the columns holding base cells that do not convert to the column's type; those
    /// cells read as their text form (`issue-data-cell-type`).
    pub misfit: BTreeSet<String>,
}

/// The dataset of `columns` (these ids, in this order) and of the data rows in `rows`, over the
/// table's base `base`, text cut to `max_cell_chars` characters.
///
/// An overlay entry wins over the base, a cleared one reads empty. A base cell converts to its
/// column's current type without loss; one that does not reads as its text form and names its
/// column in [`ViewRead::misfit`]. Base columns not in `columns` are not read; a column absent
/// from the base reads its overlay, else empty.
///
/// # Errors
///
/// `ColumnsInvalid` or `NoColumn` for the projection; `BaseInvalid` for a base whose footer or
/// content is refused, or when the decoder panics; `NeedsBase` for a table without a base.
pub fn read_view(
    table: &Table,
    base: File,
    columns: &[String],
    rows: Range<usize>,
    max_cell_chars: usize,
) -> Result<ViewRead, DataError> {
    guarded(|| {
        let picked = table.projection(columns)?;
        let ids: Vec<String> = picked.iter().map(|c| c.id.clone()).collect();
        let reader = BaseReader::open(table, base, &ids)?;
        let end = rows.end.min(table.rows.len());
        let start = rows.start.min(end);
        let mut misfit = BTreeSet::new();
        let mut source = vec![picked.iter().map(|c| DataCell::Text(c.name.clone())).collect()];
        let mut row_ids = vec![String::new()];
        for row in (start..end).filter_map(|i| table.rows.get(i)) {
            let mut cells = Vec::with_capacity(picked.len());
            for column in &picked {
                let cell = match table.cell(row, &column.id) {
                    CellRead::Overlay(cell) => cell.clone(),
                    CellRead::Cleared | CellRead::Null => DataCell::Null,
                    CellRead::Base => match reader.cell(row, &column.id, column.ty) {
                        Ok(cell) => cell,
                        Err(text) => {
                            misfit.insert(column.id.clone());
                            text
                        }
                    },
                };
                cells.push(match cell {
                    DataCell::Text(s) => DataCell::Text(cut_chars(&s, max_cell_chars)),
                    other => other,
                });
            }
            source.push(cells);
            row_ids.push(row.to_owned());
        }
        Ok(ViewRead { dataset: Dataset { source, ids: row_ids, columns: ids }, misfit })
    })
}

/// Checks that every base cell of `column` still shown converts to `ty` without loss.
///
/// # Errors
///
/// `NoColumn`; `ColumnType` naming the first rows whose base cell does not convert;
/// `BaseInvalid` as [`read_view`].
pub fn check_column_type(table: &Table, base: File, column: &str, ty: ColumnType) -> Result<(), DataError> {
    guarded(|| {
        let index = table.column_index(column)?;
        let name = table.columns[index].name.as_str();
        let reader = BaseReader::open(table, base, &[column.to_owned()])?;
        let mut refused = Vec::new();
        let mut more = 0_usize;
        for row in table.rows.iter() {
            if table.cell(row, column) != CellRead::Base {
                continue;
            }
            let converts = match reader.raw(row, column) {
                Some((cell, from)) => from.fits(cell) && ty.convert(from, cell).is_some(),
                None => true,
            };
            if converts {
                continue;
            }
            if refused.len() < NAMED_ROWS {
                refused.push(row.to_owned());
            } else {
                more += 1;
            }
        }
        snafu::ensure!(refused.is_empty(), ColumnTypeSnafu { column: name, ty, rows: refused, more });
        Ok(())
    })
}

/// The base cells of some columns, by row id.
pub(super) struct BaseReader {
    /// Base row index of each row id.
    index: HashMap<String, usize>,
    /// The read columns' cells and their type in the file, by column id.
    columns: HashMap<String, (ColumnType, Vec<DataCell>)>,
}

impl BaseReader {
    /// Reads the row ids and the base columns among `columns` from `file`, after its footer check.
    pub(super) fn open(table: &Table, file: File, columns: &[String]) -> Result<Self, DataError> {
        let base = table.base.as_ref().ok_or_else(|| crate::error::NeedsBaseSnafu.build())?;
        check_footer(&file, base)?;
        let wanted: Vec<_> = base.columns.iter().filter(|c| columns.contains(&c.id)).collect();
        let mut names = vec![ROW_ID.to_owned()];
        names.extend(wanted.iter().map(|c| c.id.clone()));
        let frame = ParquetReader::new(file).with_columns(Some(names)).finish().map_err(invalid)?;
        let index = row_index(&frame, base)?;
        let mut read = HashMap::new();
        for column in wanted {
            let cells = read_cells(frame.column(&column.id).map_err(invalid)?, column.ty)?;
            read.insert(column.id.clone(), (column.ty, cells));
        }
        Ok(BaseReader { index, columns: read })
    }

    /// The base cell at `row`, `column` and its type in the file; `None` if the base lacks either.
    pub(super) fn raw(&self, row: &str, column: &str) -> Option<(&DataCell, ColumnType)> {
        let &i = self.index.get(row)?;
        let (ty, cells) = self.columns.get(column)?;
        Some((cells.get(i)?, *ty))
    }

    /// The base cell at `row`, `column` converted to `ty`; `Err` with its text form if it does not
    /// convert. Empty when the base lacks the row or the column.
    pub(super) fn cell(&self, row: &str, column: &str, ty: ColumnType) -> Result<DataCell, DataCell> {
        let Some((cell, from)) = self.raw(row, column) else { return Ok(DataCell::Null) };
        match ty.convert(from, cell) {
            Some(converted) if from.fits(cell) && ty.fits(&converted) => Ok(converted),
            _ => Err(DataCell::Text(cell.as_label())),
        }
    }
}

/// The base row of each row id; a missing or repeated row id refuses the base.
fn row_index(frame: &DataFrame, base: &BaseRef) -> Result<HashMap<String, usize>, DataError> {
    let ids = frame.column(ROW_ID).map_err(invalid)?.str().map_err(invalid)?;
    let mut index = HashMap::with_capacity(base.rows as usize);
    for (i, id) in ids.into_iter().enumerate() {
        let id = id.ok_or_else(|| DataError::base_invalid("empty row id"))?;
        if index.insert(id.to_owned(), i).is_some() {
            return Err(DataError::base_invalid("repeated row id"));
        }
    }
    Ok(index)
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod read_tests;
