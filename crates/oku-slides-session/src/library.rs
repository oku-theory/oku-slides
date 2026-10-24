//! Bibliothèque versionnée, générique sur la sorte ([`LibraryKind`]) : packs de widgets, de layouts et thèmes.
//!
//! Une entrée est un id et des versions adressées par contenu ; chaque version
//! connaît sa parente (`parent` de son manifeste). Sur disque, sous le dossier
//! de la sorte :
//!
//! ```text
//! <data>/widgets/<uuid>/current                     version courante
//! <data>/widgets/<uuid>/versions/<hex>/widgets.json
//! <data>/widgets/<uuid>/versions/<hex>/widgets/<nom>.typ  (un par fichier de widget)
//! <data>/widgets/<uuid>/versions/<hex>/received          version reçue avec un deck
//!
//! <data>/layouts/<uuid>/versions/<hex>/layouts.json      (de même, fichiers sous `layouts/`)
//!
//! <data>/themes/<uuid>/current
//! <data>/themes/<uuid>/versions/<hex>/theme.json            (jamais de widgets)
//! ```
//!
//! Entrées intégrées (`typst/oku-slides/packs/{base,essentiel}`,
//! `typst/oku-slides/themes/*` du dépôt) : id constant, version de l'app en
//! lecture seule, courante tant que l'utilisateur n'a pas de fichier
//! `current`. La version courante ne change que par [`Library::set_current`],
//! ou à la création d'une entrée inconnue. Une version reçue avec un deck
//! ([`Library::receive_version`]) est marquée : son `created` vient d'ailleurs,
//! et le repli de la version courante ne lui fait pas confiance.
//!
//! Un espace de noms n'appartient qu'à un pack de la bibliothèque, toutes
//! sortes confondues ([`crate::PackLibrary`]) : une version d'un autre id sous
//! le même espace de noms est refusée (`NamespaceTaken`), comme une version
//! d'un pack connu sous un autre espace de noms que le sien (`NamespaceChanged`).
//! L'espace de noms d'un pack intégré lui appartient dans les deux sortes : une
//! ancienne copie de l'utilisateur qui le porte se lit par son id, mais n'a plus
//! de version neuve et ne répond plus aux recherches par nom ou espace de noms.
//! `deck` et `deck-layouts` sont réservés aux packs propres d'un deck
//! (`NamespaceReserved`). L'ancien dossier `templates/` n'est pas relu.
//!
//! Un index en mémoire ([`crate::library_index`]) évite de relire et
//! re-hacher toutes les versions à chaque accès ; charger une version vérifie
//! toujours ses octets.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackVersion, WidgetFile};
use oku_slides_platform::errors::one_line;
use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::error::{
    DeckOnlySnafu, NamespaceChangedSnafu, NamespaceReservedSnafu, NamespaceTakenSnafu, NoLibraryDirSnafu,
    PackIdMismatchSnafu, PackJsonSnafu, PackVersionMissingSnafu, UnknownPackSnafu, WriteSnafu,
};
use crate::library_index::{Current, Index, Indexed, PackEntry};
use crate::library_kind::{Layouts, LibraryKind, StoredVersion, VersionManifest, Widgets};
use crate::pack::{PackManifest, Relation};

/// Version courante d'un pack, dans son dossier.
const CURRENT_FILE: &str = "current";
/// Versions d'un pack, dans son dossier.
const VERSIONS_DIR: &str = "versions";
/// Marque d'une version reçue avec un deck, dans son dossier.
const RECEIVED_FILE: &str = "received";

/// Versions des packs intégrés des deux sortes, lues une fois par processus.
static BUILTIN_PACKS: LazyLock<Vec<StoredVersion<PackManifest>>> =
    LazyLock::new(|| Widgets::builtins().iter().chain(Layouts::builtins()).cloned().collect());

/// Versions des packs intégrés, dans l'ordre : `base` (widgets), puis `essentiel` (layouts).
pub fn builtin_versions() -> &'static [StoredVersion<PackManifest>] {
    &BUILTIN_PACKS
}

/// L'id du pack intégré qui porte l'espace de noms `ns`, de l'une ou l'autre sorte.
///
/// Un espace de noms intégré (`base`, `essentiel`) n'appartient qu'à ce pack : un pack de
/// l'utilisateur qui le porte (une copie d'avant le lot 3) se lit par son id, sans plus.
pub(crate) fn builtin_owner(ns: &Namespace) -> Option<Uuid> {
    BUILTIN_PACKS.iter().find(|b| b.manifest.namespace == *ns).map(|b| b.manifest.id)
}

