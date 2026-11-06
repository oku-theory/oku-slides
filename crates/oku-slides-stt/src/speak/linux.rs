//! The Linux voice: Speech Dispatcher, spoken to over its SSIP socket, one connection per speech.
//!
//! The socket is `unix_socket:<path>` from `$SPEECHD_ADDRESS`, else
//! `$XDG_RUNTIME_DIR/speech-dispatcher/speechd.sock`, as libspeechd finds it. An absent socket
//! starts the daemon as libspeechd does (`speech-dispatcher --spawn`, bounded in time): a
//! missing program is `NoEngine`, a daemon that does not start or answer is `DaemonSilent`.
//!
//! Each speech opens its own connection (`ssip.rs`): the text goes over the socket as data, never
//! in an argv or a log. A waiter thread reads the connection's events and calls DONE once, on
//! END or CANCELED of its message, when the daemon goes away, or [`CUT_GRACE`] after a cut the
//! daemon never confirms. A cut writes `CANCEL self` on that connection: only our own messages
//! stop, never another client's.

use std::os::unix::net::UnixStream;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use snafu::IntoError;

use super::ssip::{Event, EventKind, Ssip, cancel};
use super::{
    SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo, chosen, for_language,
};
use crate::error::{SttError, VoiceIoSnafu, VoiceUnavailableSnafu};

/// How long the daemon may take to answer one command before it counts as silent.
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);

/// How long `speech-dispatcher --spawn` may take to start the daemon.
const SPAWN_TIMEOUT: Duration = Duration::from_secs(5);

/// How often a waiter looks up from its connection, and a spawn from its child.
const POLL: Duration = Duration::from_millis(50);

/// How long a waiter waits for the daemon to confirm a cut before calling DONE anyway.
const CUT_GRACE: Duration = Duration::from_secs(1);

/// One speech in progress: its id, the connection to cancel it on, and when it was cut.
#[derive(Debug)]
struct Running {
    id: SpeechId,
    control: UnixStream,
    cut: Arc<OnceLock<Instant>>,
}

/// The Speech Dispatcher voice.
#[derive(Debug)]
pub(crate) struct SpeechDispatcher {
    /// The daemon's socket; `None` when `$SPEECHD_ADDRESS` names no Unix socket.
    socket: Option<PathBuf>,
    /// The `speech-dispatcher` program that starts the daemon, a path or a name on `PATH`.
    spawner: PathBuf,
    /// The user part of our client name.
    user: String,
    /// The speech last started, until it ends, is cut or is replaced.
    current: Arc<Mutex<Option<Running>>>,
}

impl SpeechDispatcher {
    /// A voice that talks to the daemon at `socket`, started by `spawner` when absent.
    pub(crate) fn new(socket: Option<PathBuf>, spawner: PathBuf, user: String) -> Self {
        Self { socket, spawner, user, current: Arc::new(Mutex::new(None)) }
    }

