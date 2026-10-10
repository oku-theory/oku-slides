//! Bibliothèque versionnée, générique sur la sorte ([`LibraryKind`]) : packs de modèles et thèmes.
//!
//! Une entrée est un id et des versions adressées par contenu ; chaque version
//! connaît sa parente (`parent` de son manifeste). Sur disque, sous le dossier
//! de la sorte :
//!
//! ```text
//! <data>/templates/<uuid>/current                     version courante
//! <data>/templates/<uuid>/versions/<hex>/pack.json
//! <data>/templates/<uuid>/versions/<hex>/widgets/<nom>.typ  (un par fichier de widget)
//!
//! <data>/themes/<uuid>/current
//! <data>/themes/<uuid>/versions/<hex>/theme.json            (jamais de widgets)
//! ```
//!
//! Entrées intégrées (`templates/essentiel`, `templates/grilles`,
//! `typst/oku-slides/themes/*` du dépôt) : id constant, version de l'app en
//! lecture seule, courante tant que l'utilisateur n'a pas de fichier
//! `current`. La version courante ne change que par [`Library::set_current`],
//! ou à la création d'une entrée inconnue.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion, WidgetFile};
use oku_slides_platform::errors::one_line;
use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::error::{
    NoLibraryDirSnafu, PackIdMismatchSnafu, PackJsonSnafu, PackVersionMissingSnafu, UnknownPackSnafu, WriteSnafu,
};
use crate::library_kind::{LibraryKind, StoredVersion, Templates, VersionManifest};
use crate::pack::{PackInfo, PackManifest, Relation};

/// Version courante d'un pack, dans son dossier.
const CURRENT_FILE: &str = "current";
/// Versions d'un pack, dans son dossier.
const VERSIONS_DIR: &str = "versions";
const WIDGETS_DIR: &str = "widgets";

/// Versions des packs intégrés, dans l'ordre.
pub fn builtin_versions() -> &'static [StoredVersion<PackManifest>] {
    Templates::builtins()
}

/// La plus récente par `created` (la dernière vue à égalité).
fn latest<M: VersionManifest>(versions: &[StoredVersion<M>]) -> Option<PackVersion> {
    versions.iter().max_by(|a, b| a.manifest.created().cmp(b.manifest.created())).map(|s| s.version.clone())
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
    LEGACY_SEEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(dir.to_path_buf())
}

/// Le fichier `current` d'un pack.
enum Current {
    Absent,
    At(PackVersion),
    /// Illisible ou mal formé.
    Broken,
}

/// Bibliothèque : versions intégrées et versions de l'utilisateur, d'une sorte.
#[derive(Debug, Clone)]
pub struct Library<K: LibraryKind> {
    dir: Option<PathBuf>,
    storage: Storage,
    kind: PhantomData<K>,
}

/// La bibliothèque des packs de modèles.
pub type PackLibrary = Library<Templates>;

impl<K: LibraryKind> Library<K> {
    /// `dir` : `<data>/<K::DIR>`, créé à la première écriture.
    pub fn new(dir: Option<PathBuf>, storage: Storage) -> Self {
        Self { dir, storage, kind: PhantomData }
    }

    /// Une entrée intégrée à l'app ?
    pub fn is_builtin(id: Uuid) -> bool {
        Self::builtin(id).is_some()
    }

