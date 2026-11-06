//! Speech Dispatcher replaced by a fake SSIP daemon on a Unix socket: any Unix host.
#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

/// What the fake daemon lists for `LIST SYNTHESIS_VOICES`.
const VOICES: &str = "249-english\ten\tnone\r\n249-French Male 1\tfr-FR\tnone\r\n249 OK VOICE LIST SENT\r\n";

/// How long a test waits for a DONE that must come.
const DONE_WITHIN: Duration = Duration::from_secs(10);

/// How the fake daemon treats a queued message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Speech {
    /// Speaks it at once: `702 END`.
    Ends,
    /// Speaks until `CANCEL self`: `703 CANCELED`.
    UntilCancel,
    /// Never ends, and answers `CANCEL` without any event.
    Deaf,
}

/// The fake daemon's behaviour.
#[derive(Debug, Clone)]
struct Script {
    speech: Speech,
    /// The reply to `LIST SYNTHESIS_VOICES`.
    voices: String,
    /// The reply to `SET self SYNTHESIS_VOICE`.
    voice_reply: &'static str,
}

impl Default for Script {
    fn default() -> Self {
        Self { speech: Speech::Ends, voices: VOICES.to_owned(), voice_reply: "209 OK VOICE SET" }
    }
}

/// A fake Speech Dispatcher listening in a fresh directory; each connection's input is kept.
struct Daemon {
    dir: tempfile::TempDir,
    socket: PathBuf,
    transcripts: Arc<Mutex<Vec<Arc<Mutex<String>>>>>,
}

impl Daemon {
    fn new(script: Script) -> Self {
        let daemon = Self::unbound();
        daemon.listen(script);
        daemon
    }

    /// A daemon whose socket does not exist yet.
    fn unbound() -> Self {
        let dir = crate::test_support::fresh_dir("speechd");
        let socket = dir.path().join("speechd.sock");
        Self { dir, socket, transcripts: Arc::default() }
    }

    fn listen(&self, script: Script) {
        let listener = UnixListener::bind(&self.socket).unwrap_or_else(|e| panic!("bind fake daemon: {e}"));
        let transcripts = Arc::clone(&self.transcripts);
        let ids = Arc::new(AtomicU64::new(1));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let transcript = Arc::new(Mutex::new(String::new()));
                transcripts.lock().unwrap_or_else(PoisonError::into_inner).push(Arc::clone(&transcript));
                let (script, ids) = (script.clone(), Arc::clone(&ids));
                std::thread::spawn(move || serve(&stream, &script, &ids, &transcript));
            }
        });
    }

    fn engine(&self) -> SpeechDispatcher {
        SpeechDispatcher::new(Some(self.socket.clone()), self.dir.path().join("no-speech-dispatcher"), "joe".into())
    }

    /// What each connection sent, in connection order.
    fn transcripts(&self) -> Vec<String> {
        let all = self.transcripts.lock().unwrap_or_else(PoisonError::into_inner);
        all.iter().map(|t| t.lock().unwrap_or_else(PoisonError::into_inner).clone()).collect()
    }
}

/// Answers one client's commands as `script` says, recording what it sent.
fn serve(stream: &UnixStream, script: &Script, ids: &AtomicU64, transcript: &Mutex<String>) {
    let mut reader = BufReader::new(stream);
    let mut out = stream;
    let mut queued = None;
    let mut line = String::new();
    let mut reply = |text: &str| out.write_all(text.as_bytes()).ok();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        transcript.lock().unwrap_or_else(PoisonError::into_inner).push_str(&line);
        let command = line.trim_end();
        if command.starts_with("SET self SYNTHESIS_VOICE") {
            reply(&format!("{}\r\n", script.voice_reply));
        } else if command.starts_with("SET") {
            reply("200 OK SET\r\n");
        } else if command == "LIST SYNTHESIS_VOICES" {
            reply(&script.voices);
        } else if command == "SPEAK" {
            reply("230 OK RECEIVING DATA\r\n");
            let mut body = String::new();
            while line != ".\r\n" {
                line.clear();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                transcript.lock().unwrap_or_else(PoisonError::into_inner).push_str(&line);
                body.push_str(&line);
            }
            // As speechd's server: only a data part that is a lone `.` line queues nothing.
            if body == ".\r\n" {
                reply("227 OK MESSAGE CANCELED\r\n");
                continue;
            }
            let id = ids.fetch_add(1, Ordering::SeqCst);
            reply(&format!("225-{id}\r\n225 OK MESSAGE QUEUED\r\n"));
            if script.speech == Speech::Ends {
                reply(&format!("702-{id}\r\n702-1\r\n702 END\r\n"));
            } else {
                queued = Some(id);
            }
        } else if command == "CANCEL self" {
            reply("213 OK CANCELED\r\n");
            if let Some(id) = queued.take().filter(|_| script.speech == Speech::UntilCancel) {
                reply(&format!("703-{id}\r\n703-1\r\n703 CANCELED\r\n"));
            }
        } else if command == "QUIT" {
            reply("231 HAPPY HACKING\r\n");
            return;
        } else {
            reply("500 ERR INVALID COMMAND\r\n");
        }
    }
}