    /// The voice of this session, found as libspeechd finds it.
    #[cfg(target_os = "linux")]
    pub(crate) fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        // glib's `g_get_user_runtime_dir`, which the daemon uses too: the cache dir without it.
        let runtime = var("XDG_RUNTIME_DIR")
            .or_else(|| var("XDG_CACHE_HOME"))
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|home| Path::new(&home).join(".cache")));
        let socket = socket_path(var("SPEECHD_ADDRESS").as_deref(), runtime);
        if socket.is_none() {
            tracing::warn!("SPEECHD_ADDRESS names no Unix socket: the Linux voice is unavailable");
        }
        let user = var("USER").or_else(|| var("LOGNAME")).unwrap_or_default();
        // Looked up on `PATH`: a sandboxed build (AppImage, Flatpak) that cannot see it is unavailable.
        Self::new(socket, "speech-dispatcher".into(), user)
    }

    /// A connection to the daemon, started first if its socket is absent.
    fn open(&self) -> Result<UnixStream, UnavailableReason> {
        let Some(path) = &self.socket else { return Err(UnavailableReason::DaemonSilent) };
        let stream = match UnixStream::connect(path) {
            Ok(stream) => stream,
            Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused) => {
                self.spawn(path)?;
                UnixStream::connect(path).map_err(|_| UnavailableReason::DaemonSilent)?
            }
            Err(e) => {
                tracing::debug!(error = %e, "speech daemon socket");
                return Err(UnavailableReason::DaemonSilent);
            }
        };
        let timeouts = stream.set_read_timeout(Some(REPLY_TIMEOUT)).and(stream.set_write_timeout(Some(REPLY_TIMEOUT)));
        timeouts.map_err(|_| UnavailableReason::DaemonSilent)?;
        Ok(stream)
    }

    /// Starts the daemon on `path` as libspeechd's autospawn does, within [`SPAWN_TIMEOUT`].
    fn spawn(&self, path: &Path) -> Result<(), UnavailableReason> {
        let spawned = Command::new(&self.spawner)
            .args(["--spawn", "--communication-method", "unix_socket", "--socket-path"])
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(UnavailableReason::NoEngine),
            Err(_) => return Err(UnavailableReason::DaemonSilent),
        };
        let deadline = Instant::now().checked_add(SPAWN_TIMEOUT);
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(None) if deadline.is_some_and(|d| Instant::now() < d) => std::thread::sleep(POLL),
                _ => {
                    child.kill().ok();
                    child.wait().ok();
                    return Err(UnavailableReason::DaemonSilent);
                }
            }
        }
    }

    /// The voices of the daemon's current synthesizer.
    fn list(&self) -> Result<Vec<VoiceInfo>, UnavailableReason> {
        let stream = self.open()?;
        let listed = Ssip::connect(stream, &self.user).and_then(|mut ssip| {
            let voices = ssip.voices();
            ssip.quit();
            voices
        });
        listed.map_err(|e| {
            tracing::debug!(error = %e, "speech daemon voice list");
            UnavailableReason::DaemonSilent
        })
    }

    /// Takes speech `id` out of `current` if it is still the one there.
    fn take(&self, id: SpeechId) -> Option<Running> {
        let mut g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if g.as_ref().is_some_and(|r| r.id == id) { g.take() } else { None }
    }
}

/// `socket` of `$SPEECHD_ADDRESS` (`unix_socket[:<path>]`), else the default under `runtime`.
///
/// Another method (`inet_socket`) is not supported: `None`.
pub(crate) fn socket_path(address: Option<&str>, runtime: Option<PathBuf>) -> Option<PathBuf> {
    let default = || runtime.map(|dir| dir.join("speech-dispatcher").join("speechd.sock"));
    let Some(address) = address else { return default() };
    let (method, path) = address.split_once(':').unwrap_or((address, ""));
    match (method, path) {
        ("unix_socket", "") => default(),
        ("unix_socket", path) => Some(PathBuf::from(path)),
        _ => None,
    }
}

/// The `RATE` value: 0 is the default rate, ±100 twice faster or slower.
fn rate_arg(rate: SpeechRate) -> i32 {
    crate::num::round_i32(rate.get().log2() * 100.0).clamp(-100, 100)
}

/// Whether `text` has nothing to say: only whitespace and NULs (spoken as spaces).
fn silent(text: &str) -> bool {
    text.chars().all(|c| c.is_whitespace() || c == '\0')
}

/// The unavailable error for a daemon that cannot be reached.
fn unavailable(reason: UnavailableReason) -> SttError {
    VoiceUnavailableSnafu { reason }.build().into()
}

/// Cancels a speech: marks it cut, then sends `CANCEL self` on its connection.
fn cut(running: &Running) {
    running.cut.set(Instant::now()).ok();
    if let Err(e) = cancel(&running.control) {
        // The daemon is gone: the waiter reads the end of the connection and calls DONE.
        tracing::debug!(error = %e, "speech daemon CANCEL");
    }
}

