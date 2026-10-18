//! Les packs de widgets du deck : les versions embarquées dans ses sources font foi.
//!
//! Un deck sans pack (`widgets` vide : deck neuf, deck JSON de
//! développement) épingle la version courante du pack par défaut
//! (`essentiel`) et sa fermeture, embarquées dès son chargement. Épingler un
//! pack (`AddPack`) embarque d'abord la version voulue, prise dans la
//! bibliothèque. Toute version adoptée entre ainsi dans les sources (en
//! ajout seul) et s'enregistre avec le deck.

use std::collections::BTreeMap;

use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{Namespace, Op, PackFiles, PackPin, SourceBundle};

use crate::deck_closure::closure_ops;
use crate::error::{
    PackIdMismatchSnafu, PackJsonSnafu, PackPinMismatchSnafu, PackVersionMissingSnafu, SourcesSnafu, UnknownPackSnafu,
};
use crate::library::PackLibrary;
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::{Result, Session};

/// Un deck sans pack épingle la version courante du pack par défaut et ses besoins.
///
/// Elles sont embarquées dans `sources` et épinglées par le deck, sans étape
/// d'annulation. Un deck qui épingle déjà un pack ne change pas. Seule règle
/// des packs par défaut : la session et la CLI passent par ici.
pub fn adopt_default_packs(library: &PackLibrary, sources: &mut SourceBundle) -> Result<()> {
    if !sources.deck().widgets.is_empty() {
        return Ok(());
    }
    let default = library.current_version(DEFAULT_PACK).context(UnknownPackSnafu { id: DEFAULT_PACK })?;
    let pin = PackPin { id: DEFAULT_PACK, version: default.version, requires: default.manifest.requires };
    let wanted = Op::AddPack { namespace: default.manifest.namespace, pin };
    let mut deck = sources.deck().clone();
    for op in closure_ops(library, &deck, &[wanted])? {
        let Op::AddPack { namespace, pin } = op else { continue };
        let files = library
            .version(pin.id, &pin.version)
            .context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        check_pack_id(&namespace, &pin, &files)?;
        sources.add_pack(files);
        deck.widgets.insert(namespace, pin);
    }
    sources.apply_silent(deck).context(SourcesSnafu)?;
    Ok(())
}

/// `files` sont-ils bien la version épinglée par `pin` sous `ns` ?
///
/// Manifeste lisible, d'id `pin.id`, d'espace de noms `ns` et de besoins
/// `pin.requires`. Seule règle : la session et la CLI passent par ici.
pub fn check_pack_id(ns: &Namespace, pin: &PackPin, files: &PackFiles) -> Result<()> {
    let manifest = PackManifest::from_files(files).context(PackJsonSnafu { id: pin.id })?;
    ensure!(manifest.id == pin.id, PackIdMismatchSnafu { id: pin.id, found: manifest.id });
    let same = manifest.namespace == *ns && manifest.requires == pin.requires;
    ensure!(same, PackPinMismatchSnafu { namespace: ns.as_str() });
    Ok(())
}

impl Session {
    /// Les packs épinglés par le deck de l'utilisateur, même pendant l'édition des modèles.
    pub(crate) fn deck_pins(&self) -> &BTreeMap<Namespace, PackPin> {
        &self.user_deck().widgets
    }

    /// Embarque la version épinglée par `pin` sous `ns` dans les sources du document.
    ///
    /// Déjà embarquée, elle est seulement vérifiée ; sinon elle vient de la
    /// bibliothèque (absente : `PackVersionMissing`). Une version qui ne
    /// correspond pas à l'épinglage est refusée, sans rien embarquer.
    pub(crate) fn embed_from_library(&mut self, ns: &Namespace, pin: &PackPin) -> Result<()> {
        if let Some(files) = self.doc.sources.packs.get(&pin.version) {
            return check_pack_id(ns, pin, files);
        }
        let files = self
            .library
            .version(pin.id, &pin.version)
            .context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        check_pack_id(ns, pin, &files)?;
        self.doc.sources.add_pack(files);
        Ok(())
    }

    /// Nom du pack `id` dans sa version courante ; `None` s'il est inconnu.
    pub fn library_pack_name(&self, id: uuid::Uuid) -> Option<String> {
        self.library.current_version(id).map(|s| s.manifest.name)
    }
}

#[cfg(test)]
#[path = "deck_pack_tests.rs"]
mod deck_pack_tests;
