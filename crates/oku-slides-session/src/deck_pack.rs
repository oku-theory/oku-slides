//! Les packs du deck, widgets et layouts : les versions embarquées dans ses sources font foi.
//!
//! Un deck sans pack (aucun épinglage d'aucune sorte : deck neuf, deck JSON
//! de développement) épingle la version courante de `base` (widgets) et
//! d'`essentiel` (layouts), et leur fermeture, embarquées dès son chargement.
//! Épingler un pack (`AddPack`) embarque d'abord la version voulue, prise
//! dans la bibliothèque. Toute version adoptée entre ainsi dans les sources
//! (en ajout seul) et s'enregistre avec le deck. La version intégrée
//! courante de `base` et d'`essentiel` se résout dans ce processus, sans
//! processus de rendu.

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Namespace, Op, PackFiles, PackPin, PackVersion, SourceBundle, WidgetRegistry};
use oku_slides_render::{BuiltinPack, PackSet, PackWidgets, RegistryError};

use crate::deck_closure::{Pinned, closure_ops};
use crate::error::{
    PackIdMismatchSnafu, PackManifestSnafu, PackPinMismatchSnafu, PackVersionMissingSnafu, SourcesSnafu,
    UnknownPackSnafu,
};
use crate::library::builtin_versions;
use crate::library_packs::PackLibrary;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK, PackManifest};
use crate::{Result, Session};

/// Un deck sans aucun pack épingle `base` et `essentiel`, et leurs besoins.
///
/// Un deck qui épingle déjà un pack, de l'une ou l'autre sorte, ne change pas :
/// ses épinglages ne sont jamais complétés à l'ouverture. Les versions
/// courantes sont embarquées dans `sources` et épinglées par le deck, sans
/// étape d'annulation. `base` est demandé lui-même : une version d'`essentiel`
/// de l'utilisateur peut ne pas le requérir. Seule règle des packs par défaut :
/// la session et la CLI passent par ici.
pub fn adopt_default_packs(library: &PackLibrary, sources: &mut SourceBundle) -> Result<()> {
    if sources.deck().all_pins().next().is_some() {
        return Ok(());
    }
    let mut wanted = Vec::new();
    for id in [BASE_PACK, ESSENTIEL_PACK] {
        let current = library.current_version(id).context(UnknownPackSnafu { id })?;
        let pin = PackPin { id, version: current.version, requires: current.manifest.requires };
        wanted.push(Op::AddPack { kind: current.files.kind(), namespace: current.manifest.namespace, pin });
    }
    let mut deck = sources.deck().clone();
    for op in closure_ops(library, &deck, &wanted)? {
        let Op::AddPack { kind, namespace, pin } = op else { continue };
        if deck.pins(kind).contains_key(&namespace) {
            continue;
        }
        let files = library
            .version(pin.id, &pin.version)
            .context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        check_pack_id(&namespace, &pin, &files)?;
        sources.add_pack(files);
        deck.pins_mut(kind).insert(namespace, pin);
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

/// La version intégrée courante du pack `id`, que ce processus résout sans processus de rendu.
pub(crate) fn builtin_version(id: Uuid) -> Option<&'static PackVersion> {
    builtin_versions().iter().find(|b| b.manifest.id == id).map(|b| &b.version)
}

/// L'id de bibliothèque d'un pack intégré.
fn builtin_id(pack: BuiltinPack) -> Uuid {
    match pack {
        BuiltinPack::Base => BASE_PACK,
        BuiltinPack::Essentiel => ESSENTIEL_PACK,
    }
}

/// Le pack intégré dont `version` est la version courante, s'il y en a un.
fn current_builtin(version: &PackVersion) -> Option<BuiltinPack> {
    BuiltinPack::ALL.into_iter().find(|b| builtin_version(builtin_id(*b)) == Some(version))
}

/// Les widgets du pack intégré courant de version `version`, s'il y en a un ; sinon `None` (à sonder).
///
/// Code embarqué et de confiance : ni sonde ni processus de rendu.
pub(crate) fn in_process_pack(version: &PackVersion) -> Option<Result<PackWidgets, RegistryError>> {
    current_builtin(version).map(BuiltinPack::widgets)
}

/// `set` tient-il exactement les packs intégrés courants ? Son registre est alors `builtin_registry`.
pub(crate) fn only_builtins(set: &PackSet) -> bool {
    let current = |p: &PackWidgets| {
        PackVersion::from_hex(p.id()).and_then(|v| current_builtin(&v)).is_some_and(|b| b.namespace() == *p.namespace())
    };
    set.packs().len() == BuiltinPack::ALL.len() && set.packs().iter().all(current)
}

/// Le registre sans pack : vide, le même `Arc` à chaque appel.
pub(crate) fn empty_registry() -> Arc<WidgetRegistry> {
    static EMPTY: LazyLock<Arc<WidgetRegistry>> = LazyLock::new(Arc::default);
    EMPTY.clone()
}

/// `files` sont-ils bien la version épinglée par `pin` sous `ns` ?
///
/// Manifeste lisible, d'id `pin.id`, d'espace de noms `ns` et de besoins
/// `pin.requires`. Seule règle : la session et la CLI passent par ici.
pub fn check_pack_id(ns: &Namespace, pin: &PackPin, files: &PackFiles) -> Result<()> {
    let manifest = PackManifest::from_files(files).context(PackManifestSnafu { id: pin.id })?;
    ensure!(manifest.id == pin.id, PackIdMismatchSnafu { id: pin.id, found: manifest.id });
    let same = manifest.namespace == *ns && manifest.requires == pin.requires;
    ensure!(same, PackPinMismatchSnafu { namespace: ns.as_str() });
    Ok(())
}

impl Session {
    /// Les packs de widgets épinglés par le deck de l'utilisateur, même pendant l'édition d'un pack.
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
