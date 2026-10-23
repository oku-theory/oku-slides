//! Ce que le mode pack lit et écrit dans la bibliothèque.
//!
//! Un pack de la bibliothèque part de sa version courante ; ses versions de
//! séance y sont écrites. Un pack du deck seul (`PackScope::Deck`, celui d'un
//! import) part de la version qu'embarque le deck ; ses versions restent dans
//! les sources des documents, jamais dans la bibliothèque.

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};

use crate::error::{DeckOnlySnafu, PackManifestSnafu, PackVersionMissingSnafu, UnknownPackSnafu};
use crate::library_kind::StoredVersion;
use crate::library_packs::PackLibrary;
use crate::pack::PackManifest;
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

/// Écrit `files`, version de séance du pack `id`, dans la bibliothèque ; rend sa version.
///
/// Créée et rendue courante la première fois, elle remplace ensuite
/// `replaced`. Un pack du deck seul n'y écrit rien.
pub(crate) fn write_session_version(
    library: &PackLibrary,
    id: Uuid,
    replaced: Option<&PackVersion>,
    files: &PackFiles,
) -> Result<PackVersion> {
    let version = files.version();
    if is_deck_only(files) {
        return Ok(version);
    }
    match replaced {
        Some(old) => library.replace_version(id, old, files),
        None => {
            library.add_version(id, files)?;
            library.set_current(id, &version)?;
            Ok(version)
        }
    }
}

/// Remet `files`, une version de séance qu'annuler peut ramener, dans la bibliothèque.
///
/// Sans la rendre courante ; rien pour un pack du deck seul. Un échec est
/// journalisé : le deck embarque la version, rien n'est perdu.
pub(crate) fn restore_session_version(library: &PackLibrary, id: Uuid, files: &PackFiles) {
    if is_deck_only(files) {
        return;
    }
    if let Err(e) = library.add_version(id, files) {
        tracing::error!(error = %e.one_line(), "version de séance non rétablie dans la bibliothèque");
    }
}
