//! The versions whose agent guides the user approved, kept in the data folder.
//!
//! One set for packs and themes, so a version held only by a deck can be
//! approved too. A failed read is logged and approves nothing; a failed write
//! is an error of the approving request.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use uuid::Uuid;

use oku_slides_core::PackVersion;
use oku_slides_platform::storage::{Storage, StorageErrorKind};

use crate::Result;
use crate::error::{GuideApprovalsWriteSnafu, ResponseSnafu};

/// One version of a pack or theme, by id and digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VersionRef {
    /// Id of the pack or theme.
    pub id: Uuid,
    /// The version.
    pub version: PackVersion,
}

/// A set of approved versions, shared by everything that loads the same file.
type ApprovedSet = Arc<Mutex<BTreeSet<(Uuid, PackVersion)>>>;

/// The set of each approvals file, process-wide: tabs approve into one set, never over each other.
static SHARED: Mutex<BTreeMap<PathBuf, ApprovedSet>> = Mutex::new(BTreeMap::new());

/// The approved versions, shared by every holder of the same file in the process.
#[derive(Debug, Clone)]
pub struct GuideApprovals {
    set: ApprovedSet,
    /// The file the set is written to; `None`: kept in memory only.
    path: Option<PathBuf>,
    storage: Storage,
}

impl GuideApprovals {
    /// The approvals stored at `path`; none when it is missing or unreadable (logged).
    ///
    /// The file is read the first time the process loads it; later loads share that set.
    /// Without a path the set is private and kept in memory.
    pub(crate) fn load(path: Option<PathBuf>, storage: Storage) -> Self {
        let set = match &path {
            None => ApprovedSet::default(),
            Some(p) => {
                let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
                let key = oku_slides_platform::storage::resolve(p);
                shared.entry(key).or_insert_with(|| Arc::new(Mutex::new(Self::read(p, &storage)))).clone()
            }
        };
        Self { set, path, storage }
    }

    /// The pairs stored in `path`: a missing file is nothing approved, silently.
    fn read(path: &std::path::Path, storage: &Storage) -> BTreeSet<(Uuid, PackVersion)> {
        let bytes = match storage.read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == StorageErrorKind::NotFound => return BTreeSet::new(),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "guide approvals unreadable: none approved");
                return BTreeSet::new();
            }
        };
        match serde_json::from_slice::<Vec<VersionRef>>(&bytes) {
            Ok(list) => list.into_iter().map(|r| (r.id, r.version)).collect(),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "guide approvals unreadable: none approved");
                BTreeSet::new()
            }
        }
    }

    /// True if the user approved version `v` of `id`.
    pub(crate) fn is_approved(&self, id: Uuid, v: &PackVersion) -> bool {
        self.set.lock().unwrap_or_else(PoisonError::into_inner).contains(&(id, v.clone()))
    }

    /// How many versions are approved: a generation, since approvals only grow.
    pub(crate) fn count(&self) -> usize {
        self.set.lock().unwrap_or_else(PoisonError::into_inner).len()
    }

    /// Approves `versions`, then writes the whole set once.
    ///
    /// Without a path the approvals stay in memory. On a failed write nothing is approved.
    ///
    /// # Errors
    ///
    /// `GuideApprovalsWrite` when the file cannot be written.
    pub(crate) fn approve(&self, versions: &[VersionRef]) -> Result<()> {
        let mut set = self.set.lock().unwrap_or_else(PoisonError::into_inner);
        let added: Vec<(Uuid, PackVersion)> =
            versions.iter().map(|r| (r.id, r.version.clone())).filter(|pair| set.insert(pair.clone())).collect();
        let written = self.write(&set);
        if written.is_err() {
            for pair in &added {
                set.remove(pair);
            }
        }
        written
    }

    /// Writes `set` as a sorted JSON array of `{id, version}`.
    fn write(&self, set: &BTreeSet<(Uuid, PackVersion)>) -> Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        let list: Vec<VersionRef> =
            set.iter().map(|(id, version)| VersionRef { id: *id, version: version.clone() }).collect();
        let json = serde_json::to_vec(&list).context(ResponseSnafu)?;
        self.storage.write_atomic(path, json).context(GuideApprovalsWriteSnafu { path })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "guide_approvals_tests.rs"]
mod guide_approvals_tests;
