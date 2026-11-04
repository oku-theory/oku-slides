//! e2e fake utterance source: lines appended to a file become closed utterances.
//!
//! Compiled only with the `e2e-fakes` feature, which `just e2e` alone turns on (Q16): with
//! `stt.fake_utterances` set, listening opens no microphone and no engine. Each complete line
//! appended to the file after listening starts is one [`Utterance`](crate::Utterance), handed
//! out on the same channel as the real ones. Lines read while listening is suspended are
//! dropped, like captured audio. The reader stops when listening ends (`Stop`, `Cancel`) or
//! when the last [`Stt`] is dropped. The file is read through [`Storage`], its text never logged.

#[cfg(not(debug_assertions))]
compile_error!("the e2e-fakes feature is for debug e2e builds only; never build a release with it");

use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use oku_slides_platform::storage::Storage;

use crate::utterance::Draft;
use crate::{Inner, State, Stt};

/// The largest fake utterance file read (bytes): an e2e run writes a few lines.
const MAX_BYTES: u64 = 1 << 20;

/// Speech length given to each fake utterance (s); the router reads text only.
const FAKE_SPEECH_SECS: f32 = 1.0;

/// Where the reader stands in the file: what it has already handed out.
#[derive(Debug)]
pub(crate) struct FakeSource {
    path: PathBuf,
    /// Bytes before this offset were read, or there before listening started.
    offset: u64,
}

impl FakeSource {
    /// A reader at the current end of `path`: what the file already holds is never heard.
    pub(crate) fn at_end(storage: &Storage, path: PathBuf) -> Self {
        let offset = match storage.len(&path) {
            Ok(len) => len.unwrap_or(0),
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "fake utterances: unreadable file");
                0
            }
        };
        Self { path, offset }
    }

    /// The complete lines appended since the last read, trimmed, without the empty ones.
    ///
    /// A line still being written (no final newline) waits for the next read.
    pub(crate) fn read_lines(&mut self, storage: &Storage) -> Vec<String> {
        let bytes = match storage.len(&self.path) {
            Ok(Some(len)) if len == self.offset => return Vec::new(),
            Ok(Some(len)) if len < self.offset => {
                // Truncated: start again from its new end.
                self.offset = len;
                return Vec::new();
            }
            Ok(None) => return Vec::new(),
            Ok(Some(_)) => storage.read_limited(&self.path, MAX_BYTES),
            Err(e) => Err(e),
        };
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "fake utterances: unreadable file");
                return Vec::new();
            }
        };
        let Ok(start) = usize::try_from(self.offset) else { return Vec::new() };
        let Some(fresh) = bytes.get(start..) else { return Vec::new() };
        let Some(last) = fresh.iter().rposition(|&b| b == b'\n') else { return Vec::new() };
        // `last` lies inside `fresh`, so the new offset is at most the file length.
        let Ok(end) = u64::try_from(start + last + 1) else { return Vec::new() };
        self.offset = end;
        fresh[..=last]
            .split(|&b| b == b'\n')
            .map(|line| String::from_utf8_lossy(line).trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect()
    }
}

/// One read of `source`: each new line becomes an utterance unless listening is suspended.
///
/// Returns false once listening `session` ended: the reader stops.
pub(crate) fn tick(inner: &Mutex<Inner>, storage: &Storage, source: &mut FakeSource, session: u64) -> bool {
    let alive = |g: &Inner| g.session == session && g.state == State::Listening;
    if !alive(&inner.lock().unwrap_or_else(PoisonError::into_inner)) {
        return false;
    }
    // The file is read outside the lock.
    let lines = source.read_lines(storage);
    let g = inner.lock().unwrap_or_else(PoisonError::into_inner);
    if !alive(&g) {
        return false;
    }
    for text in lines {
        if g.suspends > 0 {
            tracing::debug!("fake utterance dropped while suspended");
            continue;
        }
        let now = Instant::now();
        g.deliver(Draft { text, started: now, ended: now, speech_secs: FAKE_SPEECH_SECS });
    }
    true
}

/// How the fake reader thread ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReaderEnd {
    /// Listening ended, or the dictation was dropped.
    Ended,
    /// A read panicked; the panic was logged.
    Panicked,
}

/// Calls `step` every `every` until it returns false; a panic in it is caught and logged.
pub(crate) fn reader_loop(every: Duration, mut step: impl FnMut() -> bool) -> ReaderEnd {
    let looped = std::panic::catch_unwind(AssertUnwindSafe(|| {
        loop {
            std::thread::sleep(every);
            if !step() {
                return ReaderEnd::Ended;
            }
        }
    }));
    looped.unwrap_or_else(|_| {
        tracing::error!("fake utterance reader panicked: no more fake utterances are heard");
        ReaderEnd::Panicked
    })
}

impl Stt {
    /// Listens to the fake utterance file at `path` instead of the microphone, from state `from`.
    ///
    /// The file's end is measured before `Listening` shows: a line appended once the page sees
    /// it listening is always new.
    pub(crate) fn begin_fake(&self, path: PathBuf, from: State) {
        self.listen_with(FakeSource::at_end(&self.storage, path), from);
    }

    /// Publishes `Listening` from state `from`, then reads `source` on its own thread.
    pub(crate) fn listen_with(&self, mut source: FakeSource, from: State) {
        let session = {
            let mut g = self.state();
            if g.state != from {
                return;
            }
            g.session += 1;
            g.state = State::Listening;
            g.phrases.discard();
            g.partial.clear();
            g.stop_in = None;
            g.started = Some(Instant::now());
            g.session
        };
        let (inner, storage) = (Arc::downgrade(&self.inner), self.storage.clone());
        let every = Duration::from_millis(self.settings.tick_ms.get());
        std::thread::spawn(move || {
            reader_loop(every, || inner.upgrade().is_some_and(|inner| tick(&inner, &storage, &mut source, session)))
        });
    }
}

#[cfg(test)]
#[path = "fake_source_tests.rs"]
mod fake_source_tests;