/// A DONE that reports its id on a channel.
fn reporting() -> (SpeechDone, mpsc::Receiver<SpeechId>) {
    let (tx, rx) = mpsc::channel();
    (SpeechDone::new(move |id| tx.send(id).unwrap_or_else(|e| panic!("done receiver gone: {e}"))), rx)
}

fn rate(r: f32) -> SpeechRate {
    SpeechRate::new(r).unwrap_or_else(|e| panic!("rate {r}: {e}"))
}

fn speak_and_wait(engine: &SpeechDispatcher, id: u64, text: &str, voice: Option<&str>, r: f32) {
    let (done, rx) = reporting();
    engine.speak(SpeechId::new(id), text, voice, rate(r), done).unwrap_or_else(|e| panic!("speak: {e}"));
    assert_that!(rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(id)));
}

/// Waits until the daemon saw `needle` on connection `n`.
fn wait_for(daemon: &Daemon, n: usize, needle: &str) {
    let deadline = std::time::Instant::now() + DONE_WITHIN;
    while !daemon.transcripts().get(n).is_some_and(|t| t.contains(needle)) {
        assert_that!(std::time::Instant::now() < deadline).is_true();
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// An executable shell script at `path`.
fn script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap_or_else(|e| panic!("script: {e}"));
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|e| panic!("script mode: {e}"));
}

#[test]
fn linux_speak_sends_voice_rate_and_text_over_the_socket() {
    let daemon = Daemon::new(Script::default());
    speak_and_wait(&daemon.engine(), 1, "bonjour", Some("French Male 1"), 2.0);
    wait_for(&daemon, 0, "QUIT");
    assert_that!(daemon.transcripts()).is_equal_to(vec![
        "SET self CLIENT_NAME joe:oku-slides:voice\r\nSET self NOTIFICATION END on\r\n\
         SET self NOTIFICATION CANCEL on\r\nSET self SYNTHESIS_VOICE French Male 1\r\nSET self RATE 100\r\n\
         SPEAK\r\nbonjour\r\n.\r\nQUIT\r\n"
            .to_owned(),
    ]);
}

#[test]
fn linux_slow_rate_is_minus_100() {
    let daemon = Daemon::new(Script::default());
    speak_and_wait(&daemon.engine(), 1, "x", Some("english"), 0.5);
    assert_that!(daemon.transcripts().concat().contains("SET self RATE -100\r\n")).is_true();
}

