//! The read side of an approvals file in the data folder: a JSON array, read once.
//!
//! Shared by the guide approvals and the knowledge approvals: a missing file approves nothing,
//! silently; an unreadable one approves nothing and is logged.

use std::collections::BTreeSet;
use std::path::Path;

use serde::de::DeserializeOwned;

use oku_slides_platform::storage::{Storage, StorageErrorKind};

/// The entries stored in `path`, a JSON array; `what` names the file in the log.
pub(crate) fn read_set<K: DeserializeOwned + Ord>(path: &Path, storage: &Storage, what: &str) -> BTreeSet<K> {
    let bytes = match storage.read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == StorageErrorKind::NotFound => return BTreeSet::new(),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "{what} unreadable: none approved");
            return BTreeSet::new();
        }
    };
    match serde_json::from_slice::<Vec<K>>(&bytes) {
        Ok(list) => list.into_iter().collect(),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "{what} unreadable: none approved");
            BTreeSet::new()
        }
    }
}
