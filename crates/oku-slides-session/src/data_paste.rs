//! A block pasted into a grid, the `host-data` pane's or the `data-sheet`'s, as id-based edits of
//! the data: `set_cell` on the rows and columns it covers, `insert_column` and `insert_row` past
//! the grid's last column and the data's last row. The pane's grid shows a page of the rows,
//! the sheet's another: a block taller than the page writes into the data's next rows.

use std::collections::BTreeMap;

use oku_slides_core::{Op, OpError, Origin, Target};
use oku_slides_data::{DataCell, DataError, DataId, TableOp};
use snafu::{IntoError, ResultExt};

use crate::data_edit::{Follow, data_views, follow_inserts, follow_op};
use crate::data_ops::prepared;
use crate::data_panel::{DataTarget, data_missing, manual_table};
use crate::error::PrepareSnafu;
use crate::{Result, Session};

/// A block pasted at a cell of the grid.
pub(crate) struct Paste {
    /// Row of the top-left cell.
    pub(crate) row: String,
    /// Column of the top-left cell.
    pub(crate) column: String,
    /// The pasted cells, row by row, parsed as the grid parses typed input.
    pub(crate) cells: Vec<Vec<DataCell>>,
    /// Names of the columns the block adds past the view's last one, in order.
    pub(crate) names: Vec<String>,
}

impl Session {
    /// The changes with the row and column ids they leave out drawn by `prepare_ops`, as the
    /// apply of `data` would draw them.
    ///
    /// # Errors
    ///
    /// `Prepare` when the core refuses a change, or draws no id for an insert (a missing data).
    pub(crate) fn draw_ids(&self, data: &DataId, changes: Vec<TableOp>) -> Result<Vec<TableOp>> {
        let ops = changes.into_iter().map(|change| Op::EditData { data: data.clone(), change }).collect();
        let deck = self.doc.sources.deck();
        let drawn = prepared(deck, &self.registry, &self.chart_labels, ops, Origin::External)?.ops;
        drawn
            .into_iter()
            .map(|op| match op {
                Op::EditData { change, .. } if has_id(&change) => Ok(change),
                _ => Err(refused(DataError::id_missing())),
            })
            .collect()
    }

    /// The ops writing `paste` into the data of view `target`, with the view's projection and
    /// series following the columns it adds; one undo step for the caller.
    ///
    /// # Errors
    ///
    /// `Prepare` naming the anchor row or column when the data or the view no longer holds it.
    pub(crate) fn paste_ops(&self, owner: &Target, target: &DataTarget, paste: Paste) -> Result<Vec<Op>> {
        let id = DataId::try_from(target.data()?.to_owned()).map_err(OpError::from).context(PrepareSnafu)?;
        let read = self.view_columns(target)?;
        let (mut ops, added) = self.paste_changes(&id, &read, paste)?;
        let all: Vec<String> = read.iter().chain(&added).cloned().collect();
        ops.extend(follow_op(owner, target, &all, &Follow::Inserted((read.len()..all.len()).collect()))?);
        Ok(ops)
    }

    /// The ops writing `paste` into `data` from the data sheet, over all its columns, with
    /// every view of the data following the columns it adds; one undo step for the caller.
    ///
    /// # Errors
    ///
    /// `Prepare` naming the anchor row or column when the data no longer holds it.
    pub(crate) fn sheet_paste_ops(&self, data: &DataId, paste: Paste) -> Result<Vec<Op>> {
        let read: Vec<String> = {
            let deck = self.doc.sources.deck();
            let stored = deck.data.get(data.as_str()).ok_or_else(|| data_missing(data.as_str()))?;
            manual_table(data.as_str(), stored)?.columns.iter().map(|c| c.id.clone()).collect()
        };
        let (mut ops, added) = self.paste_changes(data, &read, paste)?;
        if !added.is_empty() {
            for view in data_views(self.doc.sources.deck(), &self.registry, data)? {
                ops.extend(follow_inserts(&view, read.last().map(String::as_str), &added)?);
            }
        }
        Ok(ops)
    }

