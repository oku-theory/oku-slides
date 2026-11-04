//! The Linux voice: `spd-say`, the Speech Dispatcher client, one child process per speech.
//!
//! Each speech runs `spd-say -w -N oku-slides -y VOICE -r RATE -- TEXT`, stdin null, a NUL in
//! the text spoken as a space:
//! - `-w` keeps the child alive as long as the speech, so its exit is DONE;
//! - `-N` names the client `oku-slides`;
//! - `--` ends the options, so a text starting with `-` is spoken, never parsed;
//! - never `-C`, which would cancel the other clients' speech.
//!
//! A cut kills our own child only; a waiter thread per speech reaps it and calls DONE once.
//! Voices come from `spd-say -L`, bounded in time: a missing program is `NoEngine`, a daemon
//! that fails, hangs or lists nothing is `DaemonSilent`. Once `-L` exits, its output is read
//! after at most [`EOF_GRACE`], even if an autospawned daemon still holds the pipe; the reader
//! thread then ends with that daemon.

use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use snafu::IntoError;

use super::{
    SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo, chosen, for_language,
};
use crate::error::{SttError, VoiceSpawnSnafu, VoiceUnavailableSnafu};

/// The client name Speech Dispatcher shows for our speech.
const CLIENT: &str = "oku-slides";

/// How long `spd-say -L` may take before the daemon counts as silent.
const LIST_TIMEOUT: Duration = Duration::from_secs(3);

/// How long the listing waits for EOF once `spd-say -L` exited, before reading what came.
const EOF_GRACE: Duration = Duration::from_millis(200);

/// How often a waiter checks whether its child has exited.
const POLL: Duration = Duration::from_millis(20);

/// The largest voice list read (bytes): a few hundred voices take a few KiB.
const MAX_LIST_BYTES: u64 = 1 << 20;

/// One speech in progress: its id and its child, shared with its waiter.
#[derive(Debug)]
struct Running {
    id: SpeechId,
    child: Arc<Mutex<Child>>,
}

/// What `spd-say -L` answered.
#[derive(Debug)]
enum Listed {
    /// The voices of the current output module.
    Voices(Vec<VoiceInfo>),
    /// The program could not start.
    Missing,
    /// The program failed, hung, or listed nothing.
    Silent,
}

/// The `spd-say` voice.
#[derive(Debug)]
pub(crate) struct SpdSay {
    /// The `spd-say` program, a path or a name looked up on `PATH`.
    program: PathBuf,
    /// The speech last started, until it ends, is cut or is replaced.
    current: Arc<Mutex<Option<Running>>>,
}

impl SpdSay {
    /// A voice that runs `program` as `spd-say`.
    pub(crate) fn new(program: PathBuf) -> Self {
        Self { program, current: Arc::new(Mutex::new(None)) }
    }

    /// The voices `spd-say -L` lists, within [`LIST_TIMEOUT`].
    fn list(&self) -> Listed {
        let spawned = Command::new(&self.program)
            .arg("-L")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = spawned else { return Listed::Missing };
        let Some(stdout) = child.stdout.take() else { return reap_silent(child) };
        let read = Arc::new(Mutex::new(Vec::new()));
        let (eof_tx, eof_rx) = mpsc::channel();
        // Read on its own thread: a full pipe must not block the timeout, and a daemon that
        // inherits the pipe (autospawn) must not hold the listing past the child's exit.
        let sink = Arc::clone(&read);
        let reader = std::thread::Builder::new().name("oku-voice-list".into()).spawn(move || {
            let copied = catch_unwind(AssertUnwindSafe(|| copy_bounded(stdout, &sink)));
            if copied.is_err() {
                tracing::error!("voice list reader panicked");
            }
            eof_tx.send(()).ok();
        });
        if reader.is_err() {
            return reap_silent(child);
        }
        let Some(deadline) = Instant::now().checked_add(LIST_TIMEOUT) else { return reap_silent(child) };
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => break,
                Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL),
                _ => return reap_silent(child),
            }
        }
        // The child exited: what it wrote is in the pipe; EOF follows unless someone else holds it.
        let left = deadline.saturating_duration_since(Instant::now());
        eof_rx.recv_timeout(left.min(EOF_GRACE)).ok();
        let bytes = read.lock().unwrap_or_else(PoisonError::into_inner).clone();
        match parse_voices(&String::from_utf8_lossy(&bytes)) {
            voices if voices.is_empty() => Listed::Silent,
            voices => Listed::Voices(voices),
        }
    }
}

/// Appends what `from` gives to `into`, up to [`MAX_LIST_BYTES`], until EOF or an error.
fn copy_bounded(mut from: impl Read, into: &Mutex<Vec<u8>>) {
    let mut chunk = [0_u8; 4096];
    while let Ok(n) = from.read(&mut chunk) {
        let Some(fresh) = chunk.get(..n).filter(|f| !f.is_empty()) else { return };
        let mut g = into.lock().unwrap_or_else(PoisonError::into_inner);
        let room = usize::try_from(MAX_LIST_BYTES).unwrap_or(usize::MAX).saturating_sub(g.len());
        g.extend_from_slice(fresh.get(..room.min(fresh.len())).unwrap_or_default());
    }
}

