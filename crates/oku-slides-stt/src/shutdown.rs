//! End of dictation: the phrase transcription threads are joined (t-1663).
//!
//! Parakeet loads ONNX Runtime on its transcription thread. If the process exits during that
//! load, ONNX Runtime's static objects are destroyed under the thread and the process crashes.
//! [`Stt::shutdown`], or dropping the last owning [`Stt`], stops the dictation and waits for
//! those threads first.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use crate::{Inner, Stt};

/// The phrase transcription threads not yet joined, shared by every clone of a [`Stt`].
pub(crate) type Transcribers = Arc<Mutex<Vec<JoinHandle<()>>>>;

/// Held by the owning clones of a [`Stt`], not by its own threads: the last one stops the
/// dictation and joins its transcription threads.
pub(crate) struct Owner {
    inner: Arc<Mutex<Inner>>,
    transcribers: Transcribers,
    cancel_download: Arc<AtomicBool>,
}

impl Owner {
    pub(crate) fn new(
        inner: Arc<Mutex<Inner>>,
        transcribers: Transcribers,
        cancel_download: Arc<AtomicBool>,
    ) -> Arc<Self> {
        Arc::new(Self { inner, transcribers, cancel_download })
    }
}

impl Drop for Owner {
    // May run on a tokio worker (the dev server's state): the wait is bounded by a model load
    // in progress (about 2 s) plus one listening tick, and adds no other wait.
    fn drop(&mut self) {
        stop_and_join(&self.inner, &self.transcribers, &self.cancel_download);
    }
}

/// Abandons the dictation for good, then joins the transcription threads with no lock held.
///
/// Like `Stt::cancel`, plus: nothing paused by a download resumes, and `shut` refuses any new
/// capture, download or transcriber. A transcriber is pushed under the `Inner` lock that
/// checks `shut` ([`Stt::spawn_transcriber`]), so every one spawned before is taken below.
fn stop_and_join(inner: &Mutex<Inner>, transcribers: &Mutex<Vec<JoinHandle<()>>>, cancel_download: &AtomicBool) {
    cancel_download.store(true, Ordering::Relaxed);
    {
        let mut g = inner.lock().unwrap_or_else(PoisonError::into_inner);
        g.shut = true;
        g.start_after_download = None;
        g.listen_after_download = false;
        g.reset();
    }
    let threads = std::mem::take(&mut *transcribers.lock().unwrap_or_else(PoisonError::into_inner));
    for thread in threads {
        if thread.join().is_err() {
            tracing::warn!("transcription thread panicked");
        }
    }
}

impl Stt {
    /// Stops the dictation and waits for its transcription threads (end of the app).
    ///
    /// Blocks while a model load is in progress (about 2 s): call it off async workers.
    pub fn shutdown(&self) {
        stop_and_join(&self.inner, &self.transcribers, &self.cancel_download);
    }

    /// A clone for the dictation's own threads: it never keeps the dictation alive.
    pub(crate) fn worker(&self) -> Self {
        Self { _owner: None, ..self.clone() }
    }

    /// Runs `work` on a transcription thread, joined at [`Stt::shutdown`]; false after it.
    #[must_use = "false: shut down, nothing started"]
    pub(crate) fn spawn_transcriber(&self, work: impl FnOnce() + Send + 'static) -> bool {
        // Held until the push: a shutdown either refuses this spawn or joins the thread.
        let g = self.state();
        if g.shut {
            return false;
        }
        let mut threads = self.transcribers.lock().unwrap_or_else(PoisonError::into_inner);
        threads.retain(|t| !t.is_finished());
        threads.push(std::thread::spawn(work));
        drop(threads);
        drop(g);
        true
    }
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod shutdown_tests;
