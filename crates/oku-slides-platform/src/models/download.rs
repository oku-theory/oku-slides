//! Download loop: resume by `Range`, retries, SHA-256 on the fly.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use snafu::{Backtrace, IntoError, ResultExt, Snafu};

mod transport;

pub(super) use self::transport::agent;
use self::transport::{Fetched, Stop, backoff, fetch, next};
use super::resume::{self, Answer, PROGRESS_STEP};
use super::{COMPLETE, DownloadErrorKind, PinnedFile};
use crate::config::ModelsSettings;
use crate::storage::{Storage, StorageError, StreamWriter};

/// A failed model download.
#[derive(Debug, Snafu)]
pub struct DownloadError(Box<DownloadInner>);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
enum DownloadInner {
    #[snafu(display("cannot fetch {file}"))]
    Http { file: String, source: Box<ureq::Error>, backtrace: Backtrace },
    #[snafu(display("cannot read {file} from the network"))]
    HttpRead { file: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("{file}: the hub answered an unusable byte range"))]
    Range { file: String, backtrace: Backtrace },
    #[snafu(display("{file}: nothing received from the hub for {secs} s"))]
    Stalled { file: String, secs: u64, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Write { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("incomplete download: {got} of {expected} bytes"))]
    Incomplete { got: u64, expected: u64, backtrace: Backtrace },
    #[snafu(display("{file} does not match its pinned SHA-256"))]
    Integrity { file: String, backtrace: Backtrace },
    #[snafu(display("download cancelled"))]
    Cancelled { backtrace: Backtrace },
}

impl DownloadError {
    /// What failed, for the message shown to the user.
    #[must_use]
    pub fn kind(&self) -> DownloadErrorKind {
        match &*self.0 {
            DownloadInner::Http { .. }
            | DownloadInner::HttpRead { .. }
            | DownloadInner::Range { .. }
            | DownloadInner::Stalled { .. } => DownloadErrorKind::Network,
            DownloadInner::Write { .. } => DownloadErrorKind::Write,
            DownloadInner::Incomplete { got, expected, .. } => {
                DownloadErrorKind::Incomplete { got: *got, expected: *expected }
            }
            DownloadInner::Integrity { .. } => DownloadErrorKind::Integrity,
            DownloadInner::Cancelled { .. } => DownloadErrorKind::Cancelled,
        }
    }
}

/// Chunk read from a `.part` when re-hashing its prefix.
const HASH_CHUNK: usize = 1 << 20;

/// Failed attempts a file may spend in all, as a multiple of `download_retries`.
///
/// Progress starts the retry count over; this cap holds anyway, so a hub that sends a little
/// then stalls cannot pile up request threads blocked on dead connections without end.
const RETRY_BUDGET: u32 = 8;

/// Downloads `files` into `dir`, then writes [`COMPLETE`].
///
/// Each file is written as `.part`, checked then renamed; files already there are kept on
/// their length alone. A pinned file resumes its `.part` with `Range` (re-hashing the
/// prefix); an unpinned one restarts from zero. A cut, a stall, a short body or a busy hub
/// is retried `download_retries` times in a row, the count starting over once the `.part`
/// grows by [`PROGRESS_STEP`] (at most [`RETRY_BUDGET`] times as many failures in all); the
/// `.part` is kept for the next run. A slice that ends cleanly short of the file (a capped
/// range) is continued at once. `progress(received,
/// total)` is called regularly, even while waiting on the hub; if it returns `false`, the
/// download is cancelled and the `.part` deleted. Nothing received for `read_timeout_secs`:
/// a stall.
///
/// # Errors
///
/// On a network or write failure, a truncated file or one with an unexpected fingerprint,
/// too long a wait on the hub, or a cancellation by `progress`.
pub fn download(
    files: &[PinnedFile],
    dir: &Path,
    total: u64,
    settings: &ModelsSettings,
    storage: &Storage,
    mut progress: impl FnMut(u64, u64) -> bool,
) -> Result<(), DownloadError> {
    let timeout = Duration::from_secs(settings.read_timeout_secs);
    let agent = agent(timeout);
    let mut done = 0u64;
    for pinned in files {
        let path = dir.join(pinned.name);
        if let Ok(Some(len)) = storage.len(&path) {
            done += len;
            continue;
        }
        let mut job = Job::start(pinned, dir, &agent, settings, storage, done, total, &mut progress)?;
        done += job.run(&mut progress)?;
    }
    let marker = dir.join(COMPLETE);
    storage.write(&marker, Vec::new()).context(WriteSnafu { path: &marker })?;
    progress(total, total);
    Ok(())
}

