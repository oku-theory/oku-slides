//! The stored table of a data: columns, row ids, an optional base and the overlay of cells.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ensure};

use crate::error::{
    ColumnTypeSnafu, ColumnsInvalidSnafu, DataError, IdInvalidSnafu, NeedsBaseSnafu, NoColumnSnafu, NoRowSnafu,
};
use crate::text::cut_chars;
use crate::{BaseRef, ColumnType, DataCell, Dataset, IdRepair, Rows, bounded, limits, valid_id};

/// Row ids a `data-column-type` refusal names; the others are counted.
pub(crate) const NAMED_ROWS: usize = 5;

/// A data's table, as stored.
///
/// A cell's value is its overlay entry if any, else the base's value, else empty. The overlay
/// stores an empty cell only over a base (it hides the base value); without a base an empty
/// cell has no entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(from = "TableRepr")]
pub struct Table {
    /// Id of this generation of the table; a `Replace` starts a new one.
    pub generation: String,
    /// Id of the current layer (base and overlay); each compaction starts a new one. Empty for
    /// the generation's first layer.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub epoch: String,
    /// Columns, in display order.
    pub columns: Vec<Column>,
    /// Row ids, in display order.
    #[schemars(with = "Vec<String>")]
    pub rows: Rows,
    /// The base file holding the cells not in the overlay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<BaseRef>,
    /// The overlay: cells edited since the base, keyed `<row id>/<column id>`.
    #[serde(default)]
    #[schemars(with = "BTreeMap<String, DataCell>")]
    pub cells: imbl::OrdMap<CellKey, DataCell>,
}

/// How a table parses: lengths bounded while reading, then made canonical ([`Table::canonicalize`]).
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TableRepr {
    generation: String,
    #[serde(default)]
    epoch: String,
    #[serde(deserialize_with = "bounded::vec::<_, _, { limits::MAX_COLUMNS }>")]
    columns: Vec<Column>,
    #[schemars(with = "Vec<String>")]
    rows: Rows,
    #[serde(default)]
    base: Option<BaseRef>,
    #[serde(default, deserialize_with = "bounded::map::<_, _, _, { limits::MAX_OVERLAY }>")]
    #[schemars(with = "BTreeMap<String, DataCell>")]
    cells: imbl::OrdMap<CellKey, DataCell>,
}

impl From<TableRepr> for Table {
    fn from(repr: TableRepr) -> Table {
        let TableRepr { generation, epoch, columns, rows, base, cells } = repr;
        let mut table = Table { generation, epoch, columns, rows, base, cells };
        table.canonicalize();
        table
    }
}

/// The key of an overlay cell, written `<row id>/<column id>` (ids never contain `/`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CellKey {
    /// Row id.
    pub row: String,
    /// Column id.
    pub column: String,
}

impl TryFrom<String> for CellKey {
    type Error = DataError;

    fn try_from(key: String) -> Result<Self, DataError> {
        match key.split_once('/') {
            Some((row, column)) if valid_id(row) && valid_id(column) => {
                Ok(CellKey { row: row.to_owned(), column: column.to_owned() })
            }
            _ => Err(IdInvalidSnafu { id: key }.build().into()),
        }
    }
}

impl From<CellKey> for String {
    fn from(key: CellKey) -> String {
        format!("{}/{}", key.row, key.column)
    }
}

/// A column of a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Column {
    /// Stable id, what views and the overlay reference.
    pub id: String,
    /// Displayed name.
    pub name: String,
    /// What its cells hold.
    #[serde(rename = "type")]
    pub ty: ColumnType,
}

/// Where a cell's value comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellRead<'a> {
    /// An overlay entry.
    Overlay(&'a DataCell),
    /// An empty overlay entry hiding the base value.
    Cleared,
    /// No overlay entry: the base holds the value.
    Base,
    /// Empty: no overlay entry and no base.
    Null,
}

