//! Fichiers de l'app, par OpenDAL : lire, écrire (de façon atomique),
//! lister, renommer, supprimer, écrire en flux.
//!
//! L'API est synchrone : l'app et la CLI n'ont pas de code asynchrone. Un
//! [`Storage`] garde un petit runtime Tokio à lui, sur lequel l'opérateur
//! bloquant d'OpenDAL fait tourner ses tâches. À n'appeler que hors d'un
//! contexte asynchrone (commandes Tauri synchrones, fils de la dictée, CLI).
//!
//! Les chemins sont ceux du système ; un chemin relatif part du dossier
//! courant.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use opendal::{ErrorKind, blocking, services};
use snafu::{Backtrace, OptionExt, ResultExt, Snafu, ensure};

/// Accès aux fichiers impossible.
#[derive(Debug, Snafu)]
pub struct StorageError(Box<StorageInner>);

/// Nature d'une [`StorageError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StorageErrorKind {
    NotFound,
    /// Chemin qui ne peut pas désigner un fichier (non UTF-8…).
    InvalidPath,
    Other,
}

#[derive(Debug, Snafu)]
enum StorageInner {
    #[snafu(display("storage could not start"))]
    Runtime { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("storage could not start"))]
    Setup { source: opendal::Error, backtrace: Backtrace },
    #[snafu(display("unusable path {}", path.display()))]
    InvalidPath { path: PathBuf, backtrace: Backtrace },
    #[snafu(display("cannot {action} {}", path.display()))]
    Access { action: &'static str, path: PathBuf, source: opendal::Error, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Stream { path: PathBuf, source: std::io::Error, backtrace: Backtrace },
}

impl StorageError {
    pub fn kind(&self) -> StorageErrorKind {
        match &*self.0 {
            StorageInner::Access { source, .. } if source.kind() == ErrorKind::NotFound => StorageErrorKind::NotFound,
            StorageInner::InvalidPath { .. } => StorageErrorKind::InvalidPath,
            _ => StorageErrorKind::Other,
        }
    }
}

/// Résout `.` et `..` sans toucher au disque.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Chemin absolu, `.` et `..` résolus, sans toucher au disque.
///
/// Les liens ne sont pas suivis : deux chemins vers un même fichier par un
/// lien restent différents.
pub fn resolve(path: &Path) -> PathBuf {
    normalize(&std::path::absolute(path).unwrap_or_else(|_| path.to_owned()))
}

/// Extensions des fichiers de police lus par [`Storage::read_fonts`].
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc", "otc"];

/// Les fichiers locaux, par OpenDAL. Se clone sans rien recréer.
///
/// Un opérateur par racine de chemin (`/` sous Unix, `C:\` ou `\\serveur\partage\`
/// sous Windows), créé à la première utilisation ; les clés OpenDAL sont
/// les composants du chemin sous cette racine, séparés par `/`.
#[derive(Clone)]
pub struct Storage {
    roots: Arc<Mutex<HashMap<PathBuf, blocking::Operator>>>,
    /// Fait tourner les tâches des opérateurs ; gardé en vie avec eux.
    runtime: Arc<tokio::runtime::Runtime>,
}

/// Un chemin, vu d'OpenDAL : sa racine et sa clé sous cette racine.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Located {
    root: PathBuf,
    key: String,
}

/// Racine et clé d'un chemin : absolu, `.` et `..` résolus sans toucher au
/// disque (les liens ne sont pas suivis).
fn locate(path: &Path) -> Option<Located> {
    let absolute = normalize(&std::path::absolute(path).ok()?);
    let mut root = PathBuf::new();
    let mut parts = Vec::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => root.push(component),
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir | Component::ParentDir => {}
        }
    }
    let key = parts.join("/");
    (!key.is_empty() && !root.as_os_str().is_empty()).then_some(Located { root, key })
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Storage").finish_non_exhaustive()
    }
}

