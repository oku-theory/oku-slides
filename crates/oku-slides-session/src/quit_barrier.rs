//! The windows a guarded quit waits for before the app exits (t-1377).
//!
//! A quit asks every window to run its close guards (deferred widget writes, the autosave, the
//! detached agent's draft) and to answer; the host exits once all answered or its bound passes.
//! This is the bookkeeping, without Tauri: the host holds one behind a mutex and waits outside it.

use std::collections::BTreeSet;

/// How a quit request starts against the barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitStart {
    /// The windows were asked: wait for their answers (or the bound).
    Wait,
    /// No window to ask: exit now.
    Ready,
    /// A quit is already underway: it exits, this request does nothing more.
    Joined,
}

/// The state of the app's one guarded quit.
#[derive(Debug, Default)]
pub struct QuitBarrier {
    state: State,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    /// The windows that have not answered yet.
    Waiting(BTreeSet<String>),
    /// The exit runs: every answer and request after this is ignored.
    Released,
}

impl QuitBarrier {
    /// Starts a quit waiting for `windows`, or joins the one underway.
    pub fn start(&mut self, windows: impl IntoIterator<Item = String>) -> QuitStart {
        if !matches!(self.state, State::Idle) {
            return QuitStart::Joined;
        }
        let waiting: BTreeSet<String> = windows.into_iter().collect();
        if waiting.is_empty() {
            self.state = State::Released;
            return QuitStart::Ready;
        }
        self.state = State::Waiting(waiting);
        QuitStart::Wait
    }

    /// Records window `label`'s answer; true only for the answer that completes the wait.
    ///
    /// An answer from a window not waited for, a second one, or one with no quit waiting is ignored.
    pub fn answer(&mut self, label: &str) -> bool {
        let State::Waiting(waiting) = &mut self.state else {
            return false;
        };
        if !waiting.remove(label) || !waiting.is_empty() {
            return false;
        }
        self.state = State::Released;
        true
    }

    /// The windows that have not answered the quit waiting, in label order; none otherwise.
    #[must_use]
    pub fn pending(&self) -> Vec<String> {
        match &self.state {
            State::Waiting(waiting) => waiting.iter().cloned().collect(),
            State::Idle | State::Released => Vec::new(),
        }
    }

    /// The exit runs (every window answered or the bound passed): later answers are ignored.
    pub fn release(&mut self) {
        self.state = State::Released;
    }
}

#[cfg(test)]
#[path = "quit_barrier_tests.rs"]
mod quit_barrier_tests;
