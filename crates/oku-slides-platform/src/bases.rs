//! The store of data bases: content-addressed Parquet files under `<data dir>/bases/`.
//!
//! A base is untrusted bytes, named `<hex>.parquet` by the SHA-256 of its content and checked
//! against it whenever it enters the store. Links are never followed: files are opened with
//! `O_NOFOLLOW` (a reparse point on Windows) and must be regular files.
//!
//! The store is shared by every tab and process of the app. A base in use is leased: in this
//! process by a reference count, for others by `bases/leases/<random>.json`, rewritten by
//! [`BaseStore::refresh`] every [`LEASE_REFRESH`] and ignored once older than [`LEASE_STALE`].
//! [`BaseStore::gc`] runs under the `bases/.lock` file lock and deletes only files that are
//! neither in its roots nor leased, and older than [`GC_GRACE`].

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use oku_slides_data::BaseId;
use sha2::{Digest, Sha256};
use snafu::{Backtrace, ResultExt, Snafu, ensure};

use crate::storage::{Storage, StorageError};

/// How often a process rewrites its lease file.
pub const LEASE_REFRESH: Duration = Duration::from_mins(5);
/// Age past which a lease file no longer protects its bases.
pub const LEASE_STALE: Duration = Duration::from_mins(30);
/// Age under which an unreferenced base is kept: it may be adopted and not yet committed.
pub const GC_GRACE: Duration = Duration::from_mins(10);

/// A base store failure.
#[derive(Debug, Snafu)]
pub struct BaseStoreError(Box<BaseStoreInner>);

/// The kind of a [`BaseStoreError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BaseStoreErrorKind {
    /// The base is not in the store.
    Missing,
    /// The file is a link or not a regular file.
    NotRegular,
    /// The content does not hash to the expected id.
    HashMismatch,
    /// The content is larger than allowed.
    TooLarge,
    /// The store is over its quota.
    Full,
    /// A file system failure.
    Io,
}

#[derive(Debug, Snafu)]
enum BaseStoreInner {
    #[snafu(display("cannot {action} {}", path.display()))]
    Io { action: &'static str, path: PathBuf, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("store access failed"))]
    Storage {
        #[snafu(source(from(StorageError, Box::new)))]
        source: Box<StorageError>,
        backtrace: Backtrace,
    },
    #[snafu(display("{} is not a regular file", path.display()))]
    NotRegular { path: PathBuf, backtrace: Backtrace },
    #[snafu(display("base {expected} hashes to {actual}"))]
    HashMismatch { expected: String, actual: String, backtrace: Backtrace },
    #[snafu(display("base is larger than {max} bytes"))]
    TooLarge { max: u64, backtrace: Backtrace },
    #[snafu(display("base store is over its quota of {max} bytes"))]
    Full { max: u64, backtrace: Backtrace },
}

impl BaseStoreError {
    /// The kind of the error.
    #[must_use]
    pub fn kind(&self) -> BaseStoreErrorKind {
        match &*self.0 {
            BaseStoreInner::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                BaseStoreErrorKind::Missing
            }
            BaseStoreInner::Io { .. } | BaseStoreInner::Storage { .. } => BaseStoreErrorKind::Io,
            BaseStoreInner::NotRegular { .. } => BaseStoreErrorKind::NotRegular,
            BaseStoreInner::HashMismatch { .. } => BaseStoreErrorKind::HashMismatch,
            BaseStoreInner::TooLarge { .. } => BaseStoreErrorKind::TooLarge,
            BaseStoreInner::Full { .. } => BaseStoreErrorKind::Full,
        }
    }
}

type Result<T, E = BaseStoreError> = std::result::Result<T, E>;

/// The content-addressed store of data bases. Clones share their leases.
#[derive(Debug, Clone)]
pub struct BaseStore {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    storage: Storage,
    dir: PathBuf,
    /// The store's quota, in bytes.
    max_bytes: u64,
    /// This process's lease file.
    lease_file: PathBuf,
    /// Leases of this process, by base id.
    leases: Mutex<BTreeMap<BaseId, usize>>,
}

