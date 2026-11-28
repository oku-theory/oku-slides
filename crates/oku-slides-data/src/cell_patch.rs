//! The overlay cells a data changed since an older version of it: what the host sends the render
//! worker after a cell edit, in place of the whole table the worker already caches.

use serde::{Deserialize, Serialize};

use crate::{CellKey, Data, DataCell};

/// Overlay cells set and removed between two versions of one data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellPatch {
    /// Cells added or changed (a stored `Null` hides a base value).
    pub set: Vec<(CellKey, DataCell)>,
    /// Cells gone from the overlay.
    pub removed: Vec<CellKey>,
}

impl CellPatch {
    /// The overlay edits that turn `older` into `newer`, or `None` when anything else differs
    /// (name, label, kind, generation, epoch, base, columns or rows), or for a query data, which
    /// has no cells: that one goes whole.
    /// Shared overlay nodes are not visited, so a one-cell edit costs O(log n).
    #[must_use]
    pub fn between(older: &Data, newer: &Data) -> Option<Self> {
        let (a, b) = (older.table()?, newer.table()?);
        let same = older.name == newer.name
            && older.label == newer.label
            && older.inline == newer.inline
            && a.generation == b.generation
            && a.epoch == b.epoch
            && a.base == b.base
            && a.columns == b.columns
            && (a.rows.ptr_eq(&b.rows) || a.rows == b.rows);
        if !same {
            return None;
        }
        let mut patch = Self { set: Vec::new(), removed: Vec::new() };
        for item in a.cells.diff(&b.cells) {
            match item {
                imbl::ordmap::DiffItem::Add(k, v) | imbl::ordmap::DiffItem::Update { new: (k, v), .. } => {
                    patch.set.push((k.clone(), v.clone()));
                }
                imbl::ordmap::DiffItem::Remove(k, _) => patch.removed.push(k.clone()),
            }
        }
        Some(patch)
    }

    /// `data` with this patch's cells set and removed; `None` for a query data, which has no
    /// cells to patch.
    #[must_use]
    pub fn applied(self, data: &Data) -> Option<Data> {
        let mut data = data.clone();
        let table = data.table_mut()?;
        for key in self.removed {
            table.cells.remove(&key);
        }
        for (key, cell) in self.set {
            table.cells.insert(key, cell);
        }
        Some(data)
    }
}

#[cfg(test)]
#[path = "cell_patch_tests.rs"]
mod tests;
