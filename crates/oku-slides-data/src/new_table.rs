//! A table as given by a client: ids optional, cells by row, checked into a [`Table`].

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ensure};

use crate::error::{
    BaseRowCellsSnafu, Bound, DataError, IdInvalidSnafu, IdMissingSnafu, IdTakenSnafu, NoColumnSnafu, NoRowSnafu,
    RowWidthSnafu, TooLargeSnafu,
};
use crate::table::{CellKey, Column, key};
use crate::table_op::{GivenIds, check_cell, is_non_finite};
use crate::{BaseRef, ColumnType, DataCell, Dataset, IdGen, Rows, Table, bounded, clean_label, limits, valid_id};

/// A whole table to store: what `Replace` and a new data carry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewTable {
    /// Generation id; drawn if left out.
    #[serde(default)]
    pub generation: Option<String>,
    /// Columns, in display order.
    #[serde(deserialize_with = "bounded::vec::<_, _, { limits::MAX_COLUMNS }>")]
    pub columns: Vec<NewColumn>,
    /// Rows, in display order.
    #[serde(deserialize_with = "bounded::vec::<_, _, { limits::MAX_ROWS }>")]
    pub rows: Vec<NewRow>,
    /// The base file (host only; its rows then carry ids and no cells).
    #[serde(default)]
    pub base: Option<BaseRef>,
    /// Cells over the base (host only), keyed `<row id>/<column id>`.
    #[serde(default, deserialize_with = "bounded::map::<_, _, _, { limits::MAX_OVERLAY }>")]
    #[schemars(with = "BTreeMap<String, DataCell>")]
    pub overlay: imbl::OrdMap<CellKey, DataCell>,
}

/// A column of a new table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewColumn {
    /// Column id; drawn if left out.
    #[serde(default)]
    pub id: Option<String>,
    /// Displayed name.
    pub name: String,
    /// Type; inferred from its cells if left out.
    #[serde(rename = "type", default)]
    pub ty: Option<ColumnType>,
}

/// A row of a new table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewRow {
    /// Row id; drawn if left out.
    #[serde(default)]
    pub id: Option<String>,
    /// Cells by column position; missing ones are empty.
    #[serde(default, deserialize_with = "bounded::vec::<_, _, { limits::MAX_COLUMNS }>")]
    pub cells: Vec<DataCell>,
}

/// A [`NewTable`] read from a dataset, with what reading it changed.
#[derive(Debug, Clone, PartialEq)]
pub struct FromDataset {
    /// The table, without ids.
    pub table: NewTable,
    /// NaN and infinite numbers made empty.
    pub non_finite: usize,
    /// Rows longer than the header, cut to its width.
    pub cut_rows: usize,
}

/// What one row of a [`NewTable`] stores once checked: overlay cells and their text bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RowLoad {
    /// Non-empty cells.
    pub cells: usize,
    /// UTF-8 bytes of its text cells, numbers and booleans of a Text column in display form.
    pub text: usize,
}

impl NewTable {
    /// What each row stores once checked, in order; an untyped column counts as Text.
    #[must_use]
    pub fn row_loads(&self) -> Vec<RowLoad> {
        self.rows
            .iter()
            .map(|row| {
                row.cells.iter().zip(&self.columns).fold(RowLoad::default(), |load, (cell, column)| {
                    let text = match cell {
                        DataCell::Null => return load,
                        DataCell::Text(s) => s.len(),
                        _ if column.ty.is_none_or(|ty| ty == ColumnType::Text) => cell.as_label().len(),
                        _ => 0,
                    };
                    RowLoad { cells: load.cells + 1, text: load.text + text }
                })
            })
            .collect()
    }