/// How one attempt at a file ended, short of success.
enum Failure {
    /// Counted against `download_retries`, after a backoff.
    Retry(DownloadError),
    /// The body ended cleanly short of the file: asked again at once if it brought new
    /// bytes, else counted like [`Failure::Retry`].
    Next(DownloadError),
    /// The `.part` cannot be resumed: emptied and asked again from zero at once, once per
    /// file; the error ends the download the second time.
    Again(DownloadError),
    /// Stops the download.
    Fatal(DownloadError),
}

/// One file's download: its `.part`, the hash of what it holds, the attempts.
struct Job<'a> {
    pinned: &'a PinnedFile,
    url: String,
    part: PathBuf,
    path: PathBuf,
    agent: &'a ureq::Agent,
    settings: &'a ModelsSettings,
    storage: &'a Storage,
    /// Bytes of earlier files, and the whole download's size, for `progress`.
    done: u64,
    total: u64,
    /// Bytes in the `.part`, all fed to `hasher`.
    have: u64,
    hasher: Option<Sha256>,
    /// The `.part` started from bytes of an earlier attempt or run.
    resumed: bool,
    /// A fresh restart was already spent on a `.part` that could not be resumed.
    restarted: bool,
}

impl<'a> Job<'a> {
    /// Prepares `pinned`: an unpinned `.part` is dropped, a pinned one re-hashed.
    #[expect(clippy::too_many_arguments, reason = "the download's context, passed once")]
    fn start(
        pinned: &'a PinnedFile,
        dir: &Path,
        agent: &'a ureq::Agent,
        settings: &'a ModelsSettings,
        storage: &'a Storage,
        done: u64,
        total: u64,
        progress: &mut impl FnMut(u64, u64) -> bool,
    ) -> Result<Self, DownloadError> {
        let file = pinned.name;
        let url =
            format!("{}/{}/resolve/{}/{file}", settings.hub_url.trim_end_matches('/'), pinned.repo, pinned.revision);
        let mut job = Job {
            pinned,
            url,
            part: dir.join(format!("{file}.part")),
            path: dir.join(file),
            agent,
            settings,
            storage,
            done,
            total,
            have: 0,
            hasher: pinned.sha256.map(|_| Sha256::new()),
            resumed: false,
            restarted: false,
        };
        let len = match storage.len(&job.part) {
            Ok(len) => len.unwrap_or(0),
            Err(e) => {
                job.unreadable(&e);
                0
            }
        };
        if len > 0 && pinned.sha256.is_none() {
            // Its revision can move between runs: never resumed.
            discard(storage, &job.part);
        } else if len > 0 {
            job.rehash(progress)?;
        }
        Ok(job)
    }

    /// Feeds the `.part` to the hasher; an unreadable one is dropped and fetched again.
    fn rehash(&mut self, progress: &mut impl FnMut(u64, u64) -> bool) -> Result<(), DownloadError> {
        match self.hash_part(progress) {
            Ok(Some((read, hasher))) => {
                self.have = read;
                self.hasher = Some(hasher);
            }
            Ok(None) => {
                discard(self.storage, &self.part);
                return CancelledSnafu.fail()?;
            }
            Err(e) => self.unreadable(&e),
        }
        Ok(())
    }

    /// Drops a `.part` that cannot be read: it is fetched again (a failed drop shows on write).
    fn unreadable(&self, e: &StorageError) {
        tracing::warn!(
            path = %self.part.display(),
            error = %crate::errors::one_line(e),
            "fichier partiel illisible : téléchargé de nouveau"
        );
        discard(self.storage, &self.part);
    }

