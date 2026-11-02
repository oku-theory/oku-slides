//! A workspace's own folder under a parent other processes may share.

use std::path::{Path, PathBuf};

use oku_slides_platform::storage::Storage;
use snafu::ResultExt;

use crate::error::{Result, WriteSnafu};

/// A folder only this workspace writes, removed when it is dropped.
///
/// Named at random (a v4 UUID) under its parent: two dev servers on one data
/// folder never write to, nor delete, each other's tabs (t-1413). A folder a
/// crashed process left behind stays; no process can tell it from a live one's.
#[derive(Debug)]
pub(crate) struct OwnDir {
    storage: Storage,
    path: PathBuf,
}

impl OwnDir {
    /// Creates a new random folder under `parent`, through storage.
    ///
    /// # Errors
    ///
    /// `File` if the folder cannot be created.
    pub(crate) fn create(storage: &Storage, parent: &Path) -> Result<Self> {
        let path = parent.join(uuid::Uuid::new_v4().simple().to_string());
        storage.create_dir_all(&path).context(WriteSnafu { path: &path })?;
        Ok(Self { storage: storage.clone(), path })
    }

    /// The folder's path.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for OwnDir {
    fn drop(&mut self) {
        // On a thread of its own: storage blocks on its runtime, never allowed inside an async context.
        let (storage, path) = (self.storage.clone(), self.path.clone());
        let removed = std::thread::spawn(move || storage.delete_dir(&path)).join();
        let path = &self.path;
        match removed {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!(
                path = %path.display(),
                error = %oku_slides_platform::errors::one_line(&e),
                "workspace folder not removed"
            ),
            Err(_) => tracing::warn!(path = %path.display(), "workspace folder removal panicked"),
        }
    }
}
