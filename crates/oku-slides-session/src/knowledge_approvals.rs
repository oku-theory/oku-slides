//! The deck knowledge files the user approved, by content hash, kept in the data folder.
//!
//! `knowledge-approvals.json` is a JSON array of `sha256:<hex>` hashes of whole file texts
//! (`oku_slides_core::asset_id`): a text changed by undo, merge or receive has a new hash and
//! awaits again. Loading is lenient: a missing file approves nothing; an unreadable one is
//! logged and approves nothing. Approving is strict: the file is read again under the set
//! lock, so hashes added to it by hand after the load are kept; a file that cannot be read or
//! parsed is damaged, one that cannot be written is a write error, and neither approves.
//!
//! Approvals are global, for every deck and tab, and keyed by hash only: the same text approved
//! in one deck is approved in all. The file only grows, with no cap or pruning: a hash removed
//! by hand while the app runs comes back with the next approval, until a restart. Two processes
//! sharing the file can each write their union over the other's, losing approvals the other
//! made: that fails closed, the lost texts awaiting approval again.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::approvals_file::{read_set, read_set_strict};
use crate::error::{KnowledgeApprovalsDamagedSnafu, KnowledgeApprovalsWriteSnafu};

/// A set of approved hashes, shared by everything that loads the same file.
type ApprovedSet = Arc<Mutex<BTreeSet<String>>>;

/// The set of each approvals file, process-wide: every tab reads the one set of its file.
static SHARED: Mutex<BTreeMap<PathBuf, ApprovedSet>> = Mutex::new(BTreeMap::new());

/// The approved knowledge texts, shared by every holder of the same file in the process.
#[derive(Debug, Clone)]
pub(crate) struct KnowledgeApprovals {
    set: ApprovedSet,
    /// The file the set is written to; `None`: kept in memory only.
    path: Option<PathBuf>,
    storage: Storage,
}

impl KnowledgeApprovals {
    /// The approvals stored at `path`; none when it is missing or unreadable (logged).
    ///
    /// The file is read the first time the process loads it; later loads share that set.
    /// Without a path the set is private and empty.
    pub(crate) fn load(path: Option<PathBuf>, storage: Storage) -> Self {
        let set = match &path {
            None => ApprovedSet::default(),
            Some(p) => {
                let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
                let key = oku_slides_platform::storage::resolve(p);
                shared
                    .entry(key)
                    .or_insert_with(|| Arc::new(Mutex::new(read_set(p, &storage, "knowledge approvals"))))
                    .clone()
            }
        };
        Self { set, path, storage }
    }

    /// True if the user approved the text whose hash is `hash` (`sha256:<hex>`).
    pub(crate) fn is_approved(&self, hash: &str) -> bool {
        self.set.lock().unwrap_or_else(PoisonError::into_inner).contains(hash)
    }

    /// How many hashes are approved: a generation, since approvals only grow.
    pub(crate) fn count(&self) -> usize {
        self.set.lock().unwrap_or_else(PoisonError::into_inner).len()
    }

    /// Approves `hashes` (`sha256:<hex>`), writing the file only if it lacks one of them.
    ///
    /// The set lock is held throughout, so two approvals never write over each other. The
    /// file is read strictly and its hashes join the memory (also ones added by hand after
    /// the load). The union of file, memory and `hashes` is written (sorted) only if it differs
    /// from what the file already holds, and only then does the memory take it: a failed write
    /// approves nothing. When the file already holds the whole union nothing is written and the
    /// memory just absorbs it. Without a path the approvals stay in memory.
    ///
    /// # Errors
    ///
    /// `KnowledgeApprovalsDamaged` when the file exists but cannot be read or parsed;
    /// `KnowledgeApprovalsWrite` when it cannot be written.
    pub(crate) fn approve(&self, hashes: &[String]) -> Result<()> {
        let mut set = self.set.lock().unwrap_or_else(PoisonError::into_inner);
        let mut union = set.clone();
        let mut stored = None;
        if let Some(path) = &self.path {
            let read = read_set_strict::<String>(path, &self.storage).map_err(|e| {
                tracing::warn!(path = %path.display(), error = %snafu::Report::from_error(&e), "knowledge approvals not saved: file damaged");
                KnowledgeApprovalsDamagedSnafu.build()
            })?;
            union.extend(read.iter().cloned());
            stored = Some((path, read));
        }
        union.extend(hashes.iter().cloned());
        if let Some((path, read)) = stored
            && read != union
        {
            self.write(path, &union)?;
        }
        *set = union;
        Ok(())
    }

    /// Writes `set` as a sorted JSON array of hashes.
    fn write(&self, path: &Path, set: &BTreeSet<String>) -> Result<()> {
        let list: Vec<&String> = set.iter().collect();
        let json = serde_json::to_vec(&list).map_err(|e| {
            tracing::warn!(path = %path.display(), error = %e, "knowledge approvals not saved: encoding failed");
            KnowledgeApprovalsWriteSnafu.build()
        })?;
        self.storage.write_atomic(path, json).map_err(|e| {
            tracing::warn!(path = %path.display(), error = %e, "knowledge approvals not saved: write failed");
            KnowledgeApprovalsWriteSnafu.build()
        })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "knowledge_approvals_tests.rs"]
mod knowledge_approvals_tests;