    /// The `.part`'s length and hash, a chunk at a time; `None` if `progress` cancels.
    fn hash_part(&self, progress: &mut impl FnMut(u64, u64) -> bool) -> Result<Option<(u64, Sha256)>, StorageError> {
        let mut reader = self.storage.reader(&self.part)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; HASH_CHUNK];
        let mut read = 0u64;
        loop {
            let n = reader.read_chunk(&mut buf)?;
            if n == 0 {
                return Ok(Some((read, hasher)));
            }
            hasher.update(&buf[..n]);
            read += n as u64;
            if !progress((self.done + read).min(self.total), self.total) {
                return Ok(None);
            }
        }
    }

    /// Attempts until the file is in place; returns its length.
    fn run(&mut self, progress: &mut impl FnMut(u64, u64) -> bool) -> Result<u64, DownloadError> {
        let retries = self.settings.download_retries;
        let budget = retries.saturating_mul(RETRY_BUDGET);
        let (mut attempt, mut failures) = (0, 0);
        // Where the count last started over, and the most the `.part` ever held.
        let (mut high, mut best) = (self.have, self.have);
        loop {
            if !progress(self.at(), self.total) {
                discard(self.storage, &self.part);
                return CancelledSnafu.fail()?;
            }
            let failed = match self.attempt(progress) {
                Ok(()) => return Ok(self.have),
                Err(failed) => failed,
            };
            let e = match failed {
                Failure::Again(_) if !self.restarted => {
                    self.restarted = true;
                    self.reset();
                    continue;
                }
                Failure::Again(e) | Failure::Fatal(e) => return Err(e),
                Failure::Next(_) if self.have > best => {
                    best = self.have;
                    continue;
                }
                Failure::Next(e) | Failure::Retry(e) => e,
            };
            best = best.max(self.have);
            // Real progress starts the count over; a cutting proxy that ignores `Range`
            // makes none and runs out of retries.
            if self.have >= high.saturating_add(PROGRESS_STEP) {
                high = self.have;
                attempt = 0;
            }
            if attempt >= retries || failures >= budget {
                return Err(e);
            }
            attempt += 1;
            failures += 1;
            let wait = Duration::from_secs(self.settings.download_backoff_secs.saturating_mul(u64::from(attempt)));
            if !backoff(wait, self.at(), self.total, progress) {
                discard(self.storage, &self.part);
                return CancelledSnafu.fail()?;
            }
        }
    }

    fn at(&self) -> u64 {
        (self.done + self.have).min(self.total)
    }

    /// Empties the `.part` and its hash: the next attempt starts from zero.
    fn reset(&mut self) {
        discard(self.storage, &self.part);
        self.have = 0;
        self.resumed = false;
        self.hasher = self.pinned.sha256.map(|_| Sha256::new());
    }

    fn file(&self) -> String {
        self.pinned.name.to_owned()
    }

    fn stalled(&self) -> DownloadError {
        StalledSnafu { file: self.file(), secs: self.settings.read_timeout_secs }.build().into()
    }

    /// One request: head, body into the `.part`, then the checks and the rename.
    fn attempt(&mut self, progress: &mut impl FnMut(u64, u64) -> bool) -> Result<(), Failure> {
        let timeout = Duration::from_secs(self.settings.read_timeout_secs);
        if self.pinned.sha256.is_none() && self.have > 0 {
            // Its revision can move: an unpinned file never resumes.
            self.reset();
        }
        let from = (self.have > 0).then_some(self.have);
        let rx = fetch(self.agent, self.url.clone(), from);
        let head = match next(&rx, timeout, self.at(), self.total, progress) {
            Ok(Fetched::Head(head)) => head,
            Ok(Fetched::Http(e)) => {
                return match *e {
                    // The `.part` already reaches the end: complete if its hash matches,
                    // else it is a stale prefix like any resumed one.
                    ureq::Error::StatusCode(416) if from.is_some() => {
                        self.resumed = true;
                        self.finish()
                    }
                    e => Err(self.http_failure(e)),
                };
            }
            Ok(Fetched::Read(e)) => return Err(Failure::Retry(self.read_error(e))),
            // The thread sends the head before any chunk.
            Ok(Fetched::Chunk(_) | Fetched::End) | Err(Stop::Stalled) => return Err(Failure::Retry(self.stalled())),
            Err(Stop::Cancelled) => return Err(self.cancel()),
        };
        let expected = match resume::answer(self.have, head.status, head.length, head.content_range.as_deref()) {
            Answer::Append { total } => {
                self.resumed = true;
                total
            }
            Answer::Fresh { total } => {
                self.reset();
                total
            }
            // Asked from the `.part`'s end: start over from zero; asked from zero, a broken hub.
            Answer::Refuse if from.is_some() => return Err(Failure::Again(self.range_error())),
            Answer::Refuse => return Err(Failure::Retry(self.range_error())),
        };
        let opened = if self.have > 0 { self.storage.appender(&self.part) } else { self.storage.writer(&self.part) };
        let mut out = opened.context(WriteSnafu { path: &self.part }).map_err(fatal)?;
        loop {
            match next(&rx, timeout, self.at(), self.total, progress) {
                Ok(Fetched::Chunk(chunk)) => {
                    // A body longer than announced is not the file: never written past its end.
                    if expected.is_some_and(|e| self.have.saturating_add(chunk.len() as u64) > e) {
                        drop(out);
                        return Err(self.mismatch());
                    }
                    out.write_all(&chunk).context(WriteSnafu { path: &self.part }).map_err(fatal)?;
                    if let Some(h) = self.hasher.as_mut() {
                        h.update(&chunk);
                    }
                    self.have += chunk.len() as u64;
                    if !progress(self.at(), self.total) {
                        drop(out);
                        return Err(self.cancel());
                    }
                }
                Ok(Fetched::End) => break,
                Ok(Fetched::Read(e)) => {
                    self.keep(out)?;
                    return Err(Failure::Retry(match expected {
                        Some(expected) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            IncompleteSnafu { got: self.have, expected }.build().into()
                        }
                        _ => self.read_error(e),
                    }));
                }
                Ok(Fetched::Head(_) | Fetched::Http(_)) | Err(Stop::Stalled) => {
                    self.keep(out)?;
                    return Err(Failure::Retry(self.stalled()));
                }
                Err(Stop::Cancelled) => {
                    drop(out);
                    return Err(self.cancel());
                }
            }
        }
        self.keep(out)?;
        if let Some(expected) = expected
            && self.have < expected
        {
            return Err(Failure::Next(IncompleteSnafu { got: self.have, expected }.build().into()));
        }
        self.finish()
    }

    /// A failed request: retried when the hub is busy or the connection dropped.
    fn http_failure(&self, e: ureq::Error) -> Failure {
        match e {
            e if resume::retryable(&e) => {
                Failure::Retry(HttpSnafu { file: self.file() }.into_error(Box::new(e)).into())
            }
            e => Failure::Fatal(HttpSnafu { file: self.file() }.into_error(Box::new(e)).into()),
        }
    }

    fn range_error(&self) -> DownloadError {
        RangeSnafu { file: self.file() }.build().into()
    }

    fn integrity(&self) -> DownloadError {
        IntegritySnafu { file: self.file() }.build().into()
    }

    fn read_error(&self, e: std::io::Error) -> DownloadError {
        HttpReadSnafu { file: self.file() }.into_error(e).into()
    }

    /// Closes the writer so the bytes received stay in the `.part`.
    fn keep(&self, out: StreamWriter) -> Result<(), Failure> {
        out.close().context(WriteSnafu { path: &self.part }).map_err(fatal)
    }

    fn cancel(&self) -> Failure {
        discard(self.storage, &self.part);
        Failure::Fatal(CancelledSnafu.build().into())
    }

    /// Drops a `.part` that is not the pinned file; a resumed one may hold bytes of an older
    /// pin, so it starts over once.
    fn mismatch(&self) -> Failure {
        discard(self.storage, &self.part);
        if self.resumed { Failure::Again(self.integrity()) } else { Failure::Fatal(self.integrity()) }
    }

    /// Checks the hash, then renames the `.part`; a resumed one that does not match starts over.
    fn finish(&mut self) -> Result<(), Failure> {
        if let (Some(pin), Some(h)) = (self.pinned.sha256, self.hasher.take())
            && !hex::encode(h.finalize()).eq_ignore_ascii_case(pin)
        {
            return Err(self.mismatch());
        }
        self.storage.rename(&self.part, &self.path).context(WriteSnafu { path: &self.path }).map_err(fatal)
    }
}

fn fatal(e: impl Into<DownloadError>) -> Failure {
    Failure::Fatal(e.into())
}

/// Removes a partial file; a failure changes nothing for the user (the next download
/// rewrites it), it is only logged.
fn discard(storage: &Storage, part: &Path) {
    if let Err(e) = storage.delete(part) {
        tracing::warn!(path = %part.display(), error = %crate::errors::one_line(&e), "fichier partiel");
    }
}

#[cfg(test)]
#[path = "download_hub_tests.rs"]
mod download_hub_tests;
#[cfg(test)]
#[path = "download_tests.rs"]
mod download_tests;