/// Kills and reaps a listing child that failed or hung: the daemon is silent.
fn reap_silent(mut child: Child) -> Listed {
    child.kill().ok();
    child.wait().ok();
    Listed::Silent
}

/// The voices of a `spd-say -L` listing: `NAME LANGUAGE VARIANT` under a header line.
///
/// A name may hold spaces: the language and the variant are the last two columns.
fn parse_voices(text: &str) -> Vec<VoiceInfo> {
    text.lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let [name @ .., lang, _variant] = words.as_slice() else { return None };
            if name.is_empty() || (name == ["NAME"] && *lang == "LANGUAGE") {
                return None;
            }
            let name = name.join(" ");
            Some(VoiceInfo { id: name.clone(), name, lang: (*lang).to_owned() })
        })
        .collect()
}

/// The `-r` argument: 0 is the default rate, ±100 twice faster or slower.
fn rate_arg(rate: SpeechRate) -> i32 {
    crate::num::round_i32(rate.get().log2() * 100.0).clamp(-100, 100)
}

/// The unavailable error for a listing that did not give voices.
fn unavailable(reason: UnavailableReason) -> SttError {
    VoiceUnavailableSnafu { reason }.build().into()
}

/// Waits for speech `id`'s child to exit, then clears it from `current` and calls DONE.
fn wait(child: &Mutex<Child>, current: &Mutex<Option<Running>>, id: SpeechId, done: SpeechDone) {
    loop {
        let exited = !matches!(child.lock().unwrap_or_else(PoisonError::into_inner).try_wait(), Ok(None));
        if exited {
            break;
        }
        std::thread::sleep(POLL);
    }
    {
        let mut g = current.lock().unwrap_or_else(PoisonError::into_inner);
        if g.as_ref().is_some_and(|r| r.id == id) {
            *g = None;
        }
    }
    done.call(id);
}

/// Kills a child we started; one that already exited is left to its waiter.
fn kill(running: &Running) {
    running.child.lock().unwrap_or_else(PoisonError::into_inner).kill().ok();
}

impl SpeechEngine for SpdSay {
    fn status(&self, lang: &str) -> SpeechStatus {
        match self.list() {
            Listed::Voices(voices) => for_language(lang, voices),
            Listed::Missing => SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine },
            Listed::Silent => SpeechStatus::Unavailable { reason: UnavailableReason::DaemonSilent },
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
        // The voice comes from `status`, already listed. A voice removed since is unverified:
        // speech-dispatcher may fall back to its default voice (t-1490).
        let voice = chosen(voice)?;
        let mut command = Command::new(&self.program);
        command.args(["-w", "-N", CLIENT, "-y", voice, "-r", &rate_arg(rate).to_string(), "--"]);
        // A NUL cannot pass in an argument: it becomes a space, as on macOS, so all is spoken.
        command.arg(text.replace('\0', " "));
        let child = command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
        let child = match child {
            Ok(child) => Arc::new(Mutex::new(child)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(unavailable(UnavailableReason::NoEngine)),
            Err(e) => return Err(VoiceSpawnSnafu.into_error(e).into()),
        };
        let previous = self
            .current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(Running { id, child: Arc::clone(&child) });
        if let Some(previous) = previous {
            kill(&previous);
        }
        let current = Arc::clone(&self.current);
        let waiter = Arc::clone(&child);
        let spawned = std::thread::Builder::new().name("oku-voice".into()).spawn(move || {
            if catch_unwind(AssertUnwindSafe(|| wait(&waiter, &current, id, done))).is_err() {
                tracing::error!(id = id.get(), "voice waiter panicked");
            }
        });
        if let Err(e) = spawned {
            // No waiter: the speech is cut, and `done` was dropped with the closure.
            if let Some(running) = self.take(id) {
                kill(&running);
            }
            child.lock().unwrap_or_else(PoisonError::into_inner).wait().ok();
            return Err(VoiceSpawnSnafu.into_error(e).into());
        }
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        if let Some(running) = self.take(id) {
            kill(&running);
        }
        Ok(())
    }
}

impl SpdSay {
    /// Takes speech `id` out of `current` if it is still the one there.
    fn take(&self, id: SpeechId) -> Option<Running> {
        let mut g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if g.as_ref().is_some_and(|r| r.id == id) { g.take() } else { None }
    }
}

impl Drop for SpdSay {
    fn drop(&mut self) {
        // Speech outlives no engine: its child is killed, its waiter still calls DONE.
        let running = self.current.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(running) = running {
            kill(&running);
        }
    }
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod linux_tests;
