//! `spd-say` replaced by shell scripts that record their argv: any Unix host.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

/// What the fake `spd-say -L` prints: a header, then `NAME LANGUAGE VARIANT`.
const VOICES: &str = "     NAME                   LANGUAGE   VARIANT\n     english                en         none\n     French Male 1          fr-FR      none\n";

/// How long a test waits for a DONE that must come.
const DONE_WITHIN: Duration = Duration::from_secs(10);

/// A fake `spd-say` in a fresh directory: `-L` prints `listing`, otherwise `speech` runs after
/// the argv is appended to `argv.log`, one argument per line, then `---`.
struct Fake {
    dir: tempfile::TempDir,
    program: PathBuf,
}

impl Fake {
    fn new(listing: &str, speech: &str) -> Self {
        let dir = crate::test_support::fresh_dir("spd-say");
        let program = dir.path().join("spd-say");
        let log = dir.path().join("argv.log");
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"-L\" ]; then\n{listing}\nfi\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> '{log}'; done\nprintf -- '---\\n' >> '{log}'\n{speech}\n",
            log = log.display()
        );
        std::fs::write(&program, script).unwrap_or_else(|e| panic!("fake spd-say: {e}"));
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| panic!("fake spd-say mode: {e}"));
        Self { dir, program }
    }

    /// Lists [`VOICES`], and each speech ends at once.
    fn quick() -> Self {
        Self::new(&format!("printf '%s' '{VOICES}'; exit 0"), "exit 0")
    }

    fn engine(&self) -> SpdSay {
        SpdSay::new(self.program.clone())
    }

    /// The argv of each speech, in order.
    fn speeches(&self) -> Vec<Vec<String>> {
        let log = std::fs::read_to_string(self.dir.path().join("argv.log")).unwrap_or_default();
        log.split_terminator("---\n").map(|one| one.lines().map(str::to_owned).collect()).collect()
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

fn speak_and_wait(engine: &SpdSay, id: u64, text: &str, voice: Option<&str>, r: f32) {
    let (done, rx) = reporting();
    engine.speak(SpeechId::new(id), text, voice, rate(r), done).unwrap_or_else(|e| panic!("speak: {e}"));
    assert_that!(rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(id)));
}

fn missing(dir: &Path) -> SpdSay {
    SpdSay::new(dir.join("no-spd-say"))
}

#[test]
fn linux_missing_program_is_unavailable_localized() {
    let dir = crate::test_support::fresh_dir("spd-say-missing");
    let engine = missing(dir.path());
    assert_that!(engine.status("fr")).is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine });
    let (done, rx) = reporting();
    let refused = engine
        .speak(SpeechId::new(1), "bonjour", Some("english"), SpeechRate::default(), done)
        .err()
        .unwrap_or_else(|| panic!("a missing spd-say spoke"));
    assert_that!(refused.kind()).is_equal_to(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoEngine });
    let en = Localizer::new(Some("en"));
    assert_that!(refused.localize(&en)).is_equal_to(en.tr("stt-voice-no-engine"));
    assert_that!(refused.localize(&Localizer::new(Some("fr")))).contains("Aucune voix");
    // On Err, DONE was dropped uncalled.
    assert_that!(rx.recv_timeout(Duration::from_millis(50))).is_equal_to(Err(mpsc::RecvTimeoutError::Disconnected));
}

#[test]
fn linux_spd_say_argv_has_wait_double_dash_and_client_name() {
    let fake = Fake::quick();
    speak_and_wait(&fake.engine(), 1, "bonjour", Some("French Male 1"), 2.0);
    let expected = ["-w", "-N", "oku-slides", "-y", "French Male 1", "-r", "100", "--", "bonjour"];
    assert_that!(fake.speeches()).is_equal_to(vec![expected.map(str::to_owned).to_vec()]);
    assert_that!(fake.speeches().concat().contains(&"-C".to_owned())).is_false();
}

#[test]
fn linux_text_starting_with_dash_is_spoken_not_parsed() {
    let fake = Fake::quick();
    speak_and_wait(&fake.engine(), 2, "-C --help", Some("english"), 0.5);
    let expected = ["-w", "-N", "oku-slides", "-y", "english", "-r", "-100", "--", "-C --help"];
    assert_that!(fake.speeches()).is_equal_to(vec![expected.map(str::to_owned).to_vec()]);
}