    /// The table of a dataset whose row 0 names the columns; its ids are ignored.
    #[must_use]
    pub fn from_dataset(ds: &Dataset) -> FromDataset {
        let mut non_finite = 0;
        let mut cut_rows = 0;
        let Some((header, body)) = ds.source.split_first() else {
            return FromDataset { table: NewTable::empty(), non_finite, cut_rows };
        };
        let width = header.len();
        let rows: Vec<NewRow> = body
            .iter()
            .map(|row| {
                if row.len() > width {
                    cut_rows += 1;
                }
                let mut cells: Vec<DataCell> = row
                    .iter()
                    .take(width)
                    .map(|cell| {
                        if is_non_finite(cell) {
                            non_finite += 1;
                            DataCell::Null
                        } else {
                            cell.clone()
                        }
                    })
                    .collect();
                cells.resize(width, DataCell::Null);
                NewRow { id: None, cells }
            })
            .collect();
        let columns = header
            .iter()
            .enumerate()
            .map(|(j, name)| NewColumn {
                id: None,
                name: clean_label(&name.as_label()),
                ty: Some(ColumnType::infer(rows.iter().filter_map(|row| row.cells.get(j)))),
            })
            .collect();
        FromDataset { table: NewTable { columns, rows, ..NewTable::empty() }, non_finite, cut_rows }
    }

    /// A table without columns or rows.
    fn empty() -> NewTable {
        NewTable { generation: None, columns: Vec::new(), rows: Vec::new(), base: None, overlay: imbl::OrdMap::new() }
    }

    /// Refuses a table past the rows, columns or cells of a data.
    ///
    /// # Errors
    ///
    /// `TooLarge` naming the bound passed.
    pub fn check_bounds(&self) -> Result<(), DataError> {
        ensure!(self.rows.len() <= limits::MAX_ROWS, TooLargeSnafu { what: Bound::Rows, max: limits::MAX_ROWS });
        ensure!(
            self.columns.len() <= limits::MAX_COLUMNS,
            TooLargeSnafu { what: Bound::Columns, max: limits::MAX_COLUMNS }
        );
        ensure!(
            self.cell_count() <= limits::MAX_OVERLAY,
            TooLargeSnafu { what: Bound::Overlay, max: limits::MAX_OVERLAY }
        );
        Ok(())
    }

