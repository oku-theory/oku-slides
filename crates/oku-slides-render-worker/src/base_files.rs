//! The worker's side of data bases: paths checked against the one folder the host named, files
//! opened without following a link, every read through `oku_slides_data::engine`.
//!
//! A base is untrusted bytes, parsed only here (t-1690): the host never decodes one.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_data::{BaseId, DataError, Dataset, Table, engine};
use oku_slides_render::{BaseRead, WireError};

/// `path` if it names a file directly inside `dir` (both resolved), else `Error Setup`.
pub(crate) fn check_base_path(dir: Option<&Path>, path: &Path) -> Result<PathBuf, WireError> {
    let refuse = || WireError::Setup { message: format!("base path outside the bases folder: {}", path.display()) };
    let dir = dir.ok_or_else(refuse)?;
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else { return Err(refuse()) };
    let parent = std::fs::canonicalize(parent).map_err(|_| refuse())?;
    if parent != dir || Path::new(name).extension().is_none_or(|e| e != "parquet") {
        return Err(refuse());
    }
    Ok(parent.join(name))
}

/// `out` if it names a file directly inside one of `dirs` (resolved), else `Error Setup`: where a
/// compaction may write.
pub(crate) fn check_out_path(dirs: &[Option<&Path>], out: &Path) -> Result<PathBuf, WireError> {
    let refuse =
        || WireError::Setup { message: format!("compaction output outside the worker folders: {}", out.display()) };
    let (Some(parent), Some(name)) = (out.parent(), out.file_name()) else { return Err(refuse()) };
    let parent = std::fs::canonicalize(parent).map_err(|_| refuse())?;
    let inside = dirs.iter().flatten().any(|dir| std::fs::canonicalize(dir).is_ok_and(|dir| dir == parent));
    if !inside {
        return Err(refuse());
    }
    Ok(parent.join(name))
}

/// Opens base `path` for reading, refusing a symbolic link.
pub(crate) fn open_base(path: &Path) -> Result<File, DataError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path).map_err(|e| DataError::base_invalid(format!("open {}: {e}", path.display())))
}

/// The base files the host sent, by id: what the slots' renderers read.
#[derive(Debug, Clone, Default)]
pub(crate) struct BasePaths(Arc<Mutex<HashMap<BaseId, PathBuf>>>);

impl BasePaths {
    /// Records `path` as the file of base `id`.
    pub(crate) fn insert(&self, id: BaseId, path: PathBuf) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(id, path);
    }

    fn path(&self, id: &BaseId) -> Option<PathBuf> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).get(id).cloned()
    }
}

impl BaseRead for BasePaths {
    fn read_view(
        &self,
        table: &Table,
        columns: &[String],
        rows: Range<usize>,
        max_cell_chars: usize,
    ) -> Result<Dataset, DataError> {
        let base = table.base.as_ref().ok_or_else(|| DataError::base_invalid("no base"))?;
        let path =
            self.path(&base.id).ok_or_else(|| DataError::base_invalid(format!("base {} not sent", base.id.hex())))?;
        let read = engine::read_view(table, open_base(&path)?, columns, rows, max_cell_chars)?;
        if !read.misfit.is_empty() {
            // The data pane reports them (`issue-data-cell-type`); a render shows the text form.
            tracing::info!(columns = read.misfit.len(), "base cells read as text");
        }
        Ok(read.dataset)
    }
}

#[cfg(test)]
#[path = "base_files_tests.rs"]
mod base_files_tests;
