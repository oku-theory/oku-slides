//! The read side of an approvals file in the data folder: a JSON array.
//!
//! Shared by the guide approvals and the knowledge approvals. [`read_set`] is lenient (a
//! missing file approves nothing, silently; an unreadable one approves nothing and is
//! logged); [`read_set_strict`] is for a read that precedes a write, which must not
//! overwrite what it could not read.

use std::collections::BTreeSet;
use std::path::Path;

use serde::de::DeserializeOwned;
use snafu::{Backtrace, ResultExt, Snafu};

use oku_slides_platform::storage::{Storage, StorageError, StorageErrorKind};

/// An approvals file that exists but cannot be used.
#[derive(Debug, Snafu)]
pub(crate) enum ReadError {
    /// The file cannot be read.
    #[snafu(display("cannot read the approvals file"))]
    Unreadable { source: StorageError, backtrace: Backtrace },
    /// The file is not a JSON array of the expected entries.
    #[snafu(display("the approvals file is not a JSON array of entries"))]
    Malformed { source: serde_json::Error, backtrace: Backtrace },
}

/// The entries stored in `path`, a JSON array; `what` names the file in the log.
pub(crate) fn read_set<K: DeserializeOwned + Ord>(path: &Path, storage: &Storage, what: &str) -> BTreeSet<K> {
    read_set_strict(path, storage).unwrap_or_else(|e| {
        tracing::warn!(path = %path.display(), error = %snafu::Report::from_error(&e), "{what} unreadable: none approved");
        BTreeSet::new()
    })
}

/// The entries stored in `path`: a missing file is the empty set.
///
/// # Errors
///
/// [`ReadError`] when the file exists but cannot be read or is not a JSON array of entries.
pub(crate) fn read_set_strict<K: DeserializeOwned + Ord>(
    path: &Path,
    storage: &Storage,
) -> Result<BTreeSet<K>, ReadError> {
    let bytes = match storage.read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == StorageErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(e) => return Err(e).context(UnreadableSnafu),
    };
    let list = serde_json::from_slice::<Vec<K>>(&bytes).context(MalformedSnafu)?;
    Ok(list.into_iter().collect())
}