impl BaseStore {
    /// The store in `<data_dir>/bases`, quota `max_bytes`; creates its folders.
    ///
    /// # Errors
    ///
    /// If the folders cannot be created.
    pub fn new(storage: Storage, data_dir: &Path, max_bytes: u64) -> Result<Self> {
        let dir = data_dir.join("bases");
        storage.create_dir_all(&dir.join("leases")).context(StorageSnafu)?;
        let lease_file = dir.join("leases").join(format!("{}.json", random_name()?));
        let inner = Inner { storage, dir, max_bytes, lease_file, leases: Mutex::new(BTreeMap::new()) };
        Ok(Self { inner: Arc::new(inner) })
    }

    /// The path of base `id` in the store, where the render worker reads it.
    #[must_use]
    pub fn path(&self, id: &BaseId) -> PathBuf {
        self.inner.dir.join(format!("{}.parquet", id.hex()))
    }

    /// The folder holding the bases.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.inner.dir
    }

    /// Copies the regular file `src` into the store while hashing it; the base is leased.
    ///
    /// # Errors
    ///
    /// `NotRegular` for a link or a non-regular file, `TooLarge` past `max`, `Io`.
    pub fn adopt(&self, src: &Path, max: u64) -> Result<BaseId> {
        let file = open_regular(src)?;
        let (tmp, id) = self.copy_in(file, max)?;
        self.install(&tmp, &id)?;
        Ok(id)
    }

    /// Writes base `id` from `r`, at most `max` bytes, checking its hash; the base is leased.
    ///
    /// # Errors
    ///
    /// `TooLarge` past `max`, `HashMismatch` when the content is not `id`, `Io`.
    pub fn put_stream(&self, r: impl Read, id: &BaseId, max: u64) -> Result<()> {
        let (tmp, actual) = self.copy_in(r, max)?;
        if actual != *id {
            self.inner.storage.delete(&tmp).context(StorageSnafu)?;
            return Err(HashMismatchSnafu { expected: id.as_str(), actual: actual.as_str() }.build().into());
        }
        self.install(&tmp, id)
    }

    /// Opens base `id`, a regular file reached without following links.
    ///
    /// # Errors
    ///
    /// `Missing` when the store lacks it, `NotRegular`, `Io`.
    pub fn open(&self, id: &BaseId) -> Result<File> {
        open_regular(&self.path(id))
    }

    /// Whether base `id` is in the store.
    ///
    /// # Errors
    ///
    /// If the store is unreadable.
    pub fn contains(&self, id: &BaseId) -> Result<bool> {
        Ok(self.inner.storage.is_file(&self.path(id)).context(StorageSnafu)?)
    }

    /// Checks that `extra` more bytes keep the store under its quota.
    ///
    /// # Errors
    ///
    /// `Full` past the quota, `Io` if the store is unreadable.
    pub fn check_room(&self, extra: u64) -> Result<()> {
        let mut used = extra;
        for file in self.inner.storage.list_files(&self.inner.dir).context(StorageSnafu)? {
            used = used.saturating_add(self.inner.storage.len(&file).context(StorageSnafu)?.unwrap_or(0));
        }
        ensure!(used <= self.inner.max_bytes, FullSnafu { max: self.inner.max_bytes });
        Ok(())
    }

    /// Leases `ids` for this process, so no collection deletes them.
    ///
    /// # Errors
    ///
    /// If the lease file cannot be written.
    pub fn lease<'a>(&self, ids: impl IntoIterator<Item = &'a BaseId>) -> Result<()> {
        let mut leases = self.inner.leases.lock().unwrap_or_else(PoisonError::into_inner);
        for id in ids {
            *leases.entry(id.clone()).or_insert(0) += 1;
        }
        self.write_leases(&leases)
    }

    /// Releases one lease of each of `ids`.
    ///
    /// # Errors
    ///
    /// If the lease file cannot be written.
    pub fn release<'a>(&self, ids: impl IntoIterator<Item = &'a BaseId>) -> Result<()> {
        let mut leases = self.inner.leases.lock().unwrap_or_else(PoisonError::into_inner);
        for id in ids {
            if let Some(n) = leases.get_mut(id) {
                *n -= 1;
                if *n == 0 {
                    leases.remove(id);
                }
            }
        }
        self.write_leases(&leases)
    }

    /// Rewrites this process's lease file, every [`LEASE_REFRESH`], so it does not go stale.
    ///
    /// # Errors
    ///
    /// If the lease file cannot be written.
    pub fn refresh(&self) -> Result<()> {
        let leases = self.inner.leases.lock().unwrap_or_else(PoisonError::into_inner);
        self.write_leases(&leases)
    }

    /// Rewrites this store's lease file every [`LEASE_REFRESH`] on `runtime`, until the guard drops.
    #[must_use]
    pub fn spawn_refresh(&self, runtime: &tokio::runtime::Handle) -> LeaseRefresh {
        self.spawn_refresh_every(runtime, LEASE_REFRESH)
    }

    /// [`spawn_refresh`](Self::spawn_refresh) on the period `every`.
    fn spawn_refresh_every(&self, runtime: &tokio::runtime::Handle, every: Duration) -> LeaseRefresh {
        let store = self.clone();
        let task = runtime.spawn(async move {
            let mut ticks = tokio::time::interval(every);
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticks.tick().await;
                let store = store.clone();
                // A failed rewrite is retried at the next tick, well before LEASE_STALE.
                match tokio::task::spawn_blocking(move || store.refresh()).await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => tracing::warn!(error = %e, "base lease file not refreshed"),
                    Err(e) => tracing::warn!(error = %e, "base lease refresh task failed"),
                }
            }
        });
        LeaseRefresh(task.abort_handle())
    }

    /// Deletes the files neither in `roots` nor leased and older than [`GC_GRACE`]; returns
    /// their number. Runs under the `bases/.lock` file lock.
    ///
    /// # Errors
    ///
    /// If the store or the lock cannot be used.
    pub fn gc(&self, roots: &BTreeSet<BaseId>) -> Result<usize> {
        self.gc_at(roots, SystemTime::now())
    }

    /// [`gc`](Self::gc) as if it were `now`.
    pub(crate) fn gc_at(&self, roots: &BTreeSet<BaseId>, now: SystemTime) -> Result<usize> {
        let storage = &self.inner.storage;
        let lock_path = self.inner.dir.join(".lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .context(IoSnafu { action: "open", path: &lock_path })?;
        lock.lock().context(IoSnafu { action: "lock", path: &lock_path })?;
        let mut kept: BTreeSet<String> = roots.iter().map(|id| id.hex().to_owned()).collect();
        {
            let leases = self.inner.leases.lock().unwrap_or_else(PoisonError::into_inner);
            kept.extend(leases.keys().map(|id| id.hex().to_owned()));
        }
        for file in storage.list_files(&self.inner.dir.join("leases")).context(StorageSnafu)? {
            if file == self.inner.lease_file {
                continue;
            }
            if older_than(storage, &file, now, LEASE_STALE)? {
                storage.delete(&file).context(StorageSnafu)?;
                continue;
            }
            let bytes = storage.read_limited(&file, 1 << 20).context(StorageSnafu)?;
            // Lease files are written atomically: one that does not parse is damaged, and the
            // bases it protects are unknown, so nothing is collected this round.
            match serde_json::from_slice::<Vec<String>>(&bytes) {
                Ok(ids) => kept.extend(ids),
                Err(e) => {
                    tracing::warn!(file = %file.display(), error = %e, "unreadable base lease, collection skipped");
                    return Ok(0);
                }
            }
        }
        let mut deleted = 0;
        for file in storage.list_files(&self.inner.dir).context(StorageSnafu)? {
            let Some(name) = file.file_name().and_then(|n| n.to_str()) else { continue };
            let stem = name.strip_suffix(".parquet");
            let ours = stem.is_some() || name.starts_with(".tmp-");
            if !ours || stem.is_some_and(|hex| kept.contains(hex)) || !older_than(storage, &file, now, GC_GRACE)? {
                continue;
            }
            storage.delete(&file).context(StorageSnafu)?;
            deleted += 1;
        }
        drop(lock);
        Ok(deleted)
    }

    /// Copies `r` into a new temporary file of the store while hashing it, syncing it to disk.
    fn copy_in(&self, mut r: impl Read, max: u64) -> Result<(PathBuf, BaseId)> {
        let tmp = self.inner.dir.join(format!(".tmp-{}", random_name()?));
        let mut out = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp)
            .context(IoSnafu { action: "create", path: &tmp })?;
        let mut hash = Sha256::new();
        let mut total = 0_u64;
        let mut buf = vec![0_u8; 1 << 16];
        let copied = loop {
            let n = match r.read(&mut buf) {
                Ok(0) => break Ok(()),
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => break Err(e).context(IoSnafu { action: "read", path: &tmp }),
            };
            total += n as u64;
            if total > max {
                break TooLargeSnafu { max }.fail();
            }
            hash.update(&buf[..n]);
            if let Err(e) = out.write_all(&buf[..n]) {
                break Err(e).context(IoSnafu { action: "write", path: &tmp });
            }
        };
        let synced = copied.and_then(|()| out.sync_all().context(IoSnafu { action: "sync", path: &tmp }));
        drop(out);
        if let Err(e) = synced {
            self.inner.storage.delete(&tmp).context(StorageSnafu)?;
            return Err(e.into());
        }
        Ok((tmp, BaseId::from_sha256(&hash.finalize().into())))
    }

    /// Leases `id`, then renames `tmp` to it (the content is the same when it already exists).
    fn install(&self, tmp: &Path, id: &BaseId) -> Result<()> {
        self.lease([id])?;
        Ok(self.inner.storage.rename(tmp, &self.path(id)).context(StorageSnafu)?)
    }

    fn write_leases(&self, leases: &BTreeMap<BaseId, usize>) -> Result<()> {
        // Hex ids need no JSON escaping.
        let ids: Vec<String> = leases.keys().map(|id| format!("\"{}\"", id.hex())).collect();
        let bytes = format!("[{}]", ids.join(",")).into_bytes();
        let storage = &self.inner.storage;
        Ok(off_runtime(|| storage.write_atomic(&self.inner.lease_file, bytes)).context(StorageSnafu)?)
    }
}