    /// Cells given: every row cell, empty ones included, and every overlay entry.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.rows.iter().map(|row| row.cells.len()).sum::<usize>() + self.overlay.len()
    }

    /// Draws the ids left out; with `keep`, reuses its column ids by name and row ids by position.
    ///
    /// A drawn id is used nowhere in this table or in `keep`.
    pub fn fill_ids(&mut self, keep: Option<&Table>, given: &GivenIds, ids: &mut IdGen<'_>) {
        if self.generation.is_none() {
            let old = keep.map(|t| t.generation.as_str());
            self.generation = Some(ids(&|candidate| Some(candidate) == old));
        }
        let mut used: BTreeSet<String> = self.columns.iter().filter_map(|c| c.id.clone()).collect();
        let kept: BTreeSet<&str> = keep.map(|t| t.columns.iter().map(|c| c.id.as_str()).collect()).unwrap_or_default();
        for column in self.columns.iter_mut().filter(|c| c.id.is_none()) {
            let reused = keep
                .and_then(|t| {
                    t.columns
                        .iter()
                        .find(|c| c.name == column.name && !used.contains(&c.id) && !given.columns.contains(&c.id))
                })
                .map(|c| c.id.clone());
            let id = reused.unwrap_or_else(|| {
                ids(&|candidate| {
                    used.contains(candidate) || kept.contains(candidate) || given.columns.contains(candidate)
                })
            });
            used.insert(id.clone());
            column.id = Some(id);
        }
        let mut used: BTreeSet<String> = self.rows.iter().filter_map(|r| r.id.clone()).collect();
        let kept: BTreeSet<&str> = keep.map(|t| t.rows.iter().collect()).unwrap_or_default();
        for (i, row) in self.rows.iter_mut().enumerate() {
            if row.id.is_some() {
                continue;
            }
            let reused = keep
                .and_then(|t| t.rows.get(i))
                .filter(|id| !used.contains(*id) && !given.rows.contains(*id))
                .map(str::to_owned);
            let id = reused.unwrap_or_else(|| {
                ids(&|candidate| used.contains(candidate) || kept.contains(candidate) || given.rows.contains(candidate))
            });
            used.insert(id.clone());
            row.id = Some(id);
        }
    }

    /// The stored table, and how many NaN or infinite numbers it made empty.
    ///
    /// A Text column stores its other cells in display form; an untyped column is inferred.
    ///
    /// # Errors
    ///
    /// `IdMissing` for an id left out, `IdInvalid`, `IdTaken` for a repeated id, `RowWidth`,
    /// `TooLarge`, `CellType`, or `NoRow`/`NoColumn` for an overlay key.
    pub fn into_table(self) -> Result<(Table, usize), DataError> {
        self.check_bounds()?;
        let NewTable { generation, columns, rows, base, overlay } = self;
        let generation = checked_id(generation)?;
        let width = columns.len();
        let mut non_finite = 0;
        let mut row_ids = Vec::with_capacity(rows.len());
        let mut row_set = BTreeSet::new();
        let mut grid = Vec::with_capacity(rows.len());
        for row in rows {
            let id = checked_id(row.id)?;
            ensure!(row_set.insert(id.clone()), IdTakenSnafu { id });
            ensure!(row.cells.len() <= width, RowWidthSnafu { max: width });
            ensure!(base.is_none() || row.cells.is_empty(), BaseRowCellsSnafu);
            let cells: Vec<DataCell> = row
                .cells
                .into_iter()
                .map(|cell| {
                    if is_non_finite(&cell) {
                        non_finite += 1;
                        DataCell::Null
                    } else {
                        cell
                    }
                })
                .collect();
            row_ids.push(id.clone());
            grid.push((id, cells));
        }
        let mut stored = Vec::with_capacity(width);
        let mut column_set = BTreeSet::new();
        for (j, column) in columns.into_iter().enumerate() {
            let id = checked_id(column.id)?;
            ensure!(column_set.insert(id.clone()), IdTakenSnafu { id });
            let ty = column.ty.unwrap_or_else(|| ColumnType::infer(grid.iter().filter_map(|(_, cells)| cells.get(j))));
            stored.push(Column { id, name: clean_label(&column.name), ty });
        }
        let mut cells = imbl::OrdMap::new();
        for (row, row_cells) in &grid {
            for (column, cell) in stored.iter().zip(row_cells) {
                if let Some(cell) = stored_cell(cell, column)? {
                    cells.insert(key(row, &column.id), cell);
                }
            }
        }
        for (cell_key, cell) in overlay {
            ensure!(row_set.contains(&cell_key.row), NoRowSnafu { row: cell_key.row.as_str() });
            let column = stored
                .iter()
                .find(|c| c.id == cell_key.column)
                .context(NoColumnSnafu { column: cell_key.column.as_str() })?;
            if is_non_finite(&cell) {
                non_finite += 1;
                if base.is_some() {
                    cells.insert(cell_key, DataCell::Null);
                }
                continue;
            }
            match stored_cell(&cell, column)? {
                Some(cell) => {
                    cells.insert(cell_key, cell);
                }
                None if base.is_some() => {
                    cells.insert(cell_key, DataCell::Null);
                }
                None => {}
            }
        }
        let rows = Rows::from_unique(row_ids);
        Ok((Table { generation, epoch: String::new(), columns: stored, rows, base, cells }, non_finite))
    }
}

/// A given id, valid.
fn checked_id(id: Option<String>) -> Result<String, DataError> {
    let id = id.context(IdMissingSnafu)?;
    ensure!(valid_id(&id), IdInvalidSnafu { id });
    Ok(id)
}

/// The overlay entry for `cell` in `column` (`None` when empty), in display form in a Text column.
fn stored_cell(cell: &DataCell, column: &Column) -> Result<Option<DataCell>, DataError> {
    let cell = match cell {
        DataCell::Null => return Ok(None),
        DataCell::Number(_) | DataCell::Bool(_) if column.ty == ColumnType::Text => DataCell::Text(cell.as_label()),
        _ => cell.clone(),
    };
    check_cell(&cell, &column.name, column.ty)?;
    Ok(Some(cell))
}

impl From<&Table> for NewTable {
    /// The table's ids, types, base and overlay, without its generation.
    fn from(table: &Table) -> NewTable {
        NewTable {
            generation: None,
            columns: table
                .columns
                .iter()
                .map(|c| NewColumn { id: Some(c.id.clone()), name: c.name.clone(), ty: Some(c.ty) })
                .collect(),
            rows: table.rows.iter().map(|id| NewRow { id: Some(id.to_owned()), cells: Vec::new() }).collect(),
            base: table.base.clone(),
            overlay: table.cells.clone(),
        }
    }
}

#[cfg(test)]
#[path = "new_table_tests.rs"]
mod new_table_tests;
