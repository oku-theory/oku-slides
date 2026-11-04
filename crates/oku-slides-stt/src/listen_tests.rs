use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use crate::dictation_whisper::Job;
use crate::stream::Availability;
use crate::test_support::{FinalsAt, Part, ScriptedEngine, french_platform, fresh_dir, listening_stt, wait_until};
use crate::utterance::Spoken;
use crate::{Config, State, Stt, SttErrorKind, Utterance};

/// Generous: the fake microphone plays in real time, and the machine may be loaded.
const WITHIN: Duration = Duration::from_secs(40);

fn next(rx: &Receiver<Utterance>, what: &str) -> Utterance {
    rx.recv_timeout(WITHIN).unwrap_or_else(|e| panic!("{what}: {e}"))
}

/// Waits until the fake microphone has played `secs` of listening.
fn listened(stt: &Stt, secs: f32) {
    wait_until(stt, "écoute", |s| s.state != State::Listening || s.seconds >= secs);
}

#[test]
fn listen_two_phrases_with_silence_closes_two_utterances() {
    let dir = fresh_dir("listen-two");
    let parts = [Part::Silence(0.5), Part::Speech, Part::Silence(3.0), Part::Speech];
    let (stt, _engine) = listening_stt(dir.path(), &parts, 1.5);
    let rx = stt.utterances().unwrap();
    stt.listen(Vec::new());
    let (first, second) = (next(&rx, "first"), next(&rx, "second"));
    assert_that!(second.seq).is_greater_than(first.seq);
    assert_that!(first.text.is_empty() || second.text.is_empty()).is_false();
    assert_that!(first.ended).is_less_than(second.started);
    assert_that!(first.speech_secs).is_greater_than(1.0);
    assert_that!(stt.status().state).is_equal_to(State::Listening);
}

#[test]
fn listen_long_silence_after_utterance_closes_once() {
    let dir = fresh_dir("listen-once");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Speech, Part::Silence(6.0), Part::Speech], 1.5);
    let rx = stt.utterances().unwrap();
    stt.listen(Vec::new());
    let first = next(&rx, "the utterance");
    // Sentinel: the next utterance is the second speech, after the long silence closed nothing.
    let second = next(&rx, "the second speech");
    assert_that!(second.started.duration_since(first.ended)).is_greater_than_or_equal_to(Duration::from_secs(5));
}

#[test]
fn listen_phrase_after_flush_belongs_to_next_utterance() {
    let dir = fresh_dir("listen-flush");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    let rx = stt.utterances().unwrap();
    let (session, epoch) = {
        let mut g = stt.state();
        g.phrases.push(Spoken { text: None, start: 0, end: 16_000 });
        g.phrases.push(Spoken { text: None, start: 32_000, end: 48_000 });
        (g.session, g.epoch)
    };
    let (jobs, queue) = mpsc::channel();
    jobs.send(Job::Final { index: 0, audio: vec![0.0; 16_000], epoch }).unwrap();
    jobs.send(Job::Flush { upto: 1, epoch }).unwrap();
    jobs.send(Job::Final { index: 1, audio: vec![0.0; 16_000], epoch }).unwrap();
    drop(jobs);
    stt.transcribe_loop(session, &queue, &Arc::new(AtomicBool::new(false)));
    assert_that!(rx.try_recv().unwrap().text).is_equal_to("mot0.".to_owned());
    assert_that!(stt.state().phrases.committed()).is_equal_to("mot1.".to_owned());
}

