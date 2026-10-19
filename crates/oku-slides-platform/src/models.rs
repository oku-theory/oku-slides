//! Fichiers de modèles épinglés (dépôt, révision, SHA-256), téléchargés à la demande.
//!
//! Chaque fichier arrive en `.part`, haché au fil de l'eau, puis renommé ; le marqueur
//! [`COMPLETE`] n'est écrit qu'une fois tous les fichiers vérifiés. Octets non fiables :
//! un SHA-256 différent supprime le fichier.

use std::path::Path;

use crate::storage::Storage;

#[cfg(feature = "download")]
mod download;
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
    /// SHA-256 différent de celui épinglé.
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
