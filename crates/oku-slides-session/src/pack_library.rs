//! Ce que le mode pack lit et écrit dans la bibliothèque.
//!
//! Un pack de la bibliothèque part de sa version courante ; ses versions de
//! séance y sont écrites. Un pack du deck seul (`PackScope::Deck`, celui d'un
//! import) part de la version qu'embarque le deck ; ses versions restent dans
//! les sources des documents, jamais dans la bibliothèque.

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};

use crate::error::{
    DeckOnlySnafu, LeaseMissingSnafu, PackJsonSnafu, PackManifestSnafu, PackVersionMissingSnafu, UnknownPackSnafu,
};
use crate::library::{DraftLock, DraftWrite};
use crate::library_kind::StoredVersion;
use crate::library_packs::PackLibrary;
use crate::pack::PackManifest;
use crate::pack_lease::{PackLease, covers};
use crate::pack_receive::is_deck_only;
use crate::{Result, Session};

impl Session {
    /// La version d'où part l'édition du pack `id`.
    ///
    /// Un pack épinglé par le deck, s'il n'appartient qu'à lui : la version
    /// qu'il embarque. Sinon la courante de la bibliothèque. Un pack du deck seul
    /// que le deck n'utilise plus (resté dans l'historique) est refusé
    /// (`DeckOnly`) : la bibliothèque ne l'a pas. Celui du deck dont la version
    /// n'est pas embarquée l'est aussi (`PackVersionMissing`).
    pub(crate) fn pack_base(&self, id: Uuid) -> Result<StoredVersion<PackManifest>> {
        let packs = &self.user_blobs().packs;
        if let Some((_, _, pin)) = self.user_deck().all_pins().find(|(_, _, pin)| pin.id == id) {
            if let Some(files) = packs.get(&pin.version).filter(|f| is_deck_only(f)) {
                let manifest = PackManifest::from_files(files).context(PackManifestSnafu { id })?;
                return Ok(StoredVersion {
                    version: pin.version.clone(),
                    files: files.clone(),
                    manifest,
                    received: false,
                });
            }
            let missing = !packs.contains_key(&pin.version) && self.embeds_deck_only_pack(id);
            ensure!(!missing, PackVersionMissingSnafu { id, version: pin.version.clone() });
        }
        ensure!(!self.embeds_deck_only_pack(id), DeckOnlySnafu { id });
        Ok(self.library.current_version(id).context(UnknownPackSnafu { id })?)
    }
}

/// Writes the session version `files` of pack `id` as `write`; returns its version.
///
/// `write` comes from [`PackLibrary::draft_write`] under `lock`. A deck-only
/// pack writes nothing there. A library pack is written only with its
/// editor's `lease`, and only while the library still has it (a new pack, a
/// copy without parent, excepted).
pub(crate) fn write_session_version(
    library: &PackLibrary,
    lock: &DraftLock,
    id: Uuid,
    write: &DraftWrite,
    files: &PackFiles,
    lease: Option<&PackLease>,
) -> Result<PackVersion> {
    if is_deck_only(files) {
        return Ok(files.version());
    }
    let creating =
        matches!(write, DraftWrite::New) && PackManifest::from_files(files).is_ok_and(|m| m.parent.is_none());
    check_witness(library, id, lease, creating)?;
    library.save_draft(lock, id, write, files)
}

/// Refuses a library write of pack `id` without its `lease`, or of a pack the library lost.
///
/// `creating`: the write makes a new pack, which the library does not have yet.
fn check_witness(library: &PackLibrary, id: Uuid, lease: Option<&PackLease>, creating: bool) -> Result<()> {
    if !covers(lease, library.lease_scope(), id) {
        tracing::error!(pack = %id, "library pack written without its lease: refused");
        return LeaseMissingSnafu.fail()?;
    }
    // A delete that slipped in since the editor's checks never comes back through a save.
    ensure!(creating || library.current(id).is_some(), UnknownPackSnafu { id });
    Ok(())
}

/// `files` du pack `id`, refaits avec `parent` pour parente : la fille d'un brouillon figé.
pub(crate) fn rebased(id: Uuid, files: &PackFiles, parent: &PackVersion) -> Result<PackFiles> {
    let mut manifest = PackManifest::from_files(files).context(PackManifestSnafu { id })?;
    manifest.parent = Some(parent.clone());
    Ok(manifest.to_files(files.kind(), files.widgets().clone()).context(PackJsonSnafu { id })?)
}

/// Puts `files`, a session version undo can bring back, back into the library.
///
/// Without making it current; nothing for a deck-only pack. A failure is
/// logged: the deck embeds the version, nothing is lost. Without the
/// editor's `lease`, or for a pack the library lost, nothing is written (logged).
pub(crate) fn restore_session_version(library: &PackLibrary, id: Uuid, files: &PackFiles, lease: Option<&PackLease>) {
    if is_deck_only(files) {
        return;
    }
    if let Err(e) = check_witness(library, id, lease, false) {
        tracing::error!(error = %e.one_line(), "version de séance non rétablie : écriture refusée");
        return;
    }
    if let Err(e) = library.add_version(id, files) {
        tracing::error!(error = %e.one_line(), "version de séance non rétablie dans la bibliothèque");
    }
}