#[test]
fn listen_late_final_below_base_is_dropped() {
    let dir = fresh_dir("listen-late");
    let (stt, engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    let rx = stt.utterances().unwrap();
    let (session, epoch) = {
        let mut g = stt.state();
        g.phrases.push(Spoken { text: None, start: 0, end: 16_000 });
        g.phrases.discard(); // same epoch: only the base protects the slot
        (g.session, g.epoch)
    };
    let (jobs, queue) = mpsc::channel();
    jobs.send(Job::Final { index: 0, audio: vec![0.0; 16_000], epoch }).unwrap();
    jobs.send(Job::Flush { upto: 1, epoch }).unwrap();
    drop(jobs);
    stt.transcribe_loop(session, &queue, &Arc::new(AtomicBool::new(false)));
    assert_that!(engine.calls.load(std::sync::atomic::Ordering::Relaxed)).is_equal_to(1);
    assert_that!(rx.try_recv().is_err()).is_true();
    assert_that!(stt.state().phrases.is_empty()).is_true();
}

#[test]
fn listen_suspend_discards_speech_in_progress() {
    let dir = fresh_dir("listen-suspend");
    let parts = [Part::Silence(0.5), Part::Speech, Part::Silence(3.0), Part::Speech];
    let (stt, _engine) = listening_stt(dir.path(), &parts, 1.5);
    let rx = stt.utterances().unwrap();
    stt.listen(Vec::new());
    wait_until(&stt, "voix", |s| s.speaking);
    let guard = stt.suspend();
    assert_that!(stt.status().suspended).is_true();
    // Held through the end of the first speech, into the silence after it.
    listened(&stt, 7.5);
    drop(guard);
    let resumed = Instant::now();
    let heard = next(&rx, "speech after the resume");
    assert_that!(heard.started).is_greater_than_or_equal_to(resumed);
    assert_that!(stt.status().suspended).is_false();
}

#[test]
fn listen_stop_discards_utterance_in_progress() {
    let dir = fresh_dir("listen-stop");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(0.5), Part::Speech], 1.5);
    let rx = stt.utterances().unwrap();
    stt.listen(Vec::new());
    wait_until(&stt, "voix", |s| s.speaking);
    stt.stop();
    assert_that!(stt.status().state).is_equal_to(State::Idle);
    // Sentinel: listening again replays the speech; the first utterance is the new one.
    let restarted = Instant::now();
    stt.listen(Vec::new());
    assert_that!(next(&rx, "sentinel").started).is_greater_than_or_equal_to(restarted);
}

#[test]
fn listen_start_while_listening_is_refused() {
    let dir = fresh_dir("listen-start");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    stt.listen(Vec::new());
    let session = stt.state().session;
    stt.start(Vec::new());
    stt.listen(Vec::new());
    assert_that!(stt.status().state).is_equal_to(State::Listening);
    assert_that!(stt.state().session).is_equal_to(session);
    stt.cancel();
}

#[test]
fn listen_configure_while_listening_restarts_capture() {
    let dir = fresh_dir("listen-configure");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    stt.listen(Vec::new());
    let session = stt.state().session;
    assert_that!(stt.configure(Config { silence: 2.0, ..stt.status().config })).is_true();
    let s = stt.status();
    assert_that!((s.state, s.config.silence)).is_equal_to((State::Listening, 2.0));
    assert_that!(stt.state().session).is_greater_than(session);
    stt.cancel();
}

#[test]
fn listen_configure_restart_keeps_utterance_sender() {
    let dir = fresh_dir("listen-sender");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(0.5), Part::Speech], 1.5);
    let rx = stt.utterances().unwrap();
    stt.listen(Vec::new());
    assert_that!(stt.configure(Config { silence: 1.0, ..stt.status().config })).is_true();
    next(&rx, "utterance after the restart");
}

#[test]
fn listen_download_failure_while_listening_resumes_listening() {
    let dir = fresh_dir("listen-download");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    stt.listen(Vec::new());
    // The hub is a refused local port: the download fails at once.
    assert_that!(stt.download(crate::Model::Large, false)).is_true();
    let s = wait_until(&stt, "listening again", |s| s.state == State::Listening && s.error.is_some());
    assert_that!(s.error).is_some();
    stt.cancel();
}

#[test]
fn listen_download_cancel_while_listening_resumes_listening() {
    let dir = fresh_dir("listen-download-cancel");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    stt.listen(Vec::new());
    assert_that!(stt.download(crate::Model::Large, false)).is_true();
    stt.cancel();
    wait_until(&stt, "listening again", |s| s.state == State::Listening);
    stt.cancel();
}