impl Storage {
    /// Le système de fichiers local.
    pub fn local() -> Result<Self, StorageError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("oku-slides-storage")
            .enable_all()
            .build()
            .context(RuntimeSnafu)?;
        Ok(Self { roots: Arc::new(Mutex::new(HashMap::new())), runtime: Arc::new(runtime) })
    }

    /// L'opérateur de la racine de `path`, et la clé du chemin sous elle.
    fn at(&self, path: &Path) -> Result<(blocking::Operator, String), StorageError> {
        let Located { root, key } = locate(path).context(InvalidPathSnafu { path })?;
        let mut roots = self.roots.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(op) = roots.get(&root) {
            return Ok((op.clone(), key));
        }
        let root_str = root.to_str().context(InvalidPathSnafu { path })?;
        let op = opendal::Operator::new(services::Fs::default().root(root_str)).context(SetupSnafu)?;
        let op = {
            let _entered = self.runtime.enter();
            blocking::Operator::new(op).context(SetupSnafu)?
        };
        roots.insert(root, op.clone());
        Ok((op, key))
    }

    /// Comme [`at`](Self::at), pour un dossier (clé terminée par `/`).
    fn dir_at(&self, path: &Path) -> Result<(blocking::Operator, String), StorageError> {
        let (op, mut key) = self.at(path)?;
        key.push('/');
        Ok((op, key))
    }

    pub fn read(&self, path: &Path) -> Result<Vec<u8>, StorageError> {
        let (op, key) = self.at(path)?;
        let buffer = op.read(&key).context(AccessSnafu { action: "read", path })?;
        Ok(buffer.to_vec())
    }

    /// Écrit le fichier (et crée ses dossiers).
    pub fn write(&self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<(), StorageError> {
        let (op, key) = self.at(path)?;
        op.write(&key, bytes.into()).context(AccessSnafu { action: "write", path })?;
        Ok(())
    }

    /// Écrit dans un fichier temporaire à côté, puis renomme : un lecteur ne
    /// voit jamais de fichier à moitié écrit.
    ///
    /// Le nom temporaire est propre à chaque écriture : deux écritures
    /// simultanées du même fichier ne se marchent pas dessus.
    pub fn write_atomic(&self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<(), StorageError> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
        self.write(&tmp, bytes)?;
        self.rename(&tmp, path)
    }

    pub fn rename(&self, from: &Path, to: &Path) -> Result<(), StorageError> {
        let (op, from_key) = self.at(from)?;
        let (to_op, to_key) = self.at(to)?;
        // Un renommage ne change pas de volume : deux racines, deux volumes.
        ensure!(op.info().root() == to_op.info().root(), InvalidPathSnafu { path: to });
        Ok(op.rename(&from_key, &to_key).context(AccessSnafu { action: "rename", path: from })?)
    }

    /// Supprime un fichier ; un fichier absent n'est pas une erreur.
    pub fn delete(&self, path: &Path) -> Result<(), StorageError> {
        let (op, key) = self.at(path)?;
        Ok(op.delete(&key).context(AccessSnafu { action: "delete", path })?)
    }

    /// Supprime un dossier et tout son contenu ; un dossier absent n'est pas une erreur.
    pub fn delete_dir(&self, dir: &Path) -> Result<(), StorageError> {
        let (op, key) = self.dir_at(dir)?;
        let opts = opendal::options::DeleteOptions { recursive: true, ..Default::default() };
        Ok(op.delete_options(&key, opts).context(AccessSnafu { action: "delete", path: dir })?)
    }

    pub fn exists(&self, path: &Path) -> Result<bool, StorageError> {
        let (op, key) = self.at(path)?;
        Ok(op.exists(&key).context(AccessSnafu { action: "stat", path })?)
    }

    /// Taille du fichier, s'il existe.
    pub fn len(&self, path: &Path) -> Result<Option<u64>, StorageError> {
        let (op, key) = self.at(path)?;
        match op.stat(&key) {
            Ok(meta) => Ok(Some(meta.content_length())),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context(AccessSnafu { action: "stat", path })?,
        }
    }

    pub fn create_dir_all(&self, dir: &Path) -> Result<(), StorageError> {
        let (op, key) = self.dir_at(dir)?;
        Ok(op.create_dir(&key).context(AccessSnafu { action: "create", path: dir })?)
    }

    /// Fichiers d'un dossier (sans les sous-dossiers) ; un dossier absent est vide.
    pub fn list_files(&self, dir: &Path) -> Result<Vec<PathBuf>, StorageError> {
        let (op, key) = self.dir_at(dir)?;
        let entries = match op.list(&key) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).context(AccessSnafu { action: "list", path: dir })?,
        };
        Ok(entries
            .into_iter()
            .filter(|e| e.metadata().is_file())
            .map(|e| dir.join(e.name().trim_end_matches('/')))
            .collect())
    }

    /// Sous-dossiers d'un dossier ; un dossier absent est vide.
    pub fn list_dirs(&self, dir: &Path) -> Result<Vec<PathBuf>, StorageError> {
        let (op, key) = self.dir_at(dir)?;
        let entries = match op.list(&key) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).context(AccessSnafu { action: "list", path: dir })?,
        };
        Ok(entries
            .into_iter()
            // La liste d'un dossier peut contenir le dossier lui-même.
            .filter(|e| e.metadata().is_dir() && e.path() != key)
            .map(|e| dir.join(e.name().trim_end_matches('/')))
            .collect())
    }

    /// Fichiers de police d'un dossier (TTF, OTF, TTC, OTC), par nom de fichier.
    pub fn read_fonts(&self, dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, StorageError> {
        let mut fonts = BTreeMap::new();
        for path in self.list_files(dir)? {
            let is_font = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| FONT_EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)));
            if let (true, Some(name)) = (is_font, path.file_name()) {
                fonts.insert(name.to_string_lossy().into_owned(), self.read(&path)?);
            }
        }
        Ok(fonts)
    }

    /// Écriture en flux (gros fichiers) ; [`StreamWriter::close`] la termine.
    pub fn writer(&self, path: &Path) -> Result<StreamWriter, StorageError> {
        let (op, key) = self.at(path)?;
        let writer = op.writer(&key).context(AccessSnafu { action: "write", path })?;
        Ok(StreamWriter { inner: writer.into_std_write(), path: path.to_path_buf() })
    }
}

/// Écriture en flux d'un fichier. Sans [`close`](Self::close), le fichier
/// peut rester incomplet.
pub struct StreamWriter {
    inner: blocking::StdWriter,
    path: PathBuf,
}

impl std::fmt::Debug for StreamWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamWriter").field("path", &self.path).finish_non_exhaustive()
    }
}

impl StreamWriter {
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        Ok(self.inner.write_all(bytes).context(StreamSnafu { path: &self.path })?)
    }

    /// Termine l'écriture : le fichier est complet sur le disque.
    pub fn close(mut self) -> Result<(), StorageError> {
        Ok(self.inner.close().context(StreamSnafu { path: &self.path })?)
    }
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod storage_tests;