/// Reads `ssip` until message `msg` ends or is cancelled, then clears `id` and calls DONE.
fn wait(
    mut ssip: Ssip<UnixStream>,
    msg: u64,
    cut: &OnceLock<Instant>,
    current: &Mutex<Option<Running>>,
    id: SpeechId,
    done: SpeechDone,
) {
    loop {
        match ssip.read_event() {
            Ok(Some(Event { msg: m, kind: EventKind::End | EventKind::Cancel })) if m == msg => break,
            Ok(None) if cut.get().is_some_and(|at| at.elapsed() >= CUT_GRACE) => break,
            // Another message's event, a BEGIN, or nothing yet.
            Ok(_) => {}
            Err(e) => {
                tracing::debug!(error = %e, id = id.get(), "speech connection ended");
                break;
            }
        }
    }
    ssip.quit();
    {
        let mut g = current.lock().unwrap_or_else(PoisonError::into_inner);
        if g.as_ref().is_some_and(|r| r.id == id) {
            *g = None;
        }
    }
    done.call(id);
}

impl SpeechEngine for SpeechDispatcher {
    fn status(&self, lang: &str) -> SpeechStatus {
        match self.list() {
            Ok(voices) if voices.is_empty() => SpeechStatus::Unavailable { reason: UnavailableReason::DaemonSilent },
            Ok(voices) => for_language(lang, voices),
            Err(reason) => SpeechStatus::Unavailable { reason },
        }
    }

    fn speak(
        &self,
        id: SpeechId,
        text: &str,
        voice: Option<&str>,
        rate: SpeechRate,
        done: SpeechDone,
    ) -> Result<(), SttError> {
        // The voice comes from `status`, already listed. For a voice removed since, Speech
        // Dispatcher silently reads with another voice of its own choice.
        let voice = chosen(voice)?;
        if silent(text) {
            // Nothing to say: the daemon would still queue an empty line and report it later.
            // The speech in progress is cut, and this one is done here, without the daemon.
            let previous = self.current.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(previous) = previous {
                cut(&previous);
            }
            done.call(id);
            return Ok(());
        }
        let stream = self.open().map_err(unavailable)?;
        let control = stream.try_clone().map_err(|e| VoiceIoSnafu { step: "connect" }.into_error(e))?;
        let mut ssip = Ssip::connect(stream, &self.user)?;
        let msg = ssip.speak(text, voice, rate_arg(rate))?;
        // The waiter looks up every POLL to see whether a cut went unconfirmed.
        if let Err(e) = control.set_read_timeout(Some(POLL)) {
            cancel(&control).ok();
            return Err(VoiceIoSnafu { step: "connect" }.into_error(e).into());
        }
        let cut_at = Arc::new(OnceLock::new());
        let running = Running { id, control, cut: Arc::clone(&cut_at) };
        let previous = self.current.lock().unwrap_or_else(PoisonError::into_inner).replace(running);
        if let Some(previous) = previous {
            cut(&previous);
        }
        let current = Arc::clone(&self.current);
        let spawned = std::thread::Builder::new().name("oku-voice".into()).spawn(move || {
            if catch_unwind(AssertUnwindSafe(|| wait(ssip, msg, &cut_at, &current, id, done))).is_err() {
                tracing::error!(id = id.get(), "voice waiter panicked");
            }
        });
        if let Err(e) = spawned {
            // No waiter: the speech is cut, and `done` was dropped with the closure.
            if let Some(running) = self.take(id) {
                cut(&running);
            }
            return Err(VoiceIoSnafu { step: "waiter" }.into_error(e).into());
        }
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        if let Some(running) = self.take(id) {
            cut(&running);
        }
        Ok(())
    }
}

impl Drop for SpeechDispatcher {
    fn drop(&mut self) {
        // Speech outlives no engine: it is cancelled, its waiter still calls DONE.
        let running = self.current.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(running) = running {
            cut(&running);
        }
    }
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod linux_tests;
