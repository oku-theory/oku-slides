//! The login shell's `PATH`, for an app launched from the Finder or the Dock.
//!
//! launchd gives such an app `/usr/bin:/bin:/usr/sbin:/sbin`: `npx`, `node` or `soffice`
//! installed by Homebrew, nvm or a package manager are not found. The user's shell, run
//! once at startup as a login and interactive shell (`$SHELL -ilc`, its rc files read),
//! prints its `PATH` after a marker; its entries come first, then the process's missing ones.
//! [`ProgramPath`] resolves it on a thread, so the window opens without waiting.

use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Printed before the `PATH`, so what the rc files print is skipped.
const MARKER: &str = "__OKU_SLIDES_PATH__=";
/// The shell gets this long at most; slow rc files cost no more at startup.
pub(crate) const LOGIN_PATH_TIMEOUT: Duration = Duration::from_secs(3);

/// The user's shell: `$SHELL`, else zsh on macOS; `None` elsewhere without `$SHELL`.
fn user_shell() -> Option<PathBuf> {
    match std::env::var_os("SHELL").filter(|s| !s.is_empty()) {
        Some(shell) => Some(PathBuf::from(shell)),
        None if cfg!(target_os = "macos") => Some(PathBuf::from("/bin/zsh")),
        None => None,
    }
}

/// The `PATH` printed after the marker in `output` (its last such line).
pub(crate) fn marked_path(output: &[u8]) -> Option<OsString> {
    let text = String::from_utf8_lossy(output);
    let path = text.lines().rev().find_map(|l| l.strip_prefix(MARKER))?.trim_end_matches('\r');
    (!path.is_empty()).then(|| OsString::from(path))
}

/// `login`'s entries, then those of `own` it lacks, in order; absolute ones only.
///
/// An empty or relative entry would search the app's working folder.
pub(crate) fn merged(login: &OsStr, own: Option<&OsStr>) -> OsString {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let entries = std::env::split_paths(login).chain(own.map(std::env::split_paths).into_iter().flatten());
    for dir in entries.filter(|d| d.is_absolute()) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    // Entries split from a `PATH` hold no separator: joining them cannot fail.
    std::env::join_paths(&dirs).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "PATH du shell de connexion inutilisable");
        login.to_owned()
    })
}

/// The `PATH` that `shell` prints as a login shell, waiting `timeout` at most.
///
/// `None` (logged) when the shell does not start, fails, prints no marker or is too slow
/// (it is killed).
pub(crate) fn shell_path(shell: &Path, timeout: Duration) -> Option<OsString> {
    let script = format!("printf '\\n%s%s\\n' '{MARKER}' \"$PATH\"");
    let mut child = Command::new(shell)
        .args(["-i", "-l", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .inspect_err(|e| tracing::warn!(shell = %shell.display(), error = %e, "shell de connexion introuvable"))
        .ok()?;
    let stdout = child.stdout.take()?;
    let started = Instant::now();
    // Read on a thread, up to the marker line only: a background job started by an rc file
    // may keep the pipe open long after the shell has printed its `PATH`.
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::Builder::new().name("login-path-read".into()).spawn(move || {
        let marked = BufReader::new(stdout).split(b'\n').map_while(Result::ok).find_map(|line| marked_path(&line));
        let _ = tx.send(marked);
    });
    if let Err(e) = reader {
        tracing::warn!(error = %e, "lecture du shell de connexion impossible");
        stop(&mut child);
        return None;
    }
    let path = match rx.recv_timeout(timeout) {
        Ok(path) => path,
        Err(RecvTimeoutError::Timeout) => {
            tracing::warn!(shell = %shell.display(), "shell de connexion trop lent : PATH du processus gardé");
            None
        }
        Err(RecvTimeoutError::Disconnected) => {
            tracing::warn!(shell = %shell.display(), "lecture du shell de connexion interrompue");
            None
        }
    };
    if path.is_none() {
        tracing::warn!(shell = %shell.display(), "le shell de connexion n'a pas donné son PATH");
    }
    reap(&mut child, timeout.saturating_sub(started.elapsed()));
    path
}

/// Waits `grace` at most for the shell to end, then kills it.
fn reap(child: &mut std::process::Child, grace: Duration) {
    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                tracing::warn!(error = %e, "shell de connexion perdu");
                break;
            }
        }
    }
    stop(child);
}

/// Kills the shell and reaps it.
fn stop(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// The `PATH` where external programs (agent, soffice) are found and launched.
///
/// Fixed (the process's own, tests and the CLI) or resolved once from the login shell on a
/// thread; [`ProgramPath::get`] waits for it, a few seconds at most.
#[derive(Debug, Clone)]
pub struct ProgramPath(Arc<OnceLock<Option<OsString>>>);

impl ProgramPath {
    /// This `PATH`, as is.
    #[must_use]
    pub fn fixed(path: Option<OsString>) -> Self {
        Self(Arc::new(OnceLock::from(path)))
    }

    /// The login shell's `PATH` then `own`'s missing entries, resolved on a thread.
    #[must_use]
    pub fn from_login_shell(own: Option<OsString>) -> Self {
        let cell = Arc::new(OnceLock::new());
        let filled = cell.clone();
        let started = std::thread::Builder::new().name("login-path".into()).spawn(move || {
            // The cell is filled whatever happens, or `get` would wait forever.
            let path = std::panic::catch_unwind(AssertUnwindSafe(|| login_path(own.as_deref()))).unwrap_or_else(|_| {
                tracing::warn!("PATH du shell de connexion non résolu : PATH du processus gardé");
                std::env::var_os("PATH")
            });
            let _ = filled.set(path);
        });
        if let Err(e) = started {
            tracing::warn!(error = %e, "PATH du shell de connexion non résolu");
            // The thread did not start: `own` went with it, the process's PATH stands in.
            let _ = cell.set(std::env::var_os("PATH"));
        }
        Self(cell)
    }

    /// The `PATH`, waiting for the login shell if it is still running.
    #[must_use]
    pub fn get(&self) -> Option<&OsStr> {
        self.0.wait().as_deref()
    }
}

/// The process's `PATH` with the login shell's entries first; `own` unchanged without a shell.
pub(crate) fn login_path(own: Option<&OsStr>) -> Option<OsString> {
    let login = user_shell().and_then(|shell| shell_path(&shell, LOGIN_PATH_TIMEOUT));
    match login {
        Some(login) => Some(merged(&login, own)),
        None => own.map(OsStr::to_owned),
    }
}

#[cfg(test)]
#[path = "login_path_tests.rs"]
mod login_path_tests;
