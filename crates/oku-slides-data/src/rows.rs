//! A table's row ids, in display order, with an id lookup in O(log² n).
//!
//! Each row carries a hidden order label that grows along the rows. The position of an id is
//! a binary search on these labels, and its membership a hash lookup. Both structures are
//! `imbl`, so a clone is O(1) and shares them.

use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

use crate::{IdRepair, bounded, limits, valid_id};

/// Row ids, in display order.
#[derive(Clone, Default)]
pub struct Rows {
    /// Order label and id of each row; labels strictly grow along the rows.
    ordered: imbl::Vector<(u128, String)>,
    /// Order label of each id.
    labels: imbl::HashMap<String, u128>,
}

impl Rows {
    /// Rows `ids`, in this order; the caller has checked they are unique.
    pub(crate) fn from_unique(ids: impl IntoIterator<Item = String>) -> Rows {
        let mut rows = Rows { ordered: ids.into_iter().map(|id| (0, id)).collect(), labels: imbl::HashMap::new() };
        rows.relabel();
        rows
    }

    /// Rows `ids`, in this order, duplicate or invalid ids repaired by [`IdRepair`].
    #[must_use]
    pub fn repaired(ids: impl IntoIterator<Item = String>) -> Rows {
        let mut repair = IdRepair::new();
        Rows::from_unique(ids.into_iter().map(|id| repair.repair_id(&id)))
    }

    /// Number of rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    /// Whether there is no row.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    /// The id of the row at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        self.ordered.get(index).map(|(_, id)| id.as_str())
    }

    /// The ids, in display order.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.ordered.iter().map(|(_, id)| id.as_str())
    }

    /// Whether `id` is a row.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.labels.contains_key(id)
    }

    /// The position of the row `id`.
    #[must_use]
    pub fn index_of(&self, id: &str) -> Option<usize> {
        let label = *self.labels.get(id)?;
        self.ordered.binary_search_by(|(other, _)| other.cmp(&label)).ok()
    }

    /// Whether both share their storage (a clone not edited since).
    #[must_use]
    pub fn ptr_eq(&self, other: &Rows) -> bool {
        self.ordered.ptr_eq(&other.ordered)
    }

    /// Inserts the new row `id` at `index` (at most `len`).
    pub(crate) fn insert(&mut self, index: usize, id: String) {
        let low = index.checked_sub(1).and_then(|i| self.ordered.get(i)).map_or(0, |(label, _)| *label);
        let high = self.ordered.get(index).map_or(u128::MAX, |(label, _)| *label);
        if high - low >= 2 {
            let label = low + (high - low) / 2;
            self.labels.insert(id.clone(), label);
            self.ordered.insert(index, (label, id));
        } else {
            self.ordered.insert(index, (0, id));
            self.relabel();
        }
    }

    /// Removes the row at `index` and returns its id.
    pub(crate) fn remove(&mut self, index: usize) -> String {
        let (_, id) = self.ordered.remove(index);
        self.labels.remove(&id);
        id
    }

    /// Spreads the labels evenly over `u128`, leaving room on both ends.
    fn relabel(&mut self) {
        let step = u128::MAX / (self.ordered.len() as u128 + 2);
        let ordered: imbl::Vector<(u128, String)> =
            self.ordered.iter().zip(1_u128..).map(|((_, id), i)| (step * i, id.clone())).collect();
        self.labels = ordered.iter().map(|(label, id)| (id.clone(), *label)).collect();
        self.ordered = ordered;
    }
}

/// Equal when the ids are, in the same order; labels are an index detail.
impl PartialEq for Rows {
    fn eq(&self, other: &Rows) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl fmt::Debug for Rows {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Serialize for Rows {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

/// At most `MAX_ROWS` valid ids, none repeated.
impl<'de> Deserialize<'de> for Rows {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Rows, D::Error> {
        let ids: Vec<String> = bounded::vec::<_, _, { limits::MAX_ROWS }>(deserializer)?;
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        for id in &ids {
            if !valid_id(id) {
                return Err(de::Error::custom("invalid row id"));
            }
            if !seen.insert(id.as_str()) {
                return Err(de::Error::custom(format_args!("row id {id} repeated")));
            }
        }
        Ok(Rows::from_unique(ids))
    }
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod rows_tests;
