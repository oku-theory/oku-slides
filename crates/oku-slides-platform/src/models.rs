//! Pinned model files (repository, revision, SHA-256), downloaded on demand.
//!
//! Each file arrives as `.part`, hashed on the fly, then renamed; the [`COMPLETE`] marker
//! is written only once every file is verified. A cut `.part` of a pinned file is resumed
//! with `Range` (t-1052). Untrusted bytes: a different SHA-256, or a file longer than its
//! [`PinnedFile::size`], deletes the file.

use std::path::Path;

use crate::storage::Storage;

#[cfg(feature = "download")]
mod download;
#[cfg(feature = "download")]
mod resume;
#[cfg(feature = "download")]
pub use download::{DownloadError, download};

/// Marqueur écrit quand tous les fichiers d'un modèle sont là et vérifiés.
pub const COMPLETE: &str = ".complete";

/// Un fichier d'un dépôt Hugging Face, à une révision donnée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedFile {
    /// Dépôt, `organisation/nom`.
    pub repo: &'static str,
    /// Commit du dépôt (ou `main` pour un fichier non épinglé).
    pub revision: &'static str,
    /// Nom du fichier dans le dépôt et sur le disque.
    pub name: &'static str,
    /// SHA-256 attendu, en hexadécimal minuscule ; `None` : non vérifié.
    pub sha256: Option<&'static str>,
    /// Size in bytes: exact when pinned, an upper bound otherwise.
    ///
    /// The download stops past it (t-1376): an endless body cannot fill the disk before the
    /// hash check.
    pub size: u64,
}

/// Nature d'un `DownloadError` (avec la feature `download`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadErrorKind {
    /// Dépôt injoignable, refus ou coupure en route.
    Network,
    /// Écriture impossible sur le disque.
    Write,
    /// Fichier plus court que la taille annoncée.
    Incomplete {
        /// Octets reçus.
        got: u64,
        /// Octets annoncés.
        expected: u64,
    },
    /// A SHA-256 other than the pinned one, or a file longer than its size.
    Integrity,
    /// Annulé par la fonction de progression.
    Cancelled,
}

/// Le modèle rangé dans `dir` est-il complet (marqueur présent) ?
pub fn is_complete(storage: &Storage, dir: &Path) -> bool {
    match storage.exists(&dir.join(COMPLETE)) {
        Ok(found) => found,
        Err(e) => {
            // Lecture impossible : on propose le téléchargement, qui réécrira le marqueur.
            tracing::warn!(dir = %dir.display(), error = %crate::errors::one_line(&e), "marqueur de modèle illisible");
            false
        }
    }
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
