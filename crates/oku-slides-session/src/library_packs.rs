//! La bibliothèque des packs : un rangement par sorte (widgets, layouts), un seul espace de noms.
//!
//! Chaque sorte a son dossier (`<data>/widgets`, `<data>/layouts`) et sa
//! [`Library`] ; un pack n'appartient qu'à une sorte, celle de ses fichiers.
//! Un espace de noms n'appartient qu'à un pack, toutes sortes confondues
//! (`NamespaceTaken`) ; une version d'un pack connu sous l'autre sorte est
//! refusée (`PackKindChanged`).

use std::path::PathBuf;

use snafu::ensure;
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackKind, PackVersion};
use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::error::{NamespaceTakenSnafu, PackKindChangedSnafu};
use crate::library::{Library, may_own};
use crate::library_kind::{Layouts, StoredVersion, Widgets};
use crate::pack::{PackInfo, PackManifest, Relation};

/// Appelle `$body` avec `$l`, la bibliothèque de la sorte `$kind`.
macro_rules! by_kind {
    ($self:ident, $kind:expr, $l:ident => $body:expr) => {
        match $kind {
            PackKind::Widgets => {
                let $l = &$self.widgets;
                $body
            }
            PackKind::Layouts => {
                let $l = &$self.layouts;
                $body
            }
        }
    };
}

/// La bibliothèque des packs de widgets et de layouts, intégrés compris.
#[derive(Debug, Clone)]
pub struct PackLibrary {
    widgets: Library<Widgets>,
    layouts: Library<Layouts>,
}

/// L'autre sorte que `kind`.
fn other(kind: PackKind) -> PackKind {
    match kind {
        PackKind::Widgets => PackKind::Layouts,
        PackKind::Layouts => PackKind::Widgets,
    }
}

impl PackLibrary {
    /// `widgets` et `layouts` : les dossiers de chaque sorte, créés à la première écriture.
    pub fn new(widgets: Option<PathBuf>, layouts: Option<PathBuf>, storage: Storage) -> Self {
        Self { widgets: Library::new(widgets, storage.clone()), layouts: Library::new(layouts, storage) }
    }

    /// Les packs de l'utilisateur (`paths.widgets` et `paths.layouts` des réglages), et les intégrés.
    pub fn user(platform: &oku_slides_platform::Platform) -> Self {
        let paths = &platform.settings.paths;
        Self::new(Some(paths.widgets()), Some(paths.layouts()), platform.storage.clone())
    }

    /// Un pack intégré à l'app, de l'une ou l'autre sorte ?
    pub fn is_builtin(id: Uuid) -> bool {
        Library::<Widgets>::is_builtin(id) || Library::<Layouts>::is_builtin(id)
    }

    /// La sorte du pack `id` : celle qui le connaît ; inconnu, les widgets.
    pub fn kind_of(&self, id: Uuid) -> PackKind {
        if Library::<Layouts>::is_builtin(id) || (!Library::<Widgets>::is_builtin(id) && self.layouts.knows(id)) {
            PackKind::Layouts
        } else {
            PackKind::Widgets
        }
    }

    /// Les versions de `id`, chargées : l'intégrée d'abord, puis celles de l'utilisateur.
    pub fn versions(&self, id: Uuid) -> Vec<StoredVersion<PackManifest>> {
        by_kind!(self, self.kind_of(id), l => l.versions(id))
    }

    /// Les fichiers de la version `v` du pack `id`.
    pub fn version(&self, id: Uuid, v: &PackVersion) -> Option<PackFiles> {
        by_kind!(self, self.kind_of(id), l => l.version(id, v))
    }

    /// La version courante du pack `id` ; `None` pour un pack inconnu.
    pub fn current(&self, id: Uuid) -> Option<PackVersion> {
        by_kind!(self, self.kind_of(id), l => l.current(id))
    }

    /// La version courante du pack `id`, lue (voir [`Library::current_version`]).
    pub fn current_version(&self, id: Uuid) -> Option<StoredVersion<PackManifest>> {
        by_kind!(self, self.kind_of(id), l => l.current_version(id))
    }