/// L'entrée `id` peut-elle porter l'espace de noms `ns` ? Pas celui d'un autre pack intégré.
pub(crate) fn may_own(id: Uuid, ns: &Namespace) -> bool {
    builtin_owner(ns).is_none_or(|owner| owner == id)
}

/// La plus récente par `created` (la dernière vue à égalité), parmi les
/// versions créées ici d'abord : le `created` d'une version reçue est celui
/// qu'annonce son auteur, il ne compte que faute d'autre version.
fn latest<M: VersionManifest>(versions: &[Indexed<M>]) -> Option<PackVersion> {
    let newest = |received: bool| {
        versions
            .iter()
            .filter(|s| s.received == received)
            .max_by(|a, b| a.manifest.created().cmp(b.manifest.created()))
            .map(|s| s.version.clone())
    };
    newest(false).or_else(|| newest(true))
}

/// Qui ajoute une version à la bibliothèque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// Créée ici (édition, import, réalignement).
    Local,
    /// Arrivée avec un deck reçu.
    Received,
}

/// `child` descend-il de `ancestor` ? Un cycle (fichiers forgés) s'arrête.
fn descends(parents: &HashMap<PackVersion, Option<PackVersion>>, child: &PackVersion, ancestor: &PackVersion) -> bool {
    let mut seen = HashSet::new();
    let mut at = parents.get(child).cloned().flatten();
    while let Some(v) = at {
        if v == *ancestor {
            return true;
        }
        if !seen.insert(v.clone()) {
            return false;
        }
        at = parents.get(&v).cloned().flatten();
    }
    false
}

/// Dossiers au format ancien déjà signalés, pour le processus.
static LEGACY_SEEN: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Mutex::default);

/// Vrai la première fois que le dossier ancien `dir` est vu dans le processus.
fn first_legacy_sighting(dir: &Path) -> bool {
    LEGACY_SEEN.lock().unwrap_or_else(PoisonError::into_inner).insert(dir.to_path_buf())
}

/// La version courante parmi `entry` : `current` absent, l'intégrée, sinon la
/// plus récente ; `current` vers une version absente, la plus récente.
fn choose<M: VersionManifest>(entry: &PackEntry<M>, builtin: Option<&PackVersion>, id: Uuid) -> Option<PackVersion> {
    match &entry.current {
        Current::At(v) if entry.versions.iter().any(|s| s.version == *v) => Some(v.clone()),
        Current::Absent => builtin.cloned().or_else(|| latest(&entry.versions)),
        Current::At(_) | Current::Broken => {
            tracing::warn!(pack = %id, "version courante absente : la plus récente la remplace");
            latest(&entry.versions)
        }
    }
}

/// Bibliothèque : versions intégrées et versions de l'utilisateur, d'une sorte.
///
/// Ses clones partagent un même index ; deux instances distinctes (autre
/// onglet, autre processus) se voient par le fichier `generation`.
#[derive(Debug, Clone)]
pub struct Library<K: LibraryKind> {
    dir: Option<PathBuf>,
    storage: Storage,
    index: Arc<Mutex<Index<K::Manifest>>>,
    kind: PhantomData<K>,
}

impl<K: LibraryKind> Library<K> {
    /// `dir` : `<data>/<K::DIR>`, créé à la première écriture.
    pub fn new(dir: Option<PathBuf>, storage: Storage) -> Self {
        Self { dir, storage, index: Arc::default(), kind: PhantomData }
    }

    /// Une entrée intégrée à l'app ?
    pub fn is_builtin(id: Uuid) -> bool {
        Self::builtin(id).is_some()
    }