#[test]
fn linux_stop_cancels_own_speech_only() {
    let daemon = Daemon::new(Script { speech: Speech::UntilCancel, ..Script::default() });
    let engine = daemon.engine();
    let (done, rx) = reporting();
    engine
        .speak(SpeechId::new(1), "long", Some("english"), SpeechRate::default(), done)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    engine.stop(SpeechId::new(2)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(rx.recv_timeout(Duration::from_millis(300))).is_equal_to(Err(mpsc::RecvTimeoutError::Timeout));
    assert_that!(daemon.transcripts().concat().contains("CANCEL self")).is_false();
    engine.stop(SpeechId::new(1)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
    let sent = daemon.transcripts().concat();
    assert_that!(sent.contains("CANCEL self\r\n")).is_true();
    assert_that!(sent.contains("CANCEL all")).is_false();
}

#[test]
fn linux_new_speech_cuts_previous() {
    let daemon = Daemon::new(Script { speech: Speech::UntilCancel, ..Script::default() });
    let engine = daemon.engine();
    let (first, first_rx) = reporting();
    let (second, second_rx) = reporting();
    engine
        .speak(SpeechId::new(1), "one", Some("english"), SpeechRate::default(), first)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    engine
        .speak(SpeechId::new(2), "two", Some("english"), SpeechRate::default(), second)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    assert_that!(first_rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
    let transcripts = daemon.transcripts();
    assert_that!(transcripts.first().is_some_and(|t| t.contains("CANCEL self\r\n"))).is_true();
    assert_that!(transcripts.get(1).is_some_and(|t| t.contains("CANCEL self"))).is_false();
    engine.stop(SpeechId::new(2)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(second_rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(2)));
}

#[test]
fn linux_cut_unconfirmed_by_daemon_still_calls_done() {
    let daemon = Daemon::new(Script { speech: Speech::Deaf, ..Script::default() });
    let engine = daemon.engine();
    let (done, rx) = reporting();
    engine
        .speak(SpeechId::new(1), "long", Some("english"), SpeechRate::default(), done)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    engine.stop(SpeechId::new(1)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
}

#[test]
fn linux_dropped_engine_cancels_its_speech() {
    let daemon = Daemon::new(Script { speech: Speech::UntilCancel, ..Script::default() });
    let (done, rx) = reporting();
    daemon
        .engine()
        .speak(SpeechId::new(1), "long", Some("english"), SpeechRate::default(), done)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    assert_that!(rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
    assert_that!(daemon.transcripts().concat().contains("CANCEL self\r\n")).is_true();
}

#[rstest::rstest]
#[case::empty("")]
#[case::blank(" \n\t")]
#[case::nul_and_space("\0 \0")]
fn linux_silent_text_cuts_previous_and_is_done_without_daemon(#[case] text: &str) {
    let daemon = Daemon::new(Script { speech: Speech::UntilCancel, ..Script::default() });
    let engine = daemon.engine();
    let (first, first_rx) = reporting();
    engine
        .speak(SpeechId::new(1), "long", Some("english"), SpeechRate::default(), first)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    let (second, second_rx) = reporting();
    engine
        .speak(SpeechId::new(2), text, Some("english"), SpeechRate::default(), second)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    // DONE ran before `speak` returned, with no connection of its own.
    assert_that!(second_rx.try_recv()).is_equal_to(Ok(SpeechId::new(2)));
    assert_that!(first_rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
    assert_that!(daemon.transcripts().len()).is_equal_to(1);
    assert_that!(daemon.transcripts().concat().contains("CANCEL self\r\n")).is_true();
}

#[test]
fn linux_status_lists_voices_of_language() {
    let daemon = Daemon::new(Script::default());
    let french = VoiceInfo { id: "French Male 1".into(), name: "French Male 1".into(), lang: "fr-FR".into() };
    assert_that!(daemon.engine().status("fr")).is_equal_to(SpeechStatus::Ready { voices: vec![french] });
    assert_that!(daemon.engine().status("de"))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage });
}

#[rstest::rstest]
#[case::refused("300 ERR CANT LIST VOICES\r\n")]
#[case::empty("249 OK VOICE LIST SENT\r\n")]
fn linux_silent_daemon_is_unavailable(#[case] voices: &str) {
    let daemon = Daemon::new(Script { voices: voices.to_owned(), ..Script::default() });
    assert_that!(daemon.engine().status("fr"))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::DaemonSilent });
}

#[test]
fn linux_missing_daemon_and_program_is_unavailable_localized() {
    let daemon = Daemon::unbound();
    let engine = daemon.engine();
    assert_that!(engine.status("fr")).is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine });
    let (done, rx) = reporting();
    let refused = engine
        .speak(SpeechId::new(1), "bonjour", Some("english"), SpeechRate::default(), done)
        .err()
        .unwrap_or_else(|| panic!("a missing daemon spoke"));
    assert_that!(refused.kind()).is_equal_to(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoEngine });
    let en = Localizer::new(Some("en"));
    assert_that!(refused.localize(&en)).is_equal_to(en.tr("stt-voice-no-engine"));
    assert_that!(refused.localize(&Localizer::new(Some("fr")))).contains("Aucune voix");
    // On Err, DONE was dropped uncalled.
    assert_that!(rx.recv_timeout(Duration::from_millis(50))).is_equal_to(Err(mpsc::RecvTimeoutError::Disconnected));
}