#[test]
fn linux_stop_kills_own_child_only() {
    let fake = Fake::new(&format!("printf '%s' '{VOICES}'; exit 0"), "exec sleep 30");
    let engine = fake.engine();
    let mut bystander = Command::new("sleep").arg("30").spawn().unwrap_or_else(|e| panic!("bystander: {e}"));
    let (done, rx) = reporting();
    engine
        .speak(SpeechId::new(1), "long", Some("english"), SpeechRate::default(), done)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    engine.stop(SpeechId::new(2)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(rx.recv_timeout(Duration::from_millis(300))).is_equal_to(Err(mpsc::RecvTimeoutError::Timeout));
    engine.stop(SpeechId::new(1)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
    assert_that!(bystander.try_wait().is_ok_and(|s| s.is_none())).is_true();
    bystander.kill().ok();
    bystander.wait().ok();
}

#[test]
fn linux_new_speech_cuts_previous() {
    let fake = Fake::new(&format!("printf '%s' '{VOICES}'; exit 0"), "exec sleep 30");
    let engine = fake.engine();
    let (first, first_rx) = reporting();
    let (second, second_rx) = reporting();
    engine
        .speak(SpeechId::new(1), "one", Some("english"), SpeechRate::default(), first)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    engine
        .speak(SpeechId::new(2), "two", Some("english"), SpeechRate::default(), second)
        .unwrap_or_else(|e| panic!("speak: {e}"));
    assert_that!(first_rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(1)));
    engine.stop(SpeechId::new(2)).unwrap_or_else(|e| panic!("stop: {e}"));
    assert_that!(second_rx.recv_timeout(DONE_WITHIN)).is_equal_to(Ok(SpeechId::new(2)));
}

#[test]
fn linux_status_lists_voices_of_language() {
    let fake = Fake::quick();
    let french = VoiceInfo { id: "French Male 1".into(), name: "French Male 1".into(), lang: "fr-FR".into() };
    assert_that!(fake.engine().status("fr")).is_equal_to(SpeechStatus::Ready { voices: vec![french] });
    assert_that!(fake.engine().status("de"))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage });
}

#[rstest::rstest]
#[case::failing("exit 1")]
#[case::empty("printf '     NAME   LANGUAGE   VARIANT\\n'; exit 0")]
fn linux_silent_daemon_is_unavailable(#[case] listing: &str) {
    let fake = Fake::new(listing, "exit 0");
    assert_that!(fake.engine().status("fr"))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::DaemonSilent });
}

#[test]
fn linux_speak_trusts_the_resolved_voice_without_listing() {
    // The listing fails: `speak` must not list again, the speaker resolved the voice.
    let fake = Fake::new("exit 1", "exit 0");
    speak_and_wait(&fake.engine(), 1, "x", Some("english"), 1.0);
    let expected = ["-w", "-N", "oku-slides", "-y", "english", "-r", "0", "--", "x"];
    assert_that!(fake.speeches()).is_equal_to(vec![expected.map(str::to_owned).to_vec()]);
}

#[test]
fn linux_nul_in_text_is_spoken_as_space() {
    let fake = Fake::quick();
    speak_and_wait(&fake.engine(), 3, "un\0deux", Some("english"), 1.0);
    let expected = ["-w", "-N", "oku-slides", "-y", "english", "-r", "0", "--", "un deux"];
    assert_that!(fake.speeches()).is_equal_to(vec![expected.map(str::to_owned).to_vec()]);
}

#[test]
fn linux_speak_without_voice_is_refused() {
    let fake = Fake::quick();
    let (done, _rx) = reporting();
    let refused = fake.engine().speak(SpeechId::new(1), "bonjour", None, SpeechRate::default(), done);
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoVoiceForLanguage }));
    assert_that!(fake.speeches()).is_empty();
}

#[test]
fn linux_daemon_holding_list_pipe_still_lists() {
    // An autospawned daemon that inherits stdout keeps the pipe open after `-L` exits.
    let fake = Fake::new(&format!("printf '%s' '{VOICES}'; sleep 5 & exit 0"), "exit 0");
    let started = std::time::Instant::now();
    let status = fake.engine().status("fr");
    assert_that!(matches!(status, SpeechStatus::Ready { .. })).is_true();
    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(2));
}
