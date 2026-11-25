//! The deck knowledge files the user approved, by content hash, kept in the data folder.
//!
//! `knowledge-approvals.json` is a JSON array of `sha256:<hex>` hashes of whole file texts
//! (`oku_slides_core::asset_id`): a text changed by undo, merge or receive has a new hash and
//! awaits again. Lot 2 only reads the file (lot 3 approves). A missing file approves nothing;
//! an unreadable one is logged and approves nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_platform::storage::Storage;

use crate::approvals_file::read_set;

/// A set of approved hashes, shared by everything that loads the same file.
type ApprovedSet = Arc<Mutex<BTreeSet<String>>>;

/// The set of each approvals file, process-wide: every tab reads the one set of its file.
static SHARED: Mutex<BTreeMap<PathBuf, ApprovedSet>> = Mutex::new(BTreeMap::new());

/// The approved knowledge texts, shared by every holder of the same file in the process.
#[derive(Debug, Clone)]
pub(crate) struct KnowledgeApprovals {
    set: ApprovedSet,
}

impl KnowledgeApprovals {
    /// The approvals stored at `path`; none when it is missing or unreadable (logged).
    ///
    /// The file is read the first time the process loads it; later loads share that set.
    /// Without a path the set is private and empty.
    pub(crate) fn load(path: Option<PathBuf>, storage: &Storage) -> Self {
        let set = match path {
            None => ApprovedSet::default(),
            Some(p) => {
                let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
                let key = oku_slides_platform::storage::resolve(&p);
                shared
                    .entry(key)
                    .or_insert_with(|| Arc::new(Mutex::new(read_set(&p, storage, "knowledge approvals"))))
                    .clone()
            }
        };
        Self { set }
    }

    /// True if the user approved the text whose hash is `hash` (`sha256:<hex>`).
    pub(crate) fn is_approved(&self, hash: &str) -> bool {
        self.set.lock().unwrap_or_else(PoisonError::into_inner).contains(hash)
    }
}

#[cfg(test)]
#[path = "knowledge_approvals_tests.rs"]
mod knowledge_approvals_tests;