#[test]
fn apple_dictation_after_suspended_listening_keeps_its_text() {
    let dir = fresh_dir("listen-apple-dictation");
    let engine = ScriptedEngine::new(Availability::Ready, &["Un."]);
    let pushed = engine.pushed.clone();
    let stt = Stt::with_engine(&french_platform(dir.path(), |_| {}), Arc::new(engine), None);
    stt.listen(Vec::new());
    wait_until(&stt, "audio pushed", |_| pushed.load(std::sync::atomic::Ordering::Relaxed) > 0);
    drop(stt.suspend());
    wait_until(&stt, "clock rebased", |_| stt.state().clock.stream_base > 0);
    stt.stop();
    stt.start(Vec::new());
    stt.stop();
    let s = wait_until(&stt, "dictation done", |s| s.state == State::Idle && s.seq > 0);
    assert_that!(s.text).is_equal_to(Some("Un.".to_owned()));
}

#[test]
fn utterances_second_call_is_refused() {
    let dir = fresh_dir("listen-receiver");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    let _rx = stt.utterances().unwrap();
    let e = stt.utterances().unwrap_err();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::ReceiverTaken);
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("stt-receiver-taken"));
    assert_that!(e.localize(&fr)).is_equal_to("L'écoute mains libres est déjà reliée ailleurs".to_owned());
}

#[test]
fn suspend_nested_guards_resume_after_last_drop() {
    let dir = fresh_dir("listen-nested");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    let (outer, inner) = (stt.suspend(), stt.suspend());
    drop(inner);
    assert_that!(stt.status().suspended).is_true();
    drop(outer);
    assert_that!(stt.status().suspended).is_false();
}

#[test]
fn listen_status_never_carries_utterance_text() {
    let dir = fresh_dir("listen-status");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    stt.listen(Vec::new());
    {
        let mut g = stt.state();
        g.phrases.push(Spoken { text: Some("secret".into()), start: 0, end: 16_000 });
        g.partial = "secret".into();
    }
    let s = stt.status();
    assert_that!((s.committed, s.partial)).is_equal_to((String::new(), String::new()));
    stt.cancel();
}

#[test]
fn apple_finals_after_silence_close_one_utterance() {
    let dir = fresh_dir("listen-apple");
    let mut engine = ScriptedEngine::new(Availability::Ready, &["Un.", "Deux."]);
    engine.finals_at = FinalsAt::FirstPush;
    let stt = Stt::with_engine(&french_platform(dir.path(), |_| {}), Arc::new(engine), None);
    assert_that!(stt.configure(Config { silence: 0.5, ..stt.status().config })).is_true();
    let rx = stt.utterances().unwrap();
    stt.listen(Vec::new());
    let heard = next(&rx, "the Apple utterance");
    assert_that!(heard.text).is_equal_to("Un. Deux.".to_owned());
    // Both finals in one utterance: the silence closed once, after them.
    assert_that!(heard.speech_secs).is_equal_to(2.0);
    assert_that!(stt.status().state).is_equal_to(State::Listening);
}

/// The listening tests rely on it: one speech part is one utterance at a 1.5 s silence.
#[test]
fn speech_fixture_gaps_between_phrases_are_below_the_test_silence() {
    let dir = fresh_dir("listen-fixture");
    let wav = crate::test_support::speech_wav(dir.path(), &[Part::Speech]);
    let audio: Vec<f32> =
        hound::WavReader::open(wav).unwrap().samples::<i16>().map(|s| f32::from(s.unwrap()) / 32_768.0).collect();
    let vad = oku_slides_platform::config::Settings::defaults().unwrap().stt.vad;
    let phrases = crate::vad::split(&audio, vad);
    let gaps: Vec<f32> = phrases.windows(2).map(|w| crate::num::seconds(w[1].start.saturating_sub(w[0].end))).collect();
    assert_that!(phrases).has_length(1);
    assert_that!(gaps.iter().all(|g| *g < 1.5)).is_true();
}