    fn builtin(id: Uuid) -> Option<&'static StoredVersion<K::Manifest>> {
        K::builtins().iter().find(|b| b.manifest.id() == id)
    }

    /// Dossier de l'entrée `id` ; `None` sans dossier de la bibliothèque.
    fn pack_dir(&self, id: Uuid) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(id.to_string()))
    }

    /// L'index, vidé si un autre a écrit dans la bibliothèque depuis sa construction.
    fn fresh(&self) -> MutexGuard<'_, Index<K::Manifest>> {
        let mut index = self.index.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(dir) = &self.dir {
            index.refresh(&self.storage, dir);
        }
        index
    }

    /// Reporte une écriture : nouvelle génération, vue par les autres instances.
    fn bump(&self, index: &mut Index<K::Manifest>) {
        if let Some(dir) = &self.dir {
            index.bump(&self.storage, dir);
        }
    }

    /// Le pack `id` dans l'index, lu du disque s'il n'y est pas encore.
    fn entry<'a>(&self, index: &'a mut Index<K::Manifest>, id: Uuid) -> &'a mut PackEntry<K::Manifest> {
        #[cfg(test)]
        if !index.packs.contains_key(&id) {
            index.scans += 1;
        }
        index.packs.entry(id).or_insert_with(|| {
            let mut versions: Vec<Indexed<K::Manifest>> = Self::builtin(id)
                .map(|b| Indexed { version: b.version.clone(), manifest: b.manifest.clone(), received: false })
                .into_iter()
                .collect();
            for s in self.user_versions(id) {
                if !versions.iter().any(|o| o.version == s.version) {
                    versions.push(Indexed { version: s.version, manifest: s.manifest, received: s.received });
                }
            }
            PackEntry { versions, current: self.read_current(id) }
        })
    }

    /// Versions de l'utilisateur ; une version illisible ou altérée est journalisée et ignorée.
    fn user_versions(&self, id: Uuid) -> Vec<StoredVersion<K::Manifest>> {
        let Some(dir) = self.pack_dir(id).map(|d| d.join(VERSIONS_DIR)) else { return Vec::new() };
        let folders = self.storage.list_dirs(&dir).unwrap_or_else(|e| {
            tracing::warn!(dir = %dir.display(), error = %one_line(&e), kind = K::DIR, "versions illisibles");
            Vec::new()
        });
        folders
            .iter()
            .filter_map(|folder| {
                self.read_version_of(id, folder)
                    .inspect_err(
                        |e| tracing::warn!(dir = %folder.display(), error = %e, kind = K::DIR, "version ignorée"),
                    )
                    .ok()
            })
            .collect()
    }

    /// La version du dossier `folder`, vérifiée, à condition qu'elle soit de l'entrée `id`.
    fn read_version_of(&self, id: Uuid, folder: &Path) -> std::result::Result<StoredVersion<K::Manifest>, String> {
        self.read_version(folder).and_then(|s| {
            if s.manifest.id() == id { Ok(s) } else { Err(format!("version of pack {}", s.manifest.id())) }
        })
    }

    /// Une version : son dossier s'appelle `<hex>` et son contenu y correspond.
    fn read_version(&self, folder: &Path) -> std::result::Result<StoredVersion<K::Manifest>, String> {
        let name = folder.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let expected = PackVersion::from_hex(name).ok_or_else(|| format!("not a version folder: {name}"))?;
        let pack = self.storage.read(&folder.join(K::MANIFEST_FILE)).map_err(|e| one_line(&e))?;
        let mut widgets = BTreeMap::new();
        let widget_paths = match K::PACK_KIND {
            Some(kind) => self.storage.list_files(&folder.join(kind.files_dir())).map_err(|e| one_line(&e))?,
            None => Vec::new(),
        };
        for path in widget_paths {
            let file = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let Some(name) = WidgetFile::from_file_name(file) else {
                tracing::debug!(path = %path.display(), "fichier étranger aux widgets ignoré");
                continue;
            };
            widgets.insert(name, self.storage.read(&path).map_err(|e| one_line(&e))?);
        }
        // Une sorte sans fichiers de widgets (les thèmes) n'a que son manifeste.
        let files = match K::PACK_KIND {
            Some(kind) => PackFiles::from_bytes(kind, &pack, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice()))),
            None => PackFiles::theme_from_bytes(&pack),
        };
        let files = files.ok_or_else(|| "unreadable pack files".to_owned())?;
        if !files.matches(&expected) {
            return Err(format!("content does not match {expected}"));
        }
        let received = self.is_marked(folder, RECEIVED_FILE);
        StoredVersion::read(files).map(|s| StoredVersion { received, ..s })
    }

    /// Le dossier de version `folder` porte-t-il la marque `mark` (`received`, `adopted`) ?
    ///
    /// Lecture indulgente : une marque illisible compte pour absente.
    fn is_marked(&self, folder: &Path, mark: &str) -> bool {
        let path = folder.join(mark);
        self.marker(&path)
            .inspect_err(
                |e| tracing::warn!(path = %path.display(), error = %one_line(e), "marque de version illisible"),
            )
            .unwrap_or(false)
    }

    /// Le fichier de marque `path` existe-t-il ? Une erreur de lecture est rendue telle quelle.
    pub(crate) fn marker(&self, path: &Path) -> std::result::Result<bool, oku_slides_platform::storage::StorageError> {
        self.storage.len(path).map(|len| len.is_some())
    }

    /// La version `v` de `id` est-elle arrivée avec un deck reçu ?
    pub fn is_received(&self, id: Uuid, v: &PackVersion) -> bool {
        self.pack_dir(id).is_some_and(|d| self.is_marked(&d.join(VERSIONS_DIR).join(v.hex()), RECEIVED_FILE))
    }

    /// Les versions de `id`, chargées : l'intégrée d'abord, puis celles de l'utilisateur.
    pub fn versions(&self, id: Uuid) -> Vec<StoredVersion<K::Manifest>> {
        let listed: Vec<PackVersion> = {
            let mut index = self.fresh();
            self.entry(&mut index, id).versions.iter().map(|s| s.version.clone()).collect()
        };
        listed.iter().filter_map(|v| self.load(id, v)).collect()
    }

    /// Charge la version `v` de `id`, connue de l'index : l'intégrée, ou ses
    /// fichiers relus et vérifiés. Illisible, elle quitte l'index.
    fn load(&self, id: Uuid, v: &PackVersion) -> Option<StoredVersion<K::Manifest>> {
        if let Some(b) = Self::builtin(id).filter(|b| b.version == *v) {
            return Some(b.clone());
        }
        let folder = self.pack_dir(id)?.join(VERSIONS_DIR).join(v.hex());
        match self.read_version_of(id, &folder) {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!(dir = %folder.display(), error = %e, kind = K::DIR, "version ignorée");
                let mut index = self.fresh();
                if let Some(entry) = index.packs.get_mut(&id) {
                    entry.versions.retain(|s| s.version != *v);
                }
                None
            }
        }
    }

    /// Les fichiers de la version `v` du pack `id`.
    pub fn version(&self, id: Uuid, v: &PackVersion) -> Option<PackFiles> {
        let known = {
            let mut index = self.fresh();
            self.entry(&mut index, id).versions.iter().any(|s| s.version == *v)
        };
        known.then(|| self.load(id, v)).flatten().map(|s| s.files)
    }

    /// La version courante du pack `id` ; `None` pour un pack inconnu.
    pub fn current(&self, id: Uuid) -> Option<PackVersion> {
        let mut index = self.fresh();
        choose(self.entry(&mut index, id), Self::builtin(id).map(|b| &b.version), id)
    }

    /// La version courante, lue. `current` absent : l'intégrée, sinon la plus
    /// récente ; `current` vers une version absente : la plus récente.
    pub fn current_version(&self, id: Uuid) -> Option<StoredVersion<K::Manifest>> {
        // Chaque échec de chargement retire une version de l'index : la boucle finit.
        loop {
            let (chosen, count) = {
                let mut index = self.fresh();
                let entry = self.entry(&mut index, id);
                (choose(entry, Self::builtin(id).map(|b| &b.version), id)?, entry.versions.len())
            };
            if let Some(loaded) = self.load(id, &chosen) {
                return Some(loaded);
            }
            let mut index = self.fresh();
            // Un index reconstruit entre-temps qui garde la version illisible : on s'arrête.
            if self.entry(&mut index, id).versions.len() >= count {
                return None;
            }
        }
    }

    fn read_current(&self, id: Uuid) -> Current {
        let Some(path) = self.pack_dir(id).map(|d| d.join(CURRENT_FILE)) else { return Current::Absent };
        let read = match self.storage.len(&path) {
            Ok(None) => return Current::Absent,
            Ok(Some(_)) => self.storage.read(&path),
            Err(e) => Err(e),
        };
        match read {
            Ok(bytes) => String::from_utf8(bytes)
                .ok()
                .and_then(|s| PackVersion::parse(s.trim()))
                .map_or(Current::Broken, Current::At),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %one_line(&e), "version courante illisible");
                Current::Broken
            }
        }
    }

    /// Ajoute une version sans toucher la version courante ; vrai si elle
    /// était nouvelle. Un pack inconnu est créé, cette version courante.
    ///
    /// Les fichiers sont validés d'abord : un manifeste illisible, d'un
    /// autre pack, d'une version du deck seul (`DeckOnly`), sous l'espace de
    /// noms réservé `deck` ou `deck-layouts` (`NamespaceReserved`), sous celui d'un autre pack
    /// (`NamespaceTaken`) ou sous un autre que celui des versions connues du
    /// pack (`NamespaceChanged`) est refusé, rien n'est écrit.
    pub fn add_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        self.add(id, files, Origin::Local)
    }

    /// Comme [`Self::add_version`], pour une version arrivée avec un deck.
    ///
    /// Nouvelle, elle est marquée reçue (fichier `received`, écrit d'abord). Une version du deck
    /// seul est refusée de même, avant toute marque.
    pub fn receive_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        self.add(id, files, Origin::Received)
    }

    fn add(&self, id: Uuid, files: &PackFiles, origin: Origin) -> Result<bool> {
        let dir = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?;
        let manifest = K::Manifest::from_files(files)?;
        ensure!(manifest.id() == id, PackIdMismatchSnafu { id, found: manifest.id() });
        ensure!(manifest.scope().is_library(), DeckOnlySnafu { id });
        if let Some(ns) = manifest.namespace() {
            ensure!(!ns.is_deck() && !ns.is_deck_layouts(), NamespaceReservedSnafu { namespace: ns.as_str() });
            // L'espace de noms d'un pack intégré lui reste, toutes sortes confondues.
            if let Some(owner) = builtin_owner(ns).filter(|owner| *owner != id) {
                return NamespaceTakenSnafu { namespace: ns.as_str(), owner: owner.to_string() }.fail()?;
            }
            // Lu avant de prendre l'index : `owner_of` le prend aussi.
            if let Some(owner) = self.owner_of(ns).filter(|owner| *owner != id) {
                return NamespaceTakenSnafu { namespace: ns.as_str(), owner: owner.to_string() }.fail()?;
            }
        }
        // Une sorte sans widgets les perdrait à l'écriture, mais ils compteraient dans l'empreinte.
        debug_assert!(K::PACK_KIND.is_some() || files.widgets().is_empty(), "widgets given to a kind without widgets");
        let version = files.version();
        let mut index = self.fresh();
        let known = &self.entry(&mut index, id).versions;
        if known.iter().any(|s| s.version == version) {
            return Ok(false);
        }
        // Toutes les versions d'un pack, intégré compris, héritent de son espace de noms.
        if let (Some(expected), Some(found)) =
            (known.first().and_then(|s| s.manifest.namespace()), manifest.namespace())
        {
            ensure!(
                expected == found,
                NamespaceChangedSnafu { id, expected: expected.as_str(), found: found.as_str() }
            );
        }
        let first = known.is_empty();
        let at = dir.join(VERSIONS_DIR).join(version.hex());
        // La marque d'abord : une version complète n'est jamais lue sans elle.
        if origin == Origin::Received {
            let path = at.join(RECEIVED_FILE);
            self.storage.write_atomic(&path, b"").context(WriteSnafu { path: &path })?;
        }
        // Les widgets ensuite : un `widgets.json` présent dit la version complète.
        if let Some(kind) = K::PACK_KIND {
            for (name, source) in files.widgets() {
                let path = at.join(kind.files_dir()).join(name.file_name());
                self.storage.write_atomic(&path, source.as_bytes()).context(WriteSnafu { path: &path })?;
            }
        }
        let path = at.join(K::MANIFEST_FILE);
        let json = files.pack_json().context(PackJsonSnafu { id })?;
        self.storage.write_atomic(&path, json).context(WriteSnafu { path: &path })?;
        let entry = self.entry(&mut index, id);
        entry.versions.push(Indexed { version: version.clone(), manifest, received: origin == Origin::Received });
        if let Some(ids) = index.ids.as_mut().filter(|ids| !ids.contains(&id)) {
            ids.push(id);
        }
        let written = if first { self.write_current(&mut index, id, &version) } else { Ok(()) };
        self.bump(&mut index);
        written.map(|()| true)
    }

    /// Rend la version `v` courante pour l'entrée `id`.
    ///
    /// Elle s'écrit dans `<data>/<K::DIR>/<id>/current` : `widgets/`, `layouts/`
    /// ou `themes/` selon la sorte.
    pub fn set_current(&self, id: Uuid, v: &PackVersion) -> Result<()> {
        let mut index = self.fresh();
        let versions = &self.entry(&mut index, id).versions;
        ensure!(!versions.is_empty(), UnknownPackSnafu { id });
        ensure!(versions.iter().any(|s| s.version == *v), PackVersionMissingSnafu { id, version: v.clone() });
        let written = self.write_current(&mut index, id, v);
        self.bump(&mut index);
        written
    }

    /// Remplace la version `old` par `files`, rendue courante ; rend la nouvelle.
    ///
    /// Précondition : `old` est l'enregistrement précédent de la même session
    /// d'édition des modèles, qui n'a ainsi qu'une version. `old` quitte la
    /// bibliothèque, sauf s'il est figé ou la parente de `files` (la
    /// chaîne des versions reste entière). Un échec de cette suppression est
    /// journalisé : la nouvelle version est déjà enregistrée et courante.
    fn replace_version(&self, id: Uuid, old: &PackVersion, files: &PackFiles) -> Result<PackVersion> {
        let version = files.version();
        self.add_version(id, files)?;
        self.set_current(id, &version)?;
        // Jamais une version figée (intégrée, reçue, adoptée) : même appelée à tort, elle reste.
        let frozen = self.is_frozen(id, old);
        // Validé par `add_version` : le `widgets.json` se lit.
        let parent = K::Manifest::from_files(files).ok().and_then(|m| m.parent().cloned());
        if *old != version && !frozen && parent.as_ref() != Some(old) {
            let dir = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?.join(VERSIONS_DIR).join(old.hex());
            let mut index = self.fresh();
            match self.storage.delete_dir(&dir) {
                Ok(()) => self.entry(&mut index, id).versions.retain(|s| s.version != *old),
                Err(e) => {
                    tracing::warn!(dir = %dir.display(), error = %one_line(&e), "version remplacée non supprimée");
                    // À moitié supprimée peut-être : le pack sera relu.
                    index.packs.remove(&id);
                }
            }
            self.bump(&mut index);
        }
        Ok(version)
    }

    /// Écrit `current` et le reporte dans l'index ; échoué, le pack sera relu.
    fn write_current(&self, index: &mut Index<K::Manifest>, id: Uuid, v: &PackVersion) -> Result<()> {
        let path = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?.join(CURRENT_FILE);
        let written = self.storage.write_atomic(&path, v.as_str().as_bytes()).context(WriteSnafu { path: &path });
        match &written {
            Ok(()) => self.entry(index, id).current = Current::At(v.clone()),
            Err(_) => {
                index.packs.remove(&id);
            }
        }
        written?;
        Ok(())
    }

    /// `a` par rapport à `b` dans le pack `id`, par la chaîne des `parent`.
    pub fn relation(&self, id: Uuid, a: &PackVersion, b: &PackVersion) -> Relation {
        if a == b {
            return Relation::Same;
        }
        let parents: HashMap<PackVersion, Option<PackVersion>> = {
            let mut index = self.fresh();
            let entry = self.entry(&mut index, id);
            entry.versions.iter().map(|s| (s.version.clone(), s.manifest.parent().cloned())).collect()
        };
        if descends(&parents, b, a) {
            Relation::Ancestor
        } else if descends(&parents, a, b) {
            Relation::Descendant
        } else {
            Relation::Sibling
        }
    }

    /// Entrées lues sur le disque par cet index depuis sa création.
    #[cfg(test)]
    pub(crate) fn scans(&self) -> usize {
        self.index.lock().unwrap_or_else(PoisonError::into_inner).scans
    }

    /// L'entrée `id` a-t-elle une version (intégrée ou de l'utilisateur) ?
    pub(crate) fn knows(&self, id: Uuid) -> bool {
        let mut index = self.fresh();
        !self.entry(&mut index, id).versions.is_empty()
    }

    /// L'index de l'entrée `id` liste-t-il la version `v` ? Sans rien charger.
    pub(crate) fn lists(&self, id: Uuid, v: &PackVersion) -> bool {
        let mut index = self.fresh();
        self.entry(&mut index, id).versions.iter().any(|s| s.version == *v)
    }

    /// L'id de l'entrée dont la version courante porte l'espace de noms `ns` (voir [`may_own`]).
    pub(crate) fn owner_of(&self, ns: &Namespace) -> Option<Uuid> {
        self.entries()
            .into_iter()
            .find(|s| s.manifest.namespace() == Some(ns) && may_own(s.manifest.id(), ns))
            .map(|s| s.manifest.id())
    }

    /// Toutes les entrées dans leur version courante : intégrées d'abord, puis
    /// celles de l'utilisateur, par nom.
    pub fn entries(&self) -> Vec<StoredVersion<K::Manifest>> {
        self.scan().0
    }

    /// Les entrées de l'utilisateur sans version lisible, écartées de [`Self::entries`].
    ///
    /// Par exemple un thème d'avant les espaces de noms, que le manifeste actuel ne lit plus.
    pub fn skipped(&self) -> usize {
        self.scan().1
    }

    /// Les entrées dans leur version courante (voir [`Self::entries`]), et le
    /// nombre d'entrées de l'utilisateur écartées faute de version lisible.
    fn scan(&self) -> (Vec<StoredVersion<K::Manifest>>, usize) {
        let user_ids = {
            let mut index = self.fresh();
            index.ids.get_or_insert_with(|| self.user_ids()).clone()
        };
        let mut out: Vec<_> = K::builtins().iter().filter_map(|b| self.current_version(b.manifest.id())).collect();
        let user_ids: Vec<Uuid> = user_ids.into_iter().filter(|id| !Self::is_builtin(*id)).collect();
        let mut user: Vec<_> = user_ids.iter().filter_map(|id| self.current_version(*id)).collect();
        let skipped = user_ids.len() - user.len();
        user.sort_by_key(|s| s.manifest.name().to_lowercase());
        out.extend(user);
        (out, skipped)
    }

    /// Ids des éléments de l'utilisateur : dossiers nommés par un UUID canonique.
    fn user_ids(&self) -> Vec<Uuid> {
        let Some(dir) = &self.dir else { return Vec::new() };
        let folders = self.storage.list_dirs(dir).unwrap_or_else(|e| {
            tracing::warn!(dir = %dir.display(), error = %one_line(&e), kind = K::DIR, "versions illisibles");
            Vec::new()
        });
        folders
            .iter()
            .filter_map(|f| {
                let name = f.file_name()?.to_str()?;
                // Un seul nom par pack : la forme canonique (minuscules, tirets).
                let parsed = Uuid::parse_str(name).ok();
                let id = parsed.filter(|id| id.to_string() == name);
                if id.is_none() && first_legacy_sighting(f) {
                    match parsed {
                        Some(id) => tracing::warn!(dir = %f.display(), canonical = %id, "dossier au nom d'UUID non canonique ignoré"),
                        None => tracing::info!(dir = %f.display(), "ancien format de pack ignoré"),
                    }
                }
                id
            })
            .collect()
    }

    /// Supprime le pack `id` de l'utilisateur, toutes versions comprises.
    ///
    /// Sous l'id d'un pack intégré : retour à la version intégrée (les
    /// versions de l'utilisateur et son `current` disparaissent ; l'intégrée,
    /// en lecture seule, reste et redevient courante).
    pub fn delete(&self, id: Uuid) -> Result<()> {
        let dir = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?;
        let mut index = self.fresh();
        let deleted = self.storage.delete_dir(&dir).context(WriteSnafu { path: dir });
        // Même à moitié supprimé, le pack sera relu.
        index.packs.remove(&id);
        match (&deleted, index.ids.as_mut()) {
            (Ok(()), Some(ids)) => ids.retain(|i| *i != id),
            (Ok(()), None) => {}
            (Err(_), _) => index.ids = None,
        }
        self.bump(&mut index);
        deleted?;
        Ok(())
    }
}

#[path = "library_draft.rs"]
mod draft;
pub(crate) use draft::{DraftLock, DraftWrite, draft_lock};

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;

#[cfg(test)]
#[path = "library_cache_tests.rs"]
mod library_cache_tests;

#[cfg(test)]
#[path = "library_namespace_tests.rs"]
mod library_namespace_tests;
