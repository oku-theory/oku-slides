//! Le pack du deck : la version embarquée dans ses sources fait foi.
//!
//! Un deck sans pack (`templates: None` : deck neuf, deck JSON de
//! développement) prend la version courante du pack par défaut, embarquée
//! et référencée dès son chargement. Changer de pack (`SetTemplates`)
//! embarque d'abord la version voulue, prise dans la bibliothèque. Toute
//! version adoptée entre ainsi dans les sources (en ajout seul) et
//! s'enregistre avec le deck.

use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{PackFiles, PackRef, SourceBundle};

use crate::error::{PackIdMismatchSnafu, PackJsonSnafu, PackVersionMissingSnafu, SourcesSnafu, UnknownPackSnafu};
use crate::library::Library;
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::{Result, Session};

/// Un deck sans pack adopte la version courante du pack par défaut.
///
/// Elle est embarquée dans `sources` et référencée par le deck, sans étape
/// d'annulation. Un deck qui a déjà un pack ne change pas. Seule règle du
/// pack par défaut : la session et la CLI passent par ici.
pub fn adopt_default_pack(library: &Library, sources: &mut SourceBundle) -> Result<()> {
    if sources.deck().templates.is_some() {
        return Ok(());
    }
    let current = library.current_version(DEFAULT_PACK).context(UnknownPackSnafu { id: DEFAULT_PACK })?;
    let version = sources.add_pack(current.files);
    let mut deck = sources.deck().clone();
    deck.templates = Some(PackRef { id: DEFAULT_PACK, version });
    sources.apply_silent(deck).context(SourcesSnafu)?;
    Ok(())
}

/// `files` sont-ils bien une version du pack `r.id` ?
fn check_pack_id(r: &PackRef, files: &PackFiles) -> Result<()> {
    let manifest = PackManifest::from_files(files).context(PackJsonSnafu { id: r.id })?;
    ensure!(manifest.id == r.id, PackIdMismatchSnafu { id: r.id, found: manifest.id });
    Ok(())
}

impl Session {
    /// Le pack du deck de l'utilisateur, même pendant l'édition des modèles.
    pub(crate) fn deck_pack(&self) -> Option<&PackRef> {
        self.user_deck().templates.as_ref()
    }

    /// Embarque la version `r` dans les sources du document, si elle n'y est pas.
    ///
    /// Elle vient de la bibliothèque ; absente : `PackVersionMissing`. Une
    /// version (embarquée ou de la bibliothèque) d'un autre pack que `r.id`
    /// est refusée (`PackIdMismatch`), sans rien embarquer.
    pub(crate) fn embed_from_library(&mut self, r: &PackRef) -> Result<()> {
        if let Some(files) = self.doc.sources.packs.get(&r.version) {
            return check_pack_id(r, files);
        }
        let files = self
            .library
            .version(r.id, &r.version)
            .context(PackVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
        check_pack_id(r, &files)?;
        self.doc.sources.add_pack(files);
        Ok(())
    }

    /// Nom du pack `id` dans sa version courante ; `None` s'il est inconnu.
    pub fn library_pack_name(&self, id: uuid::Uuid) -> Option<String> {
        self.library.current_version(id).map(|s| s.manifest.name)
    }

    /// Nom du pack dont les widgets sont installés, lu dans sa version embarquée.
    pub(crate) fn pack_name(&self) -> Option<String> {
        let r = self.widgets_templates.as_ref()?;
        let files = self.doc.sources.packs.get(&r.version)?;
        PackManifest::from_files(files).ok().map(|m| m.name)
    }
}

#[cfg(test)]
#[path = "deck_pack_tests.rs"]
mod deck_pack_tests;
