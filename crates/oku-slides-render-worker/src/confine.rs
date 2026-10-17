//! Le côté travailleur du bac à sable : le canari de `Hello` et le dossier vidé.

use std::path::Path;

use oku_slides_platform::storage::{Storage, StorageError, StorageErrorKind};

use crate::protocol::Canary;

/// Tente de créer le fichier vide `path` ; dit ce que le système a répondu.
pub(crate) fn probe_canary(storage: &Storage, path: &Path) -> Canary {
    match storage.write(path, Vec::new()) {
        Ok(()) => Canary::Written,
        Err(e) if e.kind() == StorageErrorKind::PermissionDenied => Canary::Denied,
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "canari du bac à sable");
            Canary::Failed
        }
    }
}

/// Vide `work` et le recrée : un deck ne laisse rien au suivant.
pub(crate) fn wipe(storage: &Storage, work: &Path) -> Result<(), StorageError> {
    storage.delete_dir(work)?;
    storage.create_dir_all(work)
}

#[cfg(test)]
#[path = "confine_tests.rs"]
mod confine_tests;
