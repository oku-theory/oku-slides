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
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use opendal::{ErrorKind, blocking, services};
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};

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

/// Extensions des fichiers de police lus par [`Storage::read_fonts`].
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc", "otc"];

/// Les fichiers locaux, par OpenDAL. Se clone sans rien recréer.
#[derive(Clone)]
pub struct Storage {
    op: blocking::Operator,
    /// Fait tourner les tâches de l'opérateur ; gardé en vie avec lui.
    _runtime: Arc<tokio::runtime::Runtime>,
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
            .thread_name("echo-storage")
            .enable_all()
            .build()
            .context(RuntimeSnafu)?;
        let op = opendal::Operator::new(services::Fs::default().root("/")).context(SetupSnafu)?;
        let op = {
            let _entered = runtime.enter();
            blocking::Operator::new(op).context(SetupSnafu)?
        };
        Ok(Self { op, _runtime: Arc::new(runtime) })
    }

    /// Clé OpenDAL d'un chemin : absolu, `.` et `..` résolus (sans suivre
    /// les liens), sans la barre de tête.
    fn key(path: &Path) -> Result<String, StorageError> {
        let absolute = std::path::absolute(path).ok().map(|p| normalize(&p));
        let key = absolute.as_deref().and_then(Path::to_str).map(|p| p.trim_start_matches('/').to_owned());
        Ok(key.filter(|k| !k.is_empty()).context(InvalidPathSnafu { path })?)
    }

    fn dir_key(path: &Path) -> Result<String, StorageError> {
        let mut key = Self::key(path)?;
        if !key.ends_with('/') {
            key.push('/');
        }
        Ok(key)
    }

    pub fn read(&self, path: &Path) -> Result<Vec<u8>, StorageError> {
        let buffer = self.op.read(&Self::key(path)?).context(AccessSnafu { action: "read", path })?;
        Ok(buffer.to_vec())
    }

    /// Écrit le fichier (et crée ses dossiers).
    pub fn write(&self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<(), StorageError> {
        self.op.write(&Self::key(path)?, bytes.into()).context(AccessSnafu { action: "write", path })?;
        Ok(())
    }

    /// Écrit dans un fichier temporaire à côté, puis renomme : un lecteur ne
    /// voit jamais de fichier à moitié écrit.
    pub fn write_atomic(&self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<(), StorageError> {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = path.with_file_name(format!(".{name}.tmp"));
        self.write(&tmp, bytes)?;
        self.rename(&tmp, path)
    }

    pub fn rename(&self, from: &Path, to: &Path) -> Result<(), StorageError> {
        Ok(self.op.rename(&Self::key(from)?, &Self::key(to)?).context(AccessSnafu { action: "rename", path: from })?)
    }

    /// Supprime un fichier ; un fichier absent n'est pas une erreur.
    pub fn delete(&self, path: &Path) -> Result<(), StorageError> {
        Ok(self.op.delete(&Self::key(path)?).context(AccessSnafu { action: "delete", path })?)
    }

    pub fn exists(&self, path: &Path) -> Result<bool, StorageError> {
        Ok(self.op.exists(&Self::key(path)?).context(AccessSnafu { action: "stat", path })?)
    }

    /// Taille du fichier, s'il existe.
    pub fn len(&self, path: &Path) -> Result<Option<u64>, StorageError> {
        match self.op.stat(&Self::key(path)?) {
            Ok(meta) => Ok(Some(meta.content_length())),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context(AccessSnafu { action: "stat", path })?,
        }
    }

    pub fn create_dir_all(&self, dir: &Path) -> Result<(), StorageError> {
        Ok(self.op.create_dir(&Self::dir_key(dir)?).context(AccessSnafu { action: "create", path: dir })?)
    }

    /// Fichiers d'un dossier (sans les sous-dossiers) ; un dossier absent est vide.
    pub fn list_files(&self, dir: &Path) -> Result<Vec<PathBuf>, StorageError> {
        let entries = match self.op.list(&Self::dir_key(dir)?) {
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
        let writer = self.op.writer(&Self::key(path)?).context(AccessSnafu { action: "write", path })?;
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