impl Table {
    /// Where the cell at `row`, `column` comes from.
    #[must_use]
    pub fn cell(&self, row: &str, column: &str) -> CellRead<'_> {
        match self.cells.get(&key(row, column)) {
            Some(DataCell::Null) => CellRead::Cleared,
            Some(cell) => CellRead::Overlay(cell),
            None if self.base.is_some() => CellRead::Base,
            None => CellRead::Null,
        }
    }

    /// Entries in the overlay.
    #[must_use]
    pub fn overlay_len(&self) -> usize {
        self.cells.len()
    }

    /// UTF-8 bytes of the overlay's text cells and of the column names.
    #[must_use]
    pub fn text_bytes(&self) -> usize {
        let cells: usize = self.cells.values().map(|cell| if let DataCell::Text(s) = cell { s.len() } else { 0 }).sum();
        cells + self.columns.iter().map(|c| c.name.len()).sum::<usize>()
    }

    /// Makes column ids unique ([`IdRepair`]) and drops overlay entries of unknown rows or
    /// columns, and empty ones without a base. Row ids are unique by construction.
    pub fn canonicalize(&mut self) {
        let mut repair = IdRepair::new();
        for column in &mut self.columns {
            column.id = repair.repair_id(&column.id);
        }
        let columns: BTreeSet<&str> = self.columns.iter().map(|c| c.id.as_str()).collect();
        let has_base = self.base.is_some();
        let orphan = |key: &CellKey, cell: &DataCell| {
            !self.rows.contains(&key.row)
                || !columns.contains(key.column.as_str())
                || (!has_base && matches!(cell, DataCell::Null))
        };
        let dropped: Vec<CellKey> = self.cells.iter().filter(|(k, c)| orphan(k, c)).map(|(k, _)| k.clone()).collect();
        for key in dropped {
            self.cells.remove(&key);
        }
    }

    /// A dataset of `columns` (all if `None`, else these ids in this order) and of the data rows
    /// in `rows` (row 0 is the first data row), text cut to `max_cell_chars` characters.
    ///
    /// Its `ids[0]` is `""` (the header row), `ids[i]` the id of `source[i]`'s row.
    ///
    /// # Errors
    ///
    /// `NeedsBase` with a base, `ColumnsInvalid` for an empty or repeating projection,
    /// `NoColumn` for an unknown column.
    pub fn to_dataset(
        &self,
        columns: Option<&[String]>,
        rows: Range<usize>,
        max_cell_chars: usize,
    ) -> Result<Dataset, DataError> {
        ensure!(self.base.is_none(), NeedsBaseSnafu);
        let picked = match columns {
            None => self.columns.iter().collect(),
            Some(ids) => self.projection(ids)?,
        };
        let end = rows.end.min(self.rows.len());
        let start = rows.start.min(end);
        let mut source = vec![picked.iter().map(|c| DataCell::Text(c.name.clone())).collect()];
        let mut ids = vec![String::new()];
        for row in (start..end).filter_map(|i| self.rows.get(i)) {
            source.push(picked.iter().map(|c| self.read(row, &c.id, max_cell_chars)).collect());
            ids.push(row.to_owned());
        }
        Ok(Dataset { source, ids, columns: picked.iter().map(|c| c.id.clone()).collect() })
    }

    /// Changes `column`'s type, converting each of its overlay cells without loss.
    ///
    /// With a base, its cells are checked by the worker first (`engine::check_column_type`); a
    /// base cell that still does not convert reads as text and is reported.
    ///
    /// # Errors
    ///
    /// `NoColumn`, or `ColumnType` naming the first rows whose cell does not convert.
    pub fn set_column_type(&mut self, column: &str, ty: ColumnType) -> Result<(), DataError> {
        let index = self.column_index(column)?;
        let from = self.columns[index].ty;
        if from == ty {
            return Ok(());
        }
        let mut converted = Vec::new();
        let mut refused = Vec::new();
        let mut more = 0_usize;
        for row in self.rows.iter() {
            let key = key(row, column);
            let Some(cell) = self.cells.get(&key) else { continue };
            match ty.convert(from, cell) {
                Some(cell) => converted.push((key, cell)),
                None if refused.len() < NAMED_ROWS => refused.push(row.to_owned()),
                None => more += 1,
            }
        }
        ensure!(
            refused.is_empty(),
            ColumnTypeSnafu { column: self.columns[index].name.as_str(), ty, rows: refused, more }
        );
        for (key, cell) in converted {
            self.cells.insert(key, cell);
        }
        self.columns[index].ty = ty;
        Ok(())
    }

    /// Index of the column `id`.
    pub(crate) fn column_index(&self, id: &str) -> Result<usize, DataError> {
        Ok(self.columns.iter().position(|c| c.id == id).context(NoColumnSnafu { column: id })?)
    }

    /// Index of the row `id`.
    pub(crate) fn row_index(&self, id: &str) -> Result<usize, DataError> {
        Ok(self.rows.index_of(id).context(NoRowSnafu { row: id })?)
    }

    /// The columns `ids`, in this order.
    pub(crate) fn projection(&self, ids: &[String]) -> Result<Vec<&Column>, DataError> {
        let distinct: BTreeSet<&String> = ids.iter().collect();
        ensure!(!ids.is_empty() && distinct.len() == ids.len(), ColumnsInvalidSnafu);
        ids.iter().map(|id| self.column_index(id).map(|i| &self.columns[i])).collect()
    }

    /// The overlay value at `row`, `column` (empty if none), text cut to `max_chars`.
    fn read(&self, row: &str, column: &str, max_chars: usize) -> DataCell {
        match self.cells.get(&key(row, column)) {
            Some(DataCell::Text(s)) => DataCell::Text(cut_chars(s, max_chars)),
            Some(cell) => cell.clone(),
            None => DataCell::Null,
        }
    }
}

/// The overlay key of `row`, `column`.
pub(crate) fn key(row: &str, column: &str) -> CellKey {
    CellKey { row: row.to_owned(), column: column.to_owned() }
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
