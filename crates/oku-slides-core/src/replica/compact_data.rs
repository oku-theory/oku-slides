//! Commits a compaction: a data's overlay folded into a new base, in a new `layer` epoch.

use std::sync::Arc;

use oku_slides_data::{BaseRef, DataId, Table};

use super::{DeckReplica, KnowledgeSync};
use crate::error::ReplicaError;

impl DeckReplica {
    /// Replaces data `id`'s layer with epoch `epoch` over `base`, its overlay emptied, without
    /// an undo step. `from` is the table the base was written from.
    ///
    /// Returns `false`, writing nothing, when the data is gone or its table is no longer
    /// `from` (an edit landed while the base was written): the base would miss it. A
    /// truncated data compacts too: its read prefix becomes the data.
    ///
    /// # Errors
    ///
    /// Those of [`Self::apply_silent`].
    pub fn compact_data(
        &mut self,
        id: &DataId,
        from: &Table,
        epoch: String,
        base: BaseRef,
    ) -> Result<bool, ReplicaError> {
        let (mut deck, _) = self.deck();
        let Some(data) = deck.data.get_mut(id) else { return Ok(false) };
        if data.table != *from {
            return Ok(false);
        }
        let table = &mut Arc::make_mut(data).table;
        table.epoch = epoch;
        table.base = Some(base);
        table.cells.clear();
        // A truncated data is never written back; its compaction is the exception, as the
        // new layer holds the prefix read, now within the bounds.
        deck.truncated.remove(id);
        // The deck was read from this replica: its knowledge bundle is the replica's own.
        self.apply_silent(&deck, KnowledgeSync::Unchanged)?;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "compact_data_tests.rs"]
mod compact_data_tests;
