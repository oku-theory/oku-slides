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
    /// File or folder missing.
    NotFound,
    /// Chemin qui ne peut pas désigner un fichier (non UTF-8…).
    InvalidPath,
    /// Fichier plus gros que la limite de [`Storage::read_limited`].
    TooLarge,
    /// Écriture ou lecture refusée par le système (droits, bac à sable).
    PermissionDenied,
    /// Any other cause: storage startup, streaming write, system error.
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
    #[snafu(display("{} is larger than {max} bytes", path.display()))]
    TooLarge { path: PathBuf, max: u64, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Stream { path: PathBuf, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("cannot resolve {}", path.display()))]
    Canonical { path: PathBuf, source: std::io::Error, backtrace: Backtrace },
}

impl StorageError {
    /// Kind of the error, to react without reading the message.
    #[must_use]
    pub fn kind(&self) -> StorageErrorKind {
        match &*self.0 {
            StorageInner::Access { source, .. } if source.kind() == ErrorKind::NotFound => StorageErrorKind::NotFound,
            StorageInner::Access { source, .. } if source.kind() == ErrorKind::PermissionDenied => {
                StorageErrorKind::PermissionDenied
            }
            StorageInner::Canonical { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                StorageErrorKind::NotFound
            }
            StorageInner::InvalidPath { .. } => StorageErrorKind::InvalidPath,
            StorageInner::TooLarge { .. } => StorageErrorKind::TooLarge,
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

/// Absolute path, `.` and `..` resolved, without touching the disk.
///
/// Links are not followed: two paths to the same file through a
/// link stay different.
#[must_use]
pub fn resolve(path: &Path) -> PathBuf {
    normalize(&std::path::absolute(path).unwrap_or_else(|_| path.to_owned()))
}

/// The real path of `path`: absolute, links followed (`realpath`).
///
/// OpenDAL does not follow links; the render sandbox compares
/// real paths (t-912). The disk is only read.
///
/// # Errors
///
/// If `path` does not exist or one of its folders is unreadable.
pub fn canonical(path: &Path) -> Result<PathBuf, StorageError> {
    Ok(std::fs::canonicalize(path).context(CanonicalSnafu { path })?)
}

/// Extensions des fichiers de police lus par [`Storage::read_fonts`].
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc", "otc"];
/// Début d'un pointeur Git LFS : la police n'a jamais été récupérée.
const LFS_POINTER: &[u8] = b"version https://git-lfs.github.com/spec/v1";

/// Local files, through OpenDAL. Clones without recreating anything.
///
/// One operator per path root (`/` on Unix, `C:\` or `\\server\share\`
/// on Windows), created on first use; OpenDAL keys are
/// the path components under that root, separated by `/`.
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
    /// The local file system.
    ///
    /// # Errors
    ///
    /// If the operators' Tokio runtime does not start.
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

    /// Reads the whole file into memory.
    ///
    /// # Errors
    ///
    /// If the path is unusable or the file missing or unreadable.
    pub fn read(&self, path: &Path) -> Result<Vec<u8>, StorageError> {
        let (op, key) = self.at(path)?;
        let buffer = op.read(&key).context(AccessSnafu { action: "read", path })?;
        Ok(buffer.to_vec())
    }

    /// Reads the file if it is at most `max` bytes; beyond that, [`StorageErrorKind::TooLarge`].
    ///
    /// Never more than `max` bytes in memory: only the size seen at the start
    /// is read, even if the file grows meanwhile.
    ///
    /// # Errors
    ///
    /// If the path is unusable, the file missing, unreadable or larger than `max`.
    pub fn read_limited(&self, path: &Path, max: u64) -> Result<Vec<u8>, StorageError> {
        let (op, key) = self.at(path)?;
        let len = op.stat(&key).context(AccessSnafu { action: "read", path })?.content_length();
        ensure!(len <= max, TooLargeSnafu { path, max });
        let opts = opendal::options::ReadOptions { range: (0..len).into(), ..Default::default() };
        let buffer = op.read_options(&key, opts).context(AccessSnafu { action: "read", path })?;
        Ok(buffer.to_vec())
    }

    /// The first `n` bytes of the file, fewer if it is shorter.
    ///
    /// # Errors
    ///
    /// If the path is unusable or the file missing or unreadable.
    pub fn read_head(&self, path: &Path, n: u64) -> Result<Vec<u8>, StorageError> {
        let (op, key) = self.at(path)?;
        let len = op.stat(&key).context(AccessSnafu { action: "read", path })?.content_length().min(n);
        if len == 0 {
            return Ok(Vec::new());
        }
        let opts = opendal::options::ReadOptions { range: (0..len).into(), ..Default::default() };
        let buffer = op.read_options(&key, opts).context(AccessSnafu { action: "read", path })?;
        Ok(buffer.to_vec())
    }

    /// Writes the file (and creates its folders).
    ///
    /// # Errors
    ///
    /// If the path is unusable or the write refused.
    pub fn write(&self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<(), StorageError> {
        let (op, key) = self.at(path)?;
        op.write(&key, bytes.into()).context(AccessSnafu { action: "write", path })?;
        Ok(())
    }

    /// Writes to a temporary file next to it, then renames.
    ///
    /// A reader never sees a half-written file.
    ///
    /// The temporary name is unique to each write: two simultaneous
    /// writes of the same file do not step on each other.
    ///
    /// # Errors
    ///
    /// If writing the temporary file or the rename fails.
    pub fn write_atomic(&self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<(), StorageError> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
        self.write(&tmp, bytes)?;
        self.rename(&tmp, path)
    }

    /// Renames a file, without changing volume.
    ///
    /// # Errors
    ///
    /// If a path is unusable, if both are on different volumes
    /// or if the rename fails.
    pub fn rename(&self, from: &Path, to: &Path) -> Result<(), StorageError> {
        let (op, from_key) = self.at(from)?;
        let (to_op, to_key) = self.at(to)?;
        // Un renommage ne change pas de volume : deux racines, deux volumes.
        ensure!(op.info().root() == to_op.info().root(), InvalidPathSnafu { path: to });
        Ok(op.rename(&from_key, &to_key).context(AccessSnafu { action: "rename", path: from })?)
    }

    /// Deletes a file; a missing file is not an error.
    ///
    /// # Errors
    ///
    /// If the path is unusable or the deletion refused.
    pub fn delete(&self, path: &Path) -> Result<(), StorageError> {
        let (op, key) = self.at(path)?;
        Ok(op.delete(&key).context(AccessSnafu { action: "delete", path })?)
    }

    /// Deletes a folder and all its contents; a missing folder is not an error.
    ///
    /// Symbolic links are never followed: a link (in place of the
    /// folder or inside it) is removed, its target stays. OpenDAL strips the trailing
    /// `/` and the standard library's `remove_dir_all` does not go through
    /// links; the tests (`storage_tests`, regression t-163) guard it.
    ///
    /// # Errors
    ///
    /// If the path is unusable or the deletion refused.
    pub fn delete_dir(&self, dir: &Path) -> Result<(), StorageError> {
        let (op, key) = self.dir_at(dir)?;
        let opts = opendal::options::DeleteOptions { recursive: true, ..Default::default() };
        Ok(op.delete_options(&key, opts).context(AccessSnafu { action: "delete", path: dir })?)
    }

    /// True if a file or a folder exists at `path`.
    ///
    /// # Errors
    ///
    /// If the path is unusable or unreadable.
    pub fn exists(&self, path: &Path) -> Result<bool, StorageError> {
        let (op, key) = self.at(path)?;
        Ok(op.exists(&key).context(AccessSnafu { action: "stat", path })?)
    }

    /// True if `path` is a file.
    ///
    /// False if it is missing or is a folder.
    ///
    /// # Errors
    ///
    /// If the path is unusable or unreadable.
    pub fn is_file(&self, path: &Path) -> Result<bool, StorageError> {
        let (op, key) = self.at(path)?;
        match op.stat(&key) {
            Ok(meta) => Ok(meta.mode().is_file()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e).context(AccessSnafu { action: "stat", path })?,
        }
    }

    /// Size of the file, if it exists.
    ///
    /// # Errors
    ///
    /// If the path is unusable or unreadable.
    pub fn len(&self, path: &Path) -> Result<Option<u64>, StorageError> {
        let (op, key) = self.at(path)?;
        match op.stat(&key) {
            Ok(meta) => Ok(Some(meta.content_length())),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context(AccessSnafu { action: "stat", path })?,
        }
    }

    /// Creates the folder and its parents; an existing folder is not an error.
    ///
    /// # Errors
    ///
    /// If the path is unusable or the creation refused.
    pub fn create_dir_all(&self, dir: &Path) -> Result<(), StorageError> {
        let (op, key) = self.dir_at(dir)?;
        Ok(op.create_dir(&key).context(AccessSnafu { action: "create", path: dir })?)
    }

    /// Files of a folder (without subfolders); a missing folder is empty.
    ///
    /// # Errors
    ///
    /// If the path is unusable or the folder unreadable.
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

    /// Subfolders of a folder; a missing folder is empty.
    ///
    /// # Errors
    ///
    /// If the path is unusable or the folder unreadable.
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

    /// Font files of a folder (TTF, OTF, TTC, OTC), by file name.
    ///
    /// A Git LFS pointer (a font never fetched) is skipped and reported
    /// by an error in the log, with the remedy.
    ///
    /// # Errors
    ///
    /// If the folder or a font is unreadable.
    pub fn read_fonts(&self, dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, StorageError> {
        let mut fonts = BTreeMap::new();
        for path in self.list_files(dir)? {
            let is_font = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| FONT_EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)));
            let Some(name) = path.file_name().filter(|_| is_font) else { continue };
            let bytes = self.read(&path)?;
            // Clone sans Git LFS : un pointeur texte à la place de la police.
            if bytes.starts_with(LFS_POINTER) {
                tracing::error!(
                    path = %path.display(),
                    "police absente (pointeur Git LFS) : git lfs install && git lfs pull"
                );
                continue;
            }
            fonts.insert(name.to_string_lossy().into_owned(), bytes);
        }
        Ok(fonts)
    }

    /// Streaming write (large files); [`StreamWriter::close`] finishes it.
    ///
    /// # Errors
    ///
    /// If the path is unusable or opening for writing is refused.
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
    /// Appends `bytes` to the file.
    ///
    /// # Errors
    ///
    /// If the write fails.
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        Ok(self.inner.write_all(bytes).context(StreamSnafu { path: &self.path })?)
    }

    /// Finishes the write: the file is complete on disk.
    ///
    /// # Errors
    ///
    /// If finishing the write fails: the file may stay incomplete.
    pub fn close(mut self) -> Result<(), StorageError> {
        Ok(self.inner.close().context(StreamSnafu { path: &self.path })?)
    }
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod storage_tests;
