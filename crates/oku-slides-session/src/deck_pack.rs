//! Les packs de widgets du deck : les versions embarquées dans ses sources font foi.
//!
//! Un deck sans pack (`widgets` vide : deck neuf, deck JSON de
//! développement) épingle la version courante de `base` et du pack par
//! défaut (`essentiel`), et leur fermeture, embarquées dès son chargement.
//! Épingler un pack (`AddPack`) embarque d'abord la version voulue, prise
//! dans la bibliothèque. Toute version adoptée entre ainsi dans les sources
//! (en ajout seul) et s'enregistre avec le deck. La version intégrée
//! courante de `base` se résout dans ce processus, sans processus de rendu.

use std::collections::BTreeMap;

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Namespace, Op, PackFiles, PackPin, PackVersion, SourceBundle, kinds};
use oku_slides_render::{PackSet, PackWidgets, RegistryError};

use crate::deck_closure::{Pinned, closure_ops};
use crate::error::{
    PackIdMismatchSnafu, PackJsonSnafu, PackPinMismatchSnafu, PackVersionMissingSnafu, SourcesSnafu, UnknownPackSnafu,
};
use crate::library::{PackLibrary, builtin_versions};
use crate::pack::{BASE_PACK, DEFAULT_PACK, PackManifest};
use crate::{Result, Session};

/// Un deck sans pack épingle `base` et le pack par défaut, et leurs besoins.
///
/// Un deck qui épingle déjà un pack ne change pas : ses épinglages ne sont
/// jamais complétés à l'ouverture. Les versions courantes sont embarquées
/// dans `sources` et épinglées par le deck, sans étape d'annulation. `base`
/// est demandé lui-même : une version d'Essentiel de l'utilisateur peut ne
/// pas le requérir. Seule règle des packs par défaut : la session et la CLI
/// passent par ici.
pub fn adopt_default_packs(library: &PackLibrary, sources: &mut SourceBundle) -> Result<()> {
    if !sources.deck().widgets.is_empty() {
        return Ok(());
    }
    let mut wanted = Vec::new();
    for id in [BASE_PACK, DEFAULT_PACK] {
        let current = library.current_version(id).context(UnknownPackSnafu { id })?;
        let pin = PackPin { id, version: current.version, requires: current.manifest.requires };
        wanted.push(Op::AddPack { namespace: current.manifest.namespace, pin });
    }
    let mut deck = sources.deck().clone();
    for op in closure_ops(library, &deck, &wanted)? {
        let Op::AddPack { namespace, pin } = op else { continue };
        if deck.widgets.contains_key(&namespace) {
            continue;
        }
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

/// L'épinglage de la version intégrée du pack `id`, sous son espace de noms, avec ses fichiers.
///
/// L'aide et l'import épinglent ainsi ce qu'ils écrivent, jamais la version de l'utilisateur.
pub(crate) fn builtin_pin(id: Uuid) -> Result<Pinned> {
    let b = builtin_versions().iter().find(|b| b.manifest.id == id).context(UnknownPackSnafu { id })?;
    let pin = PackPin { id, version: b.version.clone(), requires: b.manifest.requires.clone() };
    Ok((b.manifest.namespace.clone(), pin, b.files.clone()))
}

/// La version intégrée courante de `base`, que ce processus résout sans processus de rendu.
fn builtin_base_version() -> Option<&'static PackVersion> {
    builtin_versions().iter().find(|b| b.manifest.id == BASE_PACK).map(|b| &b.version)
}

/// Les widgets du `base` intégré courant si `version` est la sienne ; sinon `None` (à sonder).
///
/// Code embarqué et de confiance, comme le noyau : ni sonde ni processus de rendu.
pub(crate) fn in_process_pack(version: &PackVersion) -> Option<Result<PackWidgets, RegistryError>> {
    (builtin_base_version() == Some(version)).then(oku_slides_render::base_pack_widgets)
}

/// `set` ne tient-il que le `base` intégré courant ? Son registre est alors `builtin_registry`.
pub(crate) fn only_builtin_base(set: &PackSet) -> bool {
    let builtin = |p: &PackWidgets| {
        p.namespace().as_str() == kinds::base::NAMESPACE && builtin_base_version().is_some_and(|v| p.id() == v.hex())
    };
    !set.is_empty() && set.packs().iter().all(builtin)
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
