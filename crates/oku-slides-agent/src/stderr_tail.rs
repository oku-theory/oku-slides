//! The tail of the agent's standard error, kept for diagnosis.
//!
//! Each launch pipes the child's stderr through [`forward`]: every byte still goes to the
//! app's stderr unchanged, and the last [`STDERR_TAIL_CAP`] lines are kept, redacted (`~`) and cut.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_platform::telemetry::{LOG_LINE_MAX, captured};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Lines of the agent's stderr kept at most, from its current launch.
pub(crate) const STDERR_TAIL_CAP: usize = 40;

/// The kept lines and the launch they belong to.
#[derive(Debug, Default)]
struct Lines {
    launch: u64,
    lines: VecDeque<String>,
}

/// The last lines of the agent's stderr; clones share them.
#[derive(Debug, Clone, Default)]
pub(crate) struct StderrTail {
    inner: Arc<Mutex<Lines>>,
    /// The home dir shown as `~`; `None`: nothing redacted.
    home: Option<String>,
}

impl StderrTail {
    /// An empty tail redacting `home`.
    pub(crate) fn new(home: Option<String>) -> Self {
        Self { inner: Arc::default(), home }
    }

    /// A new launch begins: the tail empties, lines of earlier launches are ignored from now on.
    ///
    /// Returns the launch number to [`StderrTail::push`] with.
    pub(crate) fn begin(&self) -> u64 {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        inner.launch += 1;
        inner.lines.clear();
        inner.launch
    }

    /// Keeps `line` of launch `launch`, dropping the oldest past [`STDERR_TAIL_CAP`].
    pub(crate) fn push(&self, launch: u64, line: &str) {
        let line = captured(line, self.home.as_deref());
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if inner.launch != launch {
            return;
        }
        if inner.lines.len() == STDERR_TAIL_CAP {
            inner.lines.pop_front();
        }
        inner.lines.push_back(line);
    }

    /// The lines kept, oldest first.
    pub(crate) fn lines(&self) -> Vec<String> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner).lines.iter().cloned().collect()
    }
}

/// Bytes read from the agent's stderr at once.
const CHUNK: usize = 4096;

/// Bytes of a line kept for the tail at most; the rest of the line, up to its end, is dropped.
pub(crate) const LINE_KEPT: usize = 4 * LOG_LINE_MAX;

/// Cuts the bytes read into the tail's lines: `\n`, `\r` and `\r\n` end one, a bounded prefix kept.
#[derive(Debug, Default)]
pub(crate) struct LineCut {
    /// The current line's first bytes, at most [`LINE_KEPT`].
    line: Vec<u8>,
    /// The last byte read ended a line with `\r`: a `\n` right after ends nothing more.
    after_cr: bool,
}

impl LineCut {
    /// Takes `bytes` in; returns the lines they end, lossy UTF-8, without their end.
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut ended = Vec::new();
        for piece in bytes.split_inclusive(|b| matches!(b, b'\n' | b'\r')) {
            let (body, end) = match piece.split_last() {
                Some((&end @ (b'\n' | b'\r'), body)) => (body, Some(end)),
                _ => (piece, None),
            };
            if end == Some(b'\n') && body.is_empty() && std::mem::take(&mut self.after_cr) {
                continue;
            }
            self.after_cr = false;
            let room = LINE_KEPT.saturating_sub(self.line.len());
            self.line.extend_from_slice(&body[..body.len().min(room)]);
            if let Some(end) = end {
                self.after_cr = end == b'\r';
                ended.push(String::from_utf8_lossy(&std::mem::take(&mut self.line)).into_owned());
            }
        }
        ended
    }

    /// The line still open at the end of the stream, if it has any byte.
    pub(crate) fn finish(self) -> Option<String> {
        (!self.line.is_empty()).then(|| String::from_utf8_lossy(&self.line).into_owned())
    }

    /// Bytes of the open line kept so far.
    #[cfg(test)]
    pub(crate) fn pending(&self) -> usize {
        self.line.len()
    }
}

/// Reads `reader` to its end in [`CHUNK`]s: each goes to `out` unchanged, its lines into `tail`.
///
/// Memory stays bounded whatever the agent writes: a line without end keeps only its first
/// [`LINE_KEPT`] bytes. Invalid UTF-8 is kept lossily in the tail. A failed write to `out` is
/// ignored, as an inherited stderr would ignore it; a read error ends the forwarding.
pub(crate) async fn forward(
    mut reader: impl AsyncRead + Unpin,
    mut out: impl AsyncWrite + Unpin,
    tail: StderrTail,
    launch: u64,
) {
    let mut chunk = vec![0u8; CHUNK];
    let mut cut = LineCut::default();
    loop {
        let n = match reader.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                tracing::debug!(error = %e, "agent stderr no longer read");
                break;
            }
        };
        let bytes = chunk.get(..n).unwrap_or_default();
        if out.write_all(bytes).await.is_ok() {
            let _ = out.flush().await;
        }
        for line in cut.feed(bytes) {
            tail.push(launch, &line);
        }
    }
    if let Some(line) = cut.finish() {
        tail.push(launch, &line);
    }
}

#[cfg(test)]
#[path = "stderr_tail_tests.rs"]
mod stderr_tail_tests;
