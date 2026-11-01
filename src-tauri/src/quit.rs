//! Guarded quit: every window runs its close guards before the app exits (t-1377).
//!
//! A quit emits `quit-requested` to every window (workspace windows flush their tabs' deferred
//! writes and autosave, the detached agent its draft) and waits for each `quit_ready`, at most
//! [`QUIT_WAIT`], before the exit runs. `closing` is set only by `state::settle` after the wait,
//! so the guards' own `request` and `workspace` calls are still served meanwhile. The waiting set
//! is [`QuitBarrier`] (session crate, tested there).

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use oku_slides_session::{QuitBarrier, QuitStart};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tokio::sync::Notify;

use crate::state;

/// How long a page runs its close guards on quit, in milliseconds.
///
/// The UI's `CLOSE_GUARD_MS` has the same value.
///
/// Sent with `quit-requested`, so the page bounds its guards by the host's own figure.
pub const QUIT_GUARD_MS: u64 = 3000;

/// How long a quit waits for the windows' answers: their guard bound plus a margin.
const QUIT_WAIT: Duration = Duration::from_millis(QUIT_GUARD_MS + 1000);

/// The app's one guarded quit: the windows still to answer, and the waiter's wake-up.
#[derive(Debug, Default)]
pub struct Quit {
    barrier: Mutex<QuitBarrier>,
    answered: Notify,
}

/// The `quit-requested` payload: the guard bound the page applies.
#[derive(Debug, Clone, serde::Serialize)]
struct QuitRequested {
    ms: u64,
}

/// Quits after the windows' close guards; returns at once.
///
/// The wait runs on the async runtime, so this is safe on the main thread.
pub fn exit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if guarded(&app).await {
            let job = app.clone();
            if let Err(e) = tauri::async_runtime::spawn_blocking(move || state::shutdown(&job)).await {
                tracing::error!(error = %e, "shutdown failed");
            }
        }
    });
}

/// Runs every window's close guards, waiting at most [`QUIT_WAIT`].
///
/// False if a quit is already underway (it exits, not the caller). On true the caller exits (`state::shutdown`, or `settle` and a restart for an update).
pub async fn guarded(app: &AppHandle) -> bool {
    let Some(quit) = app.try_state::<Quit>() else {
        tracing::error!("quit: Quit is not managed, exiting without the close guards");
        return true;
    };
    let labels: Vec<String> = app.webview_windows().into_keys().collect();
    let start = quit.barrier.lock().unwrap_or_else(PoisonError::into_inner).start(labels.iter().cloned());
    match start {
        QuitStart::Joined => return false,
        QuitStart::Ready => return true,
        QuitStart::Wait => {}
    }
    for label in &labels {
        if let Err(e) = app.emit_to(label.as_str(), "quit-requested", QuitRequested { ms: QUIT_GUARD_MS }) {
            tracing::warn!(label, error = %e, "quit not sent to a window");
        }
    }
    let timed_out = tokio::time::timeout(QUIT_WAIT, quit.answered.notified()).await.is_err();
    let mut barrier = quit.barrier.lock().unwrap_or_else(PoisonError::into_inner);
    if timed_out {
        let missing = barrier.pending();
        tracing::warn!(wait_ms = QUIT_WAIT.as_millis(), ?missing, "quit: windows did not answer, exiting anyway");
    }
    barrier.release();
    true
}

/// A window ran its close guards for the quit underway; the window is the caller, never a payload.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "a Tauri command takes its arguments by value")]
pub fn quit_ready(window: WebviewWindow, quit: State<'_, Quit>) {
    if quit.barrier.lock().unwrap_or_else(PoisonError::into_inner).answer(window.label()) {
        // Stores a permit when the waiting task is not polled yet: never lost.
        quit.answered.notify_one();
    }
}