#[test]
fn linux_failed_autospawn_is_daemon_silent() {
    let daemon = Daemon::unbound();
    let spawner = daemon.dir.path().join("speech-dispatcher");
    script(&spawner, "exit 1");
    let engine = SpeechDispatcher::new(Some(daemon.socket.clone()), spawner, "joe".into());
    assert_that!(engine.status("fr"))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::DaemonSilent });
}

#[test]
fn linux_absent_socket_autospawns_daemon_without_text_in_argv() {
    let daemon = Daemon::unbound();
    let spawner = daemon.dir.path().join("speech-dispatcher");
    let argv = daemon.dir.path().join("argv.log");
    // The fake spawner records its argv, then returns once the test's daemon listens.
    script(
        &spawner,
        &format!(
            "printf '%s\\n' \"$@\" > '{}'\nwhile [ ! -S '{}' ]; do sleep 0.05; done",
            argv.display(),
            daemon.socket.display()
        ),
    );
    let engine = SpeechDispatcher::new(Some(daemon.socket.clone()), spawner, "joe".into());
    let (done, rx) = reporting();
    let speaking = std::thread::spawn(move || {
        let spoke = engine.speak(SpeechId::new(1), "secret words", Some("english"), SpeechRate::default(), done);
        spoke.map_err(|e| e.kind())
    });
    let deadline = std::time::Instant::now() + DONE_WITHIN;
    while !argv.exists() {
        assert_that!(std::time::Instant::now() < deadline).is_true();
        std::thread::sleep(Duration::from_millis(10));
    }
    daemon.listen(Script::default());
    assert_that!(speaking.join().ok()).is_equal_to(Some(Ok(())));
    assert_that!(rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
    let logged = std::fs::read_to_string(&argv).unwrap_or_default();
    let socket = daemon.socket.display().to_string();
    assert_that!(logged.lines().collect::<Vec<_>>()).is_equal_to(vec![
        "--spawn",
        "--communication-method",
        "unix_socket",
        "--socket-path",
        socket.as_str(),
    ]);
    assert_that!(daemon.transcripts().concat().contains("secret words\r\n")).is_true();
}

#[test]
fn linux_speak_without_voice_is_refused_unconnected() {
    let daemon = Daemon::new(Script::default());
    let (done, _rx) = reporting();
    let refused = daemon.engine().speak(SpeechId::new(1), "bonjour", None, SpeechRate::default(), done);
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoVoiceForLanguage }));
    assert_that!(daemon.transcripts()).is_empty();
}

#[test]
fn linux_refused_voice_is_voice_failed() {
    let daemon = Daemon::new(Script { voice_reply: "409 ERR COULDNT SET VOICE", ..Script::default() });
    let (done, rx) = reporting();
    let refused = daemon.engine().speak(SpeechId::new(1), "bonjour", Some("gone"), SpeechRate::default(), done);
    assert_that!(refused.map_err(|e| e.kind())).is_equal_to(Err(SttErrorKind::VoiceFailed));
    assert_that!(rx.recv_timeout(Duration::from_millis(50))).is_equal_to(Err(mpsc::RecvTimeoutError::Disconnected));
}

#[rstest::rstest]
#[case::default_runtime(None, Some("/run/user/1000"), Some("/run/user/1000/speech-dispatcher/speechd.sock"))]
#[case::unix_socket_path(Some("unix_socket:/tmp/a:b.sock"), Some("/run/user/1000"), Some("/tmp/a:b.sock"))]
#[case::unix_socket_alone(Some("unix_socket"), Some("/run/x"), Some("/run/x/speech-dispatcher/speechd.sock"))]
#[case::unix_socket_empty(Some("unix_socket:"), Some("/run/x"), Some("/run/x/speech-dispatcher/speechd.sock"))]
#[case::inet_unsupported(Some("inet_socket:127.0.0.1:6560"), Some("/run/x"), None)]
#[case::no_runtime_dir(None, None, None)]
fn linux_socket_path_follows_speechd_address(
    #[case] address: Option<&str>,
    #[case] runtime: Option<&str>,
    #[case] expected: Option<&str>,
) {
    assert_that!(socket_path(address, runtime.map(PathBuf::from))).is_equal_to(expected.map(PathBuf::from));
}