    /// La version `v` de `id` est-elle arrivée avec un deck reçu ?
    pub fn is_received(&self, id: Uuid, v: &PackVersion) -> bool {
        by_kind!(self, self.kind_of(id), l => l.is_received(id, v))
    }

    /// Refuse `files` pour `id` si l'autre sorte connaît ce pack ou son espace de noms.
    fn check_kinds(&self, id: Uuid, files: &PackFiles) -> Result<()> {
        let kind = files.kind();
        let elsewhere = by_kind!(self, other(kind), l => l.knows(id));
        ensure!(!elsewhere, PackKindChangedSnafu { id, found: kind.files_dir() });
        // Un manifeste illisible est refusé ensuite, par la bibliothèque de sa sorte.
        if let Ok(manifest) = PackManifest::from_files(files) {
            let ns = &manifest.namespace;
            if let Some(owner) = by_kind!(self, other(kind), l => l.owner_of(ns)) {
                return NamespaceTakenSnafu { namespace: ns.as_str(), owner: owner.to_string() }.fail()?;
            }
        }
        Ok(())
    }

    /// Ajoute une version, dans la sorte de ses fichiers (voir [`Library::add_version`]).
    pub fn add_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.add_version(id, files))
    }

    /// Ajoute une version arrivée avec un deck (voir [`Library::receive_version`]).
    pub fn receive_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.receive_version(id, files))
    }

    /// Rend la version `v` courante pour le pack `id`.
    pub fn set_current(&self, id: Uuid, v: &PackVersion) -> Result<()> {
        by_kind!(self, self.kind_of(id), l => l.set_current(id, v))
    }

    /// Remplace la version `old` par `files`, rendue courante (voir [`Library::replace_version`]).
    pub fn replace_version(&self, id: Uuid, old: &PackVersion, files: &PackFiles) -> Result<PackVersion> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.replace_version(id, old, files))
    }

    /// `a` par rapport à `b` dans le pack `id`, par la chaîne des `parent`.
    pub fn relation(&self, id: Uuid, a: &PackVersion, b: &PackVersion) -> Relation {
        by_kind!(self, self.kind_of(id), l => l.relation(id, a, b))
    }

    /// Supprime le pack `id` de l'utilisateur (voir [`Library::delete`]).
    pub fn delete(&self, id: Uuid) -> Result<()> {
        by_kind!(self, self.kind_of(id), l => l.delete(id))
    }

    /// Les packs de la sorte `kind` dans leur version courante : intégrés d'abord, puis ceux de l'utilisateur, par nom.
    pub fn entries(&self, kind: PackKind) -> Vec<StoredVersion<PackManifest>> {
        by_kind!(self, kind, l => l.entries())
    }

    /// La version courante du pack qui porte l'espace de noms `ns`, de l'une ou l'autre sorte.
    ///
    /// Un espace de noms intégré désigne toujours le pack intégré, jamais une copie de l'utilisateur.
    pub fn by_namespace(&self, ns: &Namespace) -> Option<StoredVersion<PackManifest>> {
        PackKind::ALL.into_iter().find_map(|kind| {
            self.entries(kind).into_iter().find(|s| s.manifest.namespace == *ns && may_own(s.manifest.id, ns))
        })
    }

    /// Tous les espaces de noms pris dans la bibliothèque, toutes sortes confondues.
    pub(crate) fn namespaces(&self) -> Vec<Namespace> {
        PackKind::ALL.into_iter().flat_map(|kind| self.entries(kind)).map(|s| s.manifest.namespace).collect()
    }

    /// Packs de widgets lus sur le disque par cette instance depuis sa création.
    #[cfg(test)]
    pub(crate) fn widget_scans(&self) -> usize {
        self.widgets.scans()
    }

    /// Les packs de la sorte `kind` dans leur version courante, résumés pour les listes.
    pub fn packs(&self, kind: PackKind) -> Vec<PackInfo> {
        self.entries(kind)
            .into_iter()
            .map(|s| PackInfo::of(&s.files, &s.manifest, Self::is_builtin(s.manifest.id), s.received))
            .collect()
    }
}

#[cfg(test)]
#[path = "library_packs_tests.rs"]
mod library_packs_tests;
