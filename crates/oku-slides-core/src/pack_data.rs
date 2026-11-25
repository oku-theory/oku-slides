//! The data a pack carries for its examples (`PackManifest::data`), and their bounds.
//!
//! An example names a pack data as a view names a deck data; adding the example copies the data
//! into the deck. A pack's data hold their own cells: no `base`, few data, few cells.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_data::limits::MAX_BATCH_CELLS;
use oku_slides_data::{Data, DataId};
use snafu::ensure;

use crate::error::{ManifestDataBaseSnafu, ManifestDataCellsSnafu, ManifestDataCountSnafu, PackManifestError};

/// Data a pack may carry.
pub const MAX_PACK_DATA: usize = 16;

/// Checks the bounds of a pack's `data`.
///
/// # Errors
///
/// More than [`MAX_PACK_DATA`] data, a data with a `base`, or more than [`MAX_BATCH_CELLS`]
/// cells in all: the data must fit one batch that copies them into a deck.
pub fn check_pack_data(data: &BTreeMap<DataId, Arc<Data>>) -> Result<(), PackManifestError> {
    ensure!(data.len() <= MAX_PACK_DATA, ManifestDataCountSnafu { count: data.len(), max: MAX_PACK_DATA });
    if let Some((id, _)) = data.iter().find(|(_, d)| d.table.base.is_some()) {
        return ManifestDataBaseSnafu { data: id.as_str() }.fail().map_err(Into::into);
    }
    let count = data.values().map(|d| d.table.overlay_len()).sum::<usize>();
    ensure!(count <= MAX_BATCH_CELLS, ManifestDataCellsSnafu { count, max: MAX_BATCH_CELLS });
    Ok(())
}

#[cfg(test)]
#[path = "pack_data_tests.rs"]
mod pack_data_tests;
