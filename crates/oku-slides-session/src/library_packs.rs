//! La bibliothèque des packs : un rangement par sorte (widgets, layouts), un seul espace de noms.
//!
//! Chaque sorte a son dossier (`<data>/widgets`, `<data>/layouts`) et sa
//! [`Library`] ; un pack n'appartient qu'à une sorte, celle de ses fichiers.
//! Un espace de noms n'appartient qu'à un pack ou à un thème, toutes sortes
//! confondues (`NamespaceTaken`) : la bibliothèque des packs garde un clone de
//! celle des thèmes (même index) pour le vérifier. Une version d'un pack connu
//! sous l'autre sorte est refusée (`PackKindChanged`).

use std::path::PathBuf;

use snafu::ensure;
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackKind, PackVersion};
use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::error::{NamespaceReservedSnafu, NamespaceTakenSnafu, PackKindChangedSnafu};
use crate::library::{DraftLock, DraftWrite, Library, builtin_owner, may_own};
use crate::library_kind::{Layouts, StoredVersion, Widgets};
use crate::pack::{PackInfo, PackManifest, Relation};
use crate::theme::ThemeLibrary;

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
    /// Les thèmes, pour l'unicité des espaces de noms entre les trois sortes.
    themes: ThemeLibrary,
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
    ///
    /// `themes` : la bibliothèque des thèmes de la même session (un clone partage son index).
    #[must_use]
    pub fn new(widgets: Option<PathBuf>, layouts: Option<PathBuf>, themes: ThemeLibrary, storage: Storage) -> Self {
        Self { widgets: Library::new(widgets, storage.clone()), layouts: Library::new(layouts, storage), themes }
    }

    /// Les packs de l'utilisateur (`paths.widgets` et `paths.layouts` des réglages), et les intégrés.
    ///
    /// Ses thèmes sont ceux de l'utilisateur ([`ThemeLibrary::user`]).
    #[must_use]
    pub fn user(platform: &oku_slides_platform::Platform) -> Self {
        Self::with_themes(platform, ThemeLibrary::user(platform))
    }

    /// Comme [`Self::user`], avec la bibliothèque des thèmes `themes` (celle de la session).
    #[must_use]
    pub fn with_themes(platform: &oku_slides_platform::Platform, themes: ThemeLibrary) -> Self {
        let paths = &platform.settings.paths;
        Self::new(Some(paths.widgets()), Some(paths.layouts()), themes, platform.storage.clone())
    }

    /// La bibliothèque des thèmes que celle-ci consulte.
    #[must_use]
    pub fn themes(&self) -> &ThemeLibrary {
        &self.themes
    }

    /// Un pack intégré à l'app, de l'une ou l'autre sorte ?
    #[must_use]
    pub fn is_builtin(id: Uuid) -> bool {
        Library::<Widgets>::is_builtin(id) || Library::<Layouts>::is_builtin(id)
    }

    /// La sorte du pack `id` : celle qui le connaît ; inconnu, les widgets.
    #[must_use]
    pub fn kind_of(&self, id: Uuid) -> PackKind {
        if Library::<Layouts>::is_builtin(id) || (!Library::<Widgets>::is_builtin(id) && self.layouts.knows(id)) {
            PackKind::Layouts
        } else {
            PackKind::Widgets
        }
    }

    /// Les versions de `id`, chargées : l'intégrée d'abord, puis celles de l'utilisateur.
    #[must_use]
    pub fn versions(&self, id: Uuid) -> Vec<StoredVersion<PackManifest>> {
        by_kind!(self, self.kind_of(id), l => l.versions(id))
    }

    /// Les fichiers de la version `v` du pack `id`.
    #[must_use]
    pub fn version(&self, id: Uuid, v: &PackVersion) -> Option<PackFiles> {
        by_kind!(self, self.kind_of(id), l => l.version(id, v))
    }

    /// La version courante du pack `id` ; `None` pour un pack inconnu.
    #[must_use]
    pub fn current(&self, id: Uuid) -> Option<PackVersion> {
        by_kind!(self, self.kind_of(id), l => l.current(id))
    }

    /// La version courante du pack `id`, lue (voir [`Library::current_version`]).
    #[must_use]
    pub fn current_version(&self, id: Uuid) -> Option<StoredVersion<PackManifest>> {
        by_kind!(self, self.kind_of(id), l => l.current_version(id))
    }

    /// La version `v` de `id` est-elle arrivée avec un deck reçu ?
    #[must_use]
    pub fn is_received(&self, id: Uuid, v: &PackVersion) -> bool {
        by_kind!(self, self.kind_of(id), l => l.is_received(id, v))
    }

    /// Refuse `files` pour `id` si l'autre sorte connaît ce pack, ou si l'autre sorte ou un thème a son espace de noms.
    fn check_kinds(&self, id: Uuid, files: &PackFiles) -> Result<()> {
        let kind = files.kind();
        let elsewhere = by_kind!(self, other(kind), l => l.knows(id));
        ensure!(!elsewhere, PackKindChangedSnafu { id, found: kind.files_dir() });
        // Un manifeste illisible est refusé ensuite, par la bibliothèque de sa sorte.
        if let Ok(manifest) = PackManifest::from_files(files) {
            let ns = &manifest.namespace;
            let owner = by_kind!(self, other(kind), l => l.owner_of(ns)).or_else(|| self.themes.owner_of(ns));
            if let Some(owner) = owner {
                return NamespaceTakenSnafu { namespace: ns.as_str(), owner: owner.to_string() }.fail()?;
            }
        }
        Ok(())
    }

    /// Checks that theme `id` can take namespace `ns` in the library.
    ///
    /// Neither `deck` nor `deck-layouts` (`NamespaceReserved`), nor that of a pack
    /// (built-in included) or of another theme (`NamespaceTaken`). The theme that
    /// already holds it keeps it: a new version passes.
    ///
    /// # Errors
    ///
    /// Fails if `ns` is reserved (`NamespaceReserved`) or held by a pack or another theme
    /// (`NamespaceTaken`).
    pub fn check_theme(&self, id: Uuid, ns: &Namespace) -> Result<()> {
        ensure!(!ns.is_deck() && !ns.is_deck_layouts(), NamespaceReservedSnafu { namespace: ns.as_str() });
        let pack = builtin_owner(ns)
            .or_else(|| PackKind::ALL.into_iter().find_map(|k| by_kind!(self, k, l => l.owner_of(ns))));
        let owner = pack.or_else(|| self.themes.owner_of(ns).filter(|owner| *owner != id));
        if let Some(owner) = owner {
            return NamespaceTakenSnafu { namespace: ns.as_str(), owner: owner.to_string() }.fail()?;
        }
        Ok(())
    }

    /// Adds a version, under the kind of its files (see [`Library::add_version`]).
    ///
    /// # Errors
    ///
    /// Fails if `id` is already a pack of another kind (`PackKindChanged`), if the namespace is taken
    /// elsewhere, or like [`Library::add_version`].
    pub fn add_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.add_version(id, files))
    }

    /// Adds a version that arrived with a deck (see [`Library::receive_version`]).
    ///
    /// # Errors
    ///
    /// Fails if `id` is already a pack of another kind (`PackKindChanged`), if the namespace is taken
    /// elsewhere, or like [`Library::receive_version`].
    pub fn receive_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.receive_version(id, files))
    }

    /// Makes version `v` current for pack `id`.
    ///
    /// # Errors
    ///
    /// Fails like [`Library::set_current`]: unknown pack or version, failed write.
    pub fn set_current(&self, id: Uuid, v: &PackVersion) -> Result<()> {
        by_kind!(self, self.kind_of(id), l => l.set_current(id, v))
    }

    /// Comment enregistrer le brouillon `draft` du pack `id` (voir [`Library::draft_write`]).
    pub(crate) fn draft_write(&self, lock: &DraftLock, id: Uuid, draft: Option<&PackVersion>) -> DraftWrite {
        by_kind!(self, self.kind_of(id), l => l.draft_write(lock, id, draft))
    }

    /// Écrit le brouillon `files`, dans la sorte de ses fichiers (voir [`Library::save_draft`]).
    pub(crate) fn save_draft(
        &self,
        lock: &DraftLock,
        id: Uuid,
        write: &DraftWrite,
        files: &PackFiles,
    ) -> Result<PackVersion> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.save_draft(lock, id, write, files))
    }

    /// La version `v` du pack `id` est-elle figée (voir [`Library::is_frozen`]) ?
    pub(crate) fn is_frozen(&self, id: Uuid, v: &PackVersion) -> bool {
        by_kind!(self, self.kind_of(id), l => l.is_frozen(id, v))
    }

    /// L'index liste-t-il la version `v` du pack `id` ? Sans rien charger.
    pub(crate) fn lists(&self, id: Uuid, v: &PackVersion) -> bool {
        by_kind!(self, self.kind_of(id), l => l.lists(id, v))
    }

    /// Fige la version `files` du pack `id` (voir [`Library::adopt`]).
    pub(crate) fn adopt(&self, lock: &DraftLock, id: Uuid, files: &PackFiles) -> Result<()> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.adopt(lock, id, files))
    }

    /// `a` par rapport à `b` dans le pack `id`, par la chaîne des `parent`.
    #[must_use]
    pub fn relation(&self, id: Uuid, a: &PackVersion, b: &PackVersion) -> Relation {
        by_kind!(self, self.kind_of(id), l => l.relation(id, a, b))
    }

    /// Deletes the user's pack `id` (see [`Library::delete`]).
    ///
    /// # Errors
    ///
    /// Fails like [`Library::delete`]: missing folder or failed deletion.
    pub fn delete(&self, id: Uuid) -> Result<()> {
        by_kind!(self, self.kind_of(id), l => l.delete(id))
    }

    /// Les packs de la sorte `kind` dans leur version courante : intégrés d'abord, puis ceux de l'utilisateur, par nom.
    #[must_use]
    pub fn entries(&self, kind: PackKind) -> Vec<StoredVersion<PackManifest>> {
        by_kind!(self, kind, l => l.entries())
    }

    /// La version courante du pack qui porte l'espace de noms `ns`, de l'une ou l'autre sorte.
    ///
    /// Un espace de noms intégré désigne toujours le pack intégré, jamais une copie de l'utilisateur.
    #[must_use]
    pub fn by_namespace(&self, ns: &Namespace) -> Option<StoredVersion<PackManifest>> {
        PackKind::ALL.into_iter().find_map(|kind| self.owned_current(kind, ns))
    }

    /// La version courante du pack de sorte `kind` qui porte l'espace de noms `ns` (R15).
    ///
    /// Un espace de noms intégré n'appartient qu'à son pack intégré : une
    /// copie de l'utilisateur qui le porte n'est jamais un résultat.
    #[must_use]
    pub fn owned_current(&self, kind: PackKind, ns: &Namespace) -> Option<StoredVersion<PackManifest>> {
        self.entries(kind).into_iter().find(|s| Self::owns(s, ns))
    }

    /// Vrai si la version `s` est celle du pack qui porte `ns` : son espace de noms, sans le prendre à un intégré.
    pub(crate) fn owns(s: &StoredVersion<PackManifest>, ns: &Namespace) -> bool {
        s.manifest.namespace == *ns && may_own(s.manifest.id, ns)
    }

    /// Tous les espaces de noms pris dans la bibliothèque, toutes sortes confondues, thèmes compris.
    pub(crate) fn namespaces(&self) -> Vec<Namespace> {
        let packs = PackKind::ALL.into_iter().flat_map(|kind| self.entries(kind)).map(|s| s.manifest.namespace);
        packs.chain(self.themes.entries().into_iter().map(|s| s.manifest.namespace)).collect()
    }

    /// Packs de widgets lus sur le disque par cette instance depuis sa création.
    #[cfg(test)]
    pub(crate) fn widget_scans(&self) -> usize {
        self.widgets.scans()
    }

    /// Les packs de la sorte `kind` dans leur version courante, résumés pour les listes.
    #[must_use]
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