    /// The edits writing `paste` into `id` over the columns `read`, and the ids of the columns
    /// it adds after the last of them.
    fn paste_changes(&self, id: &DataId, read: &[String], paste: Paste) -> Result<(Vec<Op>, Vec<String>)> {
        let rows: Vec<String> = {
            let deck = self.doc.sources.deck();
            let stored = deck.data.get(id.as_str()).ok_or_else(|| data_missing(id.as_str()))?;
            manual_table(id.as_str(), stored)?.rows.iter().map(str::to_owned).collect()
        };
        let top = rows.iter().position(|r| *r == paste.row).ok_or_else(|| refused(DataError::no_row(&paste.row)))?;
        let left =
            read.iter().position(|c| *c == paste.column).ok_or_else(|| refused(DataError::no_column(&paste.column)))?;
        let width = paste.cells.iter().map(Vec::len).max().unwrap_or(0);
        let new_columns = (left + width).saturating_sub(read.len());
        let new_rows = (top + paste.cells.len()).saturating_sub(rows.len());
        // Ids first: the inserts are chained, each after the one before.
        // An unnamed column is a valid header: the grid names the ones it adds.
        let names: Vec<String> = (0..new_columns).map(|k| paste.names.get(k).cloned().unwrap_or_default()).collect();
        let mut inserts: Vec<TableOp> = names
            .iter()
            .map(|name| TableOp::InsertColumn { id: None, after: None, name: name.clone(), cells: BTreeMap::new() })
            .collect();
        inserts.extend((0..new_rows).map(|_| TableOp::InsertRow { id: None, after: None, cells: BTreeMap::new() }));
        let drawn: Vec<String> = self.draw_ids(id, inserts)?.iter().filter_map(inserted_id).collect();
        let (column_ids, row_ids) = drawn.split_at(new_columns.min(drawn.len()));
        let all_columns: Vec<String> = read.iter().chain(column_ids).cloned().collect();
        let all_rows: Vec<String> = rows.iter().chain(row_ids).cloned().collect();
        let mut column_cells: Vec<BTreeMap<String, DataCell>> = vec![BTreeMap::new(); new_columns];
        let mut row_cells: Vec<BTreeMap<String, DataCell>> = vec![BTreeMap::new(); new_rows];
        let mut sets = Vec::new();
        for (dr, line) in paste.cells.into_iter().enumerate() {
            for (dc, value) in line.into_iter().enumerate() {
                let (r, c) = (top + dr, left + dc);
                let (Some(row), Some(column)) = (all_rows.get(r), all_columns.get(c)) else { continue };
                if r >= rows.len() {
                    if value != DataCell::Null {
                        row_cells[r - rows.len()].insert(column.clone(), value);
                    }
                } else if c >= read.len() {
                    if value != DataCell::Null {
                        column_cells[c - read.len()].insert(row.clone(), value);
                    }
                } else {
                    sets.push(TableOp::SetCell { row: row.clone(), column: column.clone(), value });
                }
            }
        }
        let mut changes = Vec::new();
        let mut after = read.last().cloned();
        for ((column, cells), name) in column_ids.iter().zip(column_cells).zip(names) {
            changes.push(TableOp::InsertColumn { id: Some(column.clone()), after: after.clone(), name, cells });
            after = Some(column.clone());
        }
        let mut after = rows.last().cloned();
        for (row, cells) in row_ids.iter().zip(row_cells) {
            changes.push(TableOp::InsertRow { id: Some(row.clone()), after: after.clone(), cells });
            after = Some(row.clone());
        }
        changes.extend(sets);
        let ops = changes.into_iter().map(|change| Op::EditData { data: id.clone(), change }).collect();
        Ok((ops, column_ids.to_vec()))
    }
}

/// `e` as a refused op.
fn refused(e: DataError) -> crate::SessionError {
    PrepareSnafu.into_error(OpError::from(e)).into()
}

/// Whether an insert names its new row or column; any other change has nothing to draw.
fn has_id(change: &TableOp) -> bool {
    match change {
        TableOp::InsertRow { id, .. } | TableOp::InsertColumn { id, .. } => id.is_some(),
        _ => true,
    }
}

fn inserted_id(change: &TableOp) -> Option<String> {
    match change {
        TableOp::InsertRow { id, .. } | TableOp::InsertColumn { id, .. } => id.clone(),
        _ => None,
    }
}
