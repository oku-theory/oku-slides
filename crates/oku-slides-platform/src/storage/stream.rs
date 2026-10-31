//! Streaming reads and writes of large files ([`StreamReader`], [`StreamWriter`]).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use opendal::blocking;
use snafu::ResultExt;

use super::{StorageError, StreamSnafu};

/// Streaming write of a file; without [`close`](Self::close) it may stay incomplete.
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
    pub(super) fn new(inner: blocking::StdWriter, path: &Path) -> Self {
        Self { inner, path: path.to_path_buf() }
    }

    /// Appends `bytes` to the file.
    ///
    /// # Errors
    ///
    /// If the write fails.
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        Ok(self.inner.write_all(bytes).context(StreamSnafu { action: "write", path: &self.path })?)
    }

    /// Finishes the write: the file is complete on disk.
    ///
    /// # Errors
    ///
    /// If finishing the write fails: the file may stay incomplete.
    pub fn close(mut self) -> Result<(), StorageError> {
        Ok(self.inner.close().context(StreamSnafu { action: "write", path: &self.path })?)
    }
}

/// Streaming read of a file, a bounded chunk at a time.
pub struct StreamReader {
    inner: blocking::StdReader,
    path: PathBuf,
}

impl std::fmt::Debug for StreamReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamReader").field("path", &self.path).finish_non_exhaustive()
    }
}

impl StreamReader {
    pub(super) fn new(inner: blocking::StdReader, path: &Path) -> Self {
        Self { inner, path: path.to_path_buf() }
    }

    /// Fills `buf` from the file; `0` at the end.
    ///
    /// # Errors
    ///
    /// If the read fails.
    pub fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, StorageError> {
        Ok(self.inner.read(buf).context(StreamSnafu { action: "read", path: &self.path })?)
    }
}
