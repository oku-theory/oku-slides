//! Update marks kept on disk: the failed version, its pending notice, "Later".

use std::path::{Path, PathBuf};

use super::{FAILED_NOTICE_FILE, FAILED_VERSION_FILE, LATER_VERSION_FILE, release_version};
use crate::errors::one_line;
use crate::storage::{Storage, StorageError, StorageErrorKind};

/// Longest mark file read: a version, nothing more.
const MAX_MARK: u64 = 64;

/// The update marks under the data folder; blocking calls, outside async contexts only.
///
/// Each file holds one `major.minor.patch` version; anything else reads as absent.
#[derive(Debug, Clone)]
pub struct UpdateMarks {
    storage: Storage,
    failed: PathBuf,
    notice: PathBuf,
    later: PathBuf,
}

impl UpdateMarks {
    /// The marks of the data folder `data`.
    #[must_use]
    pub fn new(storage: Storage, data: &Path) -> Self {
        Self {
            storage,
            failed: data.join(FAILED_VERSION_FILE),
            notice: data.join(FAILED_NOTICE_FILE),
            later: data.join(LATER_VERSION_FILE),
        }
    }

    /// The version whose install failed, if any.
    #[must_use]
    pub fn failed_version(&self) -> Option<String> {
        self.read(&self.failed)
    }

    /// Records a failed install of `version`, and a notice for the next launch.
    ///
    /// # Errors
    ///
    /// If a mark cannot be written.
    pub fn record_failure(&self, version: &str) -> Result<(), StorageError> {
        self.storage.write_atomic(&self.failed, version.as_bytes().to_vec())?;
        self.storage.write_atomic(&self.notice, version.as_bytes().to_vec())
    }

    /// The failed install not yet told; kept until [`clear_notice`](Self::clear_notice).
    ///
    /// An unreadable or malformed notice is removed, so it is not reported at every launch.
    #[must_use]
    pub fn notice(&self) -> Option<String> {
        let version = self.read(&self.notice);
        if version.is_none()
            && let Err(e) = self.remove(&self.notice)
        {
            tracing::warn!(error = %one_line(&e), "notice d'échec de mise à jour non retirée");
        }
        version
    }

    /// Forgets the notice once a window has told it.
    ///
    /// # Errors
    ///
    /// If the mark cannot be removed.
    pub fn clear_notice(&self) -> Result<(), StorageError> {
        self.remove(&self.notice)
    }

    /// The version put off with "Later", if any.
    #[must_use]
    pub fn later(&self) -> Option<String> {
        self.read(&self.later)
    }

    /// Puts `version` off.
    ///
    /// # Errors
    ///
    /// If the mark cannot be written.
    pub fn set_later(&self, version: &str) -> Result<(), StorageError> {
        self.storage.write_atomic(&self.later, version.as_bytes().to_vec())
    }

    /// Forgets the version put off.
    ///
    /// # Errors
    ///
    /// If the mark cannot be removed.
    pub fn clear_later(&self) -> Result<(), StorageError> {
        self.remove(&self.later)
    }

    /// Removes a mark; one already gone is fine.
    fn remove(&self, path: &Path) -> Result<(), StorageError> {
        match self.storage.delete(path) {
            Err(e) if e.kind() != StorageErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// A mark's version; missing, unreadable or malformed reads as absent.
    fn read(&self, path: &Path) -> Option<String> {
        let bytes = match self.storage.read_limited(path, MAX_MARK) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == StorageErrorKind::NotFound => return None,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %one_line(&e), "marque de mise à jour illisible");
                return None;
            }
        };
        let version = String::from_utf8(bytes).ok().map(|v| v.trim().to_owned()).filter(|v| release_version(v));
        if version.is_none() {
            tracing::warn!(path = %path.display(), "marque de mise à jour mal formée, ignorée");
        }
        version
    }
}

#[cfg(test)]
#[path = "marks_tests.rs"]
mod marks_tests;