    fn builtin(id: Uuid) -> Option<&'static StoredVersion<K::Manifest>> {
        K::builtins().iter().find(|b| b.manifest.id() == id)
    }

    /// Dossier du pack `id` ; `None` sans dossier des modèles.
    fn pack_dir(&self, id: Uuid) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(id.to_string()))
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
                let read = self.read_version(folder).and_then(|s| {
                    if s.manifest.id() == id { Ok(s) } else { Err(format!("version of pack {}", s.manifest.id())) }
                });
                read.inspect_err(
                    |e| tracing::warn!(dir = %folder.display(), error = %e, kind = K::DIR, "version ignorée"),
                )
                .ok()
            })
            .collect()
    }

    /// Une version : son dossier s'appelle `<hex>` et son contenu y correspond.
    fn read_version(&self, folder: &Path) -> std::result::Result<StoredVersion<K::Manifest>, String> {
        let name = folder.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let expected = PackVersion::from_hex(name).ok_or_else(|| format!("not a version folder: {name}"))?;
        let pack = self.storage.read(&folder.join(K::MANIFEST_FILE)).map_err(|e| one_line(&e))?;
        let mut widgets = BTreeMap::new();
        let widget_paths = if K::HAS_WIDGETS {
            self.storage.list_files(&folder.join(WIDGETS_DIR)).map_err(|e| one_line(&e))?
        } else {
            Vec::new()
        };
        for path in widget_paths {
            let file = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let Some(name) = WidgetFile::from_file_name(file) else {
                tracing::debug!(path = %path.display(), "fichier étranger aux widgets ignoré");
                continue;
            };
            widgets.insert(name, self.storage.read(&path).map_err(|e| one_line(&e))?);
        }
        let files = PackFiles::from_bytes(&pack, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice())))
            .ok_or_else(|| "unreadable pack files".to_owned())?;
        if !files.matches(&expected) {
            return Err(format!("content does not match {expected}"));
        }
        StoredVersion::read(files)
    }

    /// Les versions de `id` : l'intégrée d'abord, puis celles de l'utilisateur.
    pub fn versions(&self, id: Uuid) -> Vec<StoredVersion<K::Manifest>> {
        let mut out: Vec<StoredVersion<K::Manifest>> = Self::builtin(id).into_iter().cloned().collect();
        for v in self.user_versions(id) {
            if !out.iter().any(|o| o.version == v.version) {
                out.push(v);
            }
        }
        out
    }

    /// Les fichiers de la version `v` du pack `id`.
    pub fn version(&self, id: Uuid, v: &PackVersion) -> Option<PackFiles> {
        self.versions(id).into_iter().find(|s| s.version == *v).map(|s| s.files)
    }

    /// La version courante du pack `id` ; `None` pour un pack inconnu.
    pub fn current(&self, id: Uuid) -> Option<PackVersion> {
        self.current_version(id).map(|s| s.version)
    }

    /// La version courante, lue. `current` absent : l'intégrée, sinon la plus
    /// récente ; `current` vers une version absente : la plus récente.
    pub fn current_version(&self, id: Uuid) -> Option<StoredVersion<K::Manifest>> {
        let versions = self.versions(id);
        let chosen = match self.read_current(id) {
            Current::At(v) if versions.iter().any(|s| s.version == v) => Some(v),
            Current::Absent => Self::builtin(id).map(|b| b.version.clone()).or_else(|| latest(&versions)),
            Current::At(_) | Current::Broken => {
                tracing::warn!(pack = %id, "version courante absente : la plus récente la remplace");
                latest(&versions)
            }
        }?;
        versions.into_iter().find(|s| s.version == chosen)
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
    /// Les fichiers sont validés d'abord : un `pack.json` illisible ou d'un
    /// autre pack est refusé, rien n'est écrit.
    pub fn add_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        let dir = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?;
        let manifest = K::Manifest::from_files(files).context(PackJsonSnafu { id })?;
        ensure!(manifest.id() == id, PackIdMismatchSnafu { id, found: manifest.id() });
        // Une sorte sans widgets les perdrait à l'écriture, mais ils compteraient dans l'empreinte.
        debug_assert!(K::HAS_WIDGETS || files.widgets().is_empty(), "widgets given to a kind without widgets");
        let version = files.version();
        let known = self.versions(id);
        if known.iter().any(|s| s.version == version) {
            return Ok(false);
        }
        let at = dir.join(VERSIONS_DIR).join(version.hex());
        // Les widgets d'abord : un `pack.json` présent dit la version complète.
        if K::HAS_WIDGETS {
            for (name, source) in files.widgets() {
                let path = at.join(WIDGETS_DIR).join(name.file_name());
                self.storage.write_atomic(&path, source.as_bytes()).context(WriteSnafu { path: &path })?;
            }
        }
        let path = at.join(K::MANIFEST_FILE);
        let json = files.pack_json().context(PackJsonSnafu { id })?;
        self.storage.write_atomic(&path, json).context(WriteSnafu { path: &path })?;
        if known.is_empty() {
            self.write_current(id, &version)?;
        }
        Ok(true)
    }

    /// Rend la version `v` courante pour l'entrée `id`.
    ///
    /// Elle s'écrit dans `<data>/<K::DIR>/<id>/current` : `templates/` pour
    /// les packs de modèles, `themes/` pour les thèmes.
    pub fn set_current(&self, id: Uuid, v: &PackVersion) -> Result<()> {
        let versions = self.versions(id);
        ensure!(!versions.is_empty(), UnknownPackSnafu { id });
        ensure!(versions.iter().any(|s| s.version == *v), PackVersionMissingSnafu { id, version: v.clone() });
        self.write_current(id, v)
    }

    /// Remplace la version `old` par `files`, rendue courante ; rend la nouvelle.
    ///
    /// Précondition : `old` est l'enregistrement précédent de la même session
    /// d'édition des modèles, qui n'a ainsi qu'une version. `old` quitte la
    /// bibliothèque, sauf s'il est intégré ou la parente de `files` (la
    /// chaîne des versions reste entière). Un échec de cette suppression est
    /// journalisé : la nouvelle version est déjà enregistrée et courante.
    pub fn replace_version(&self, id: Uuid, old: &PackVersion, files: &PackFiles) -> Result<PackVersion> {
        let version = files.version();
        self.add_version(id, files)?;
        self.set_current(id, &version)?;
        let builtin = Self::builtin(id).is_some_and(|b| b.version == *old);
        // Validé par `add_version` : le `pack.json` se lit.
        let parent = K::Manifest::from_files(files).ok().and_then(|m| m.parent().cloned());
        if *old != version && !builtin && parent.as_ref() != Some(old) {
            let dir = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?.join(VERSIONS_DIR).join(old.hex());
            if let Err(e) = self.storage.delete_dir(&dir) {
                tracing::warn!(dir = %dir.display(), error = %one_line(&e), "version remplacée non supprimée");
            }
        }
        Ok(version)
    }

    fn write_current(&self, id: Uuid, v: &PackVersion) -> Result<()> {
        let path = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?.join(CURRENT_FILE);
        self.storage.write_atomic(&path, v.as_str().as_bytes()).context(WriteSnafu { path: &path })?;
        Ok(())
    }

    /// `a` par rapport à `b` dans le pack `id`, par la chaîne des `parent`.
    pub fn relation(&self, id: Uuid, a: &PackVersion, b: &PackVersion) -> Relation {
        if a == b {
            return Relation::Same;
        }
        let parents: HashMap<PackVersion, Option<PackVersion>> =
            self.versions(id).into_iter().map(|s| (s.version, s.manifest.parent().cloned())).collect();
        if descends(&parents, b, a) {
            Relation::Ancestor
        } else if descends(&parents, a, b) {
            Relation::Descendant
        } else {
            Relation::Sibling
        }
    }

    /// Toutes les entrées dans leur version courante : intégrées d'abord, puis
    /// celles de l'utilisateur, par nom.
    pub fn entries(&self) -> Vec<StoredVersion<K::Manifest>> {
        let mut out: Vec<_> = K::builtins().iter().filter_map(|b| self.current_version(b.manifest.id())).collect();
        let mut user: Vec<_> = self
            .user_ids()
            .into_iter()
            .filter(|id| !Self::is_builtin(*id))
            .filter_map(|id| self.current_version(id))
            .collect();
        user.sort_by_key(|s| s.manifest.name().to_lowercase());
        out.extend(user);
        out
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
                let id = Uuid::parse_str(name).ok().filter(|id| id.to_string() == name);
                if id.is_none() && first_legacy_sighting(f) {
                    tracing::info!(dir = %f.display(), "ancien format de pack ignoré");
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
        self.storage.delete_dir(&dir).context(WriteSnafu { path: dir })?;
        Ok(())
    }
}

impl PackLibrary {
    /// Tous les packs dans leur version courante : intégrés d'abord, puis
    /// ceux de l'utilisateur, par nom.
    pub fn packs(&self) -> Vec<PackInfo> {
        self.entries()
            .into_iter()
            .map(|s| PackInfo::of(&s.manifest, s.version, Self::is_builtin(s.manifest.id)))
            .collect()
    }
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;
