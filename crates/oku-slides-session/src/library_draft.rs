//! Brouillons de la bibliothèque : remplacés sur place jusqu'à leur adoption par un deck.
//!
//! Une version est *figée* quand elle est intégrée, reçue avec un deck
//! (`received`) ou adoptée (`adopted`, écrit dès qu'un deck l'épingle). Un
//! brouillon figé n'est jamais remplacé : l'enregistrement suivant crée sa
//! fille. Décider ([`Library::draft_write`]) puis écrire ([`Library::save_draft`])
//! se fait sous [`DraftLock`], un verrou du processus que prend aussi
//! [`Library::adopt`] : une adoption ne se glisse jamais entre les deux.

use std::sync::{Mutex, MutexGuard, PoisonError};

use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};
use oku_slides_platform::errors::one_line;

use super::{Library, RECEIVED_FILE, VERSIONS_DIR};
use crate::Result;
use crate::error::{NoLibraryDirSnafu, WriteSnafu};
use crate::library_kind::{LibraryKind, VersionManifest};

/// Marque d'une version adoptée par un deck, dans son dossier.
const ADOPTED_FILE: &str = "adopted";

/// Le verrou des brouillons, commun à toutes les bibliothèques du processus.
static DRAFTS: Mutex<()> = Mutex::new(());

/// Preuve que le verrou des brouillons est pris ; à relâcher au plus tôt.
#[derive(Debug)]
pub(crate) struct DraftLock {
    /// Tenue pour sa durée de vie : la relâcher relâche le verrou.
    _guard: MutexGuard<'static, ()>,
}

/// Prend le verrou des brouillons, non réentrant : jamais deux fois sur un fil.
pub(crate) fn draft_lock() -> DraftLock {
    DraftLock { _guard: DRAFTS.lock().unwrap_or_else(PoisonError::into_inner) }
}

/// Comment s'écrit l'enregistrement d'un brouillon, décidé sous le verrou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DraftWrite {
    /// Pas encore de brouillon : une nouvelle version.
    New,
    /// Le brouillon, pas figé : remplacé sur place.
    Replace(PackVersion),
    /// Le brouillon, figé : une nouvelle version, dont il est la parente.
    Child(PackVersion),
}

impl DraftWrite {
    /// La version que l'écriture remplace ; `None` quand elle en crée une.
    pub(crate) fn replaced(&self) -> Option<&PackVersion> {
        match self {
            Self::Replace(v) => Some(v),
            Self::New | Self::Child(_) => None,
        }
    }
}

impl<K: LibraryKind> Library<K> {
    /// La version `v` de `id` est-elle figée : intégrée, reçue ou adoptée ?
    ///
    /// Une marque illisible compte pour figée : mieux vaut une fille qu'un remplacement à tort.
    pub(crate) fn is_frozen(&self, id: Uuid, v: &PackVersion) -> bool {
        if Self::builtin(id).is_some_and(|b| b.version == *v) {
            return true;
        }
        let Some(folder) = self.pack_dir(id).map(|d| d.join(VERSIONS_DIR).join(v.hex())) else { return false };
        [RECEIVED_FILE, ADOPTED_FILE].iter().any(|mark| {
            let path = folder.join(mark);
            self.marker(&path).unwrap_or_else(|e| {
                tracing::warn!(path = %path.display(), error = %one_line(&e), "marque de version illisible, version tenue pour figée");
                true
            })
        })
    }

    /// Comment enregistrer le brouillon `draft` de `id` ; le verrou reste pris jusqu'à l'écriture.
    pub(crate) fn draft_write(&self, _lock: &DraftLock, id: Uuid, draft: Option<&PackVersion>) -> DraftWrite {
        match draft {
            None => DraftWrite::New,
            Some(v) if self.is_frozen(id, v) => DraftWrite::Child(v.clone()),
            Some(v) => DraftWrite::Replace(v.clone()),
        }
    }

    /// Écrit `files`, l'enregistrement d'un brouillon, et la rend courante ; rend sa version.
    ///
    /// `write` vient de [`Self::draft_write`] sous le même verrou. Pour
    /// `Child`, l'appelant a fait du brouillon figé la parente de `files`.
    pub(crate) fn save_draft(
        &self,
        _lock: &DraftLock,
        id: Uuid,
        write: &DraftWrite,
        files: &PackFiles,
    ) -> Result<PackVersion> {
        if let DraftWrite::Child(parent) = write {
            let found = K::Manifest::from_files(files).ok().and_then(|m| m.parent().cloned());
            debug_assert_eq!(found.as_ref(), Some(parent), "la fille d'un brouillon figé descend de lui");
        }
        match write {
            DraftWrite::Replace(old) => self.replace_version(id, old, files),
            DraftWrite::New | DraftWrite::Child(_) => {
                let version = files.version();
                self.add_version(id, files)?;
                self.set_current(id, &version)?;
                Ok(version)
            }
        }
    }

    /// Fige la version `files` de `id`, qu'un deck épingle.
    ///
    /// L'appelant n'adopte qu'une version que la bibliothèque tient déjà, ou
    /// une locale : absente, elle serait rajoutée sans la marque `received`
    /// d'un deck reçu. Idempotent. La version courante ne change pas, sauf pour un pack
    /// inconnu (voir [`Self::add_version`]).
    pub(crate) fn adopt(&self, _lock: &DraftLock, id: Uuid, files: &PackFiles) -> Result<()> {
        let version = files.version();
        if self.is_frozen(id, &version) {
            return Ok(());
        }
        self.add_version(id, files)?;
        let dir = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?;
        let path = dir.join(VERSIONS_DIR).join(version.hex()).join(ADOPTED_FILE);
        self.storage.write_atomic(&path, b"").context(WriteSnafu { path: &path })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "library_draft_tests.rs"]
mod library_draft_tests;