/// Runs `f` on a plain thread. Storage blocks on its own runtime, which panics on an async
/// task; lease files are written from drops (a session's, an agent turn's), which may run there.
fn off_runtime<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    std::thread::scope(|scope| scope.spawn(f).join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)))
}

impl Drop for Inner {
    fn drop(&mut self) {
        // The lease file of an ended process: gone now rather than stale in 30 min.
        if let Err(e) = off_runtime(|| self.storage.delete(&self.lease_file)) {
            tracing::warn!(error = %e, "base lease file not deleted");
        }
    }
}

/// Whether `file` was last modified more than `age` before `now`; an unknown time is old.
fn older_than(storage: &Storage, file: &Path, now: SystemTime, age: Duration) -> Result<bool> {
    let modified = storage.modified(file).context(StorageSnafu)?;
    Ok(modified.is_none_or(|t| now.duration_since(t).is_ok_and(|d| d > age)))
}

/// Opens `path` for reading without following a final link, and checks it is a regular file.
fn open_regular(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc::O_NOFOLLOW);
    #[cfg(windows)]
    std::os::windows::fs::OpenOptionsExt::custom_flags(&mut options, FILE_FLAG_OPEN_REPARSE_POINT);
    let file = match options.open(path) {
        Ok(file) => file,
        #[cfg(unix)]
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => return Err(NotRegularSnafu { path }.build().into()),
        Err(e) => Err(e).context(IoSnafu { action: "open", path })?,
    };
    let meta = file.metadata().context(IoSnafu { action: "stat", path })?;
    ensure!(meta.is_file(), NotRegularSnafu { path });
    Ok(file)
}

/// Opens a link itself rather than its target (`winbase.h`).
#[cfg(windows)]
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

/// 32 random hexadecimal characters.
fn random_name() -> Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(std::io::Error::from)
        .context(IoSnafu { action: "draw a name for", path: PathBuf::from("bases") })?;
    Ok(hex::encode(bytes))
}

/// The running lease refresh of a [`BaseStore`]; dropping it stops the refresh.
#[derive(Debug)]
pub struct LeaseRefresh(tokio::task::AbortHandle);

impl Drop for LeaseRefresh {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
#[path = "bases_tests.rs"]
mod bases_tests;
