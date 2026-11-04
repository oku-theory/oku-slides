use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::SttError;
use crate::speak::{SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo};
use crate::test_support::{fresh_dir, platform_in};

/// How long a test waits for the speaker thread: an event, never a sleep.
const WITHIN: Duration = Duration::from_secs(10);

/// A clock the test moves by hand.
#[derive(Debug)]
struct HandClock(Mutex<Instant>);

impl HandClock {
    fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(Instant::now())))
    }

    fn advance(&self, by: Duration) {
        let mut now = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *now = now.checked_add(by).unwrap_or_else(|| panic!("clock overflow"));
    }
}

impl SpeakerClock for HandClock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a [`HandEngine`] was asked, and the DONEs it holds for the test to call.
#[derive(Default)]
struct Calls {
    spoken: Vec<(SpeechId, String, String)>,
    dones: Vec<(SpeechId, SpeechDone)>,
    stopped: Vec<SpeechId>,
    /// How many times the voices were listed.
    listed: usize,
}

/// What the microphone hears of each speech.
type Echo = Box<dyn Fn(&str) + Send + Sync>;

/// A voice whose DONE the test calls by hand, or that calls it inside `speak` (`inline`).
#[derive(Default)]
struct HandEngine {
    calls: Mutex<Calls>,
    inline: bool,
    /// Refuses every speech, as an OS voice that fails to start.
    refuse: bool,
    /// Called with each text it speaks, as a microphone would hear it.
    echo: Option<Echo>,
}

impl fmt::Debug for HandEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HandEngine")
    }
}

impl HandEngine {
    fn calls(&self) -> std::sync::MutexGuard<'_, Calls> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Calls the DONE of speech `id`, as the OS would, late or not.
    fn done(&self, id: u64) {
        let done = {
            let mut calls = self.calls();
            let at = calls.dones.iter().position(|(d, _)| d.get() == id).unwrap_or_else(|| panic!("no DONE {id}"));
            calls.dones.remove(at).1
        };
        done.call(SpeechId::new(id));
    }
}

impl SpeechEngine for HandEngine {
    fn status(&self, lang: &str) -> SpeechStatus {
        self.calls().listed += 1;
        if lang == "fr" {
            SpeechStatus::Ready {
                voices: vec![VoiceInfo { id: "v-fr".into(), name: "Amélie".into(), lang: "fr-FR".into() }],
            }
        } else {
            SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage }
        }
    }

    fn speak(
        &self,
        id: SpeechId,
        text: &str,
        voice: Option<&str>,
        _rate: SpeechRate,
        done: SpeechDone,
    ) -> Result<(), SttError> {
        let voice = voice.unwrap_or_else(|| panic!("the speaker passed no voice")).to_owned();
        if self.refuse {
            return Err(crate::error::VoiceFailedSnafu { step: "speak" }.build().into());
        }
        self.calls().spoken.push((id, text.to_owned(), voice));
        if let Some(echo) = &self.echo {
            echo(text);
        }
        if self.inline {
            done.call(id);
        } else {
            self.calls().dones.push((id, done));
        }
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        self.calls().stopped.push(id);
        Ok(())
    }
}

/// A dictation in French that never opens a microphone: only its suspension is read.
fn stt(dir: &std::path::Path) -> Stt {
    Stt::new(&platform_in(dir, "fr", |s| s.apple = false))
}

fn speech(text: &str) -> Speech {
    Speech { text: text.into(), lang: "fr".into(), voice: None, rate: SpeechRate::default(), ended: None }
}

fn settled(speaker: &Speaker) {
    assert!(speaker.settle(WITHIN), "the speaker thread should answer");
}

#[test]
fn listening_suspended_while_speaking_then_resumed() {
    let dir = fresh_dir("speaker-suspend");
    let stt = stt(dir.path());
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), clock.clone());
    speaker.say(speech("Diapositive ajoutée"));
    settled(&speaker);
    assert_that!(stt.status().suspended).is_true();
    assert_that!(engine.calls().spoken.clone()).is_equal_to(vec![(
        SpeechId::new(1),
        "Diapositive ajoutée".to_owned(),
        "v-fr".to_owned(),
    )]);
    engine.done(1);
    settled(&speaker);
    // DONE came: the guard still holds the microphone for SPEECH_GUARD.
    assert_that!(stt.status().suspended).is_true();
    clock.advance(SPEECH_GUARD);
    speaker.tick();
    settled(&speaker);
    assert_that!(stt.status().suspended).is_false();
}

#[test]
fn utterance_starting_before_speech_end_plus_guard_dropped() {
    let dir = fresh_dir("speaker-deaf");
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt(dir.path())), clock.clone());
    let start = clock.now();
    let before = start.checked_sub(Duration::from_millis(1)).unwrap_or_else(|| panic!("clock underflow"));
    speaker.say(speech("Bonjour"));
    settled(&speaker);
    assert_that!(speaker.hears(start)).is_false();
    clock.advance(Duration::from_secs(1));
    engine.done(1);
    settled(&speaker);
    let at = |ms: u64| start.checked_add(Duration::from_millis(ms)).unwrap_or_else(|| panic!("clock overflow"));
    assert_that!(speaker.hears(before)).is_true();
    assert_that!(speaker.hears(at(1200))).is_false();
    assert_that!(speaker.hears(at(1499))).is_false();
    assert_that!(speaker.hears(at(1500))).is_true();
}

#[test]
fn watchdog_resumes_when_done_never_comes() {
    let dir = fresh_dir("speaker-watchdog");
    let stt = stt(dir.path());
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), clock.clone());
    // 30 characters at rate 1: 2 s, plus 3 s.
    speaker.say(speech(&"a".repeat(30)));
    settled(&speaker);
    clock.advance(Duration::from_millis(4900));
    speaker.tick();
    settled(&speaker);
    assert_that!(speaker.speaking()).is_true();
    clock.advance(Duration::from_millis(100));
    speaker.tick();
    settled(&speaker);
    assert_that!(speaker.speaking()).is_false();
    assert_that!(engine.calls().stopped.clone()).is_equal_to(vec![SpeechId::new(1)]);
    clock.advance(SPEECH_GUARD);
    speaker.tick();
    settled(&speaker);
    assert_that!(stt.status().suspended).is_false();
}

#[rstest]
#[case(150, 1.0, 13_000)]
#[case(150, 2.0, 8_000)]
#[case(150, 0.5, 23_000)]
#[case(0, 1.0, 3_000)]
fn watchdog_shorter_at_higher_rate(#[case] chars: usize, #[case] rate: f32, #[case] millis: u64) {
    let rate = SpeechRate::new(rate).unwrap_or_else(|e| panic!("rate: {e}"));
    assert_that!(watchdog(chars, rate)).is_equal_to(Duration::from_millis(millis));
}

#[test]
fn inline_done_does_not_deadlock() {
    let dir = fresh_dir("speaker-inline");
    let stt = stt(dir.path());
    let engine = Arc::new(HandEngine { inline: true, ..HandEngine::default() });
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), HandClock::new());
    let (tx, rx) = mpsc::channel();
    let ended = SpeechEnded::new(move |_| tx.send(()).unwrap_or_else(|e| panic!("ended: {e}")));
    speaker.say(Speech { ended: Some(ended), ..speech("Terminé") });
    assert_that!(rx.recv_timeout(WITHIN).is_ok()).is_true();
    settled(&speaker);
    assert_that!(speaker.speaking()).is_false();
    assert_that!(stt.status().suspended).is_true();
}

#[test]
fn cut_then_late_done_of_old_speech_is_ignored() {
    let dir = fresh_dir("speaker-late");
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt(dir.path())), clock);
    speaker.say(speech("Première"));
    speaker.say(speech("Seconde"));
    settled(&speaker);
    engine.done(1);
    settled(&speaker);
    assert_that!(speaker.speaking()).is_true();
    engine.done(2);
    settled(&speaker);
    assert_that!(speaker.speaking()).is_false();
    let ids: Vec<u64> = engine.calls().spoken.iter().map(|(id, _, _)| id.get()).collect();
    assert_that!(ids).is_equal_to(vec![1, 2]);
}

#[test]
fn speech_of_unavailable_language_is_skipped_without_suspending() {
    let dir = fresh_dir("speaker-unavailable");
    let stt = stt(dir.path());
    let engine = Arc::new(HandEngine::default());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), HandClock::new());
    speaker.say(Speech { lang: "ja".into(), ..speech("こんにちは") });
    settled(&speaker);
    assert_that!(engine.calls().spoken.is_empty()).is_true();
    assert_that!(stt.status().suspended).is_false();
}

#[test]
fn voices_listed_once_until_refreshed() {
    let dir = fresh_dir("speaker-voices");
    let engine = Arc::new(HandEngine { inline: true, ..HandEngine::default() });
    let speaker = Speaker::with_clock(engine.clone(), Some(stt(dir.path())), HandClock::new());
    speaker.say(speech("Un"));
    speaker.say(speech("Deux"));
    settled(&speaker);
    let _ = speaker.status("fr");
    let before = engine.calls().listed;
    speaker.refresh_voices();
    let _ = speaker.status("fr");
    // Each read on its own line: the calls lock is not held twice.
    let after = engine.calls().listed;
    let spoken = spoken_texts(&engine).len();
    assert_that!((before, after, spoken)).is_equal_to((1, 2, 2));
}

#[test]
fn unavailable_language_is_listed_again() {
    let dir = fresh_dir("speaker-voices-none");
    let engine = Arc::new(HandEngine::default());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt(dir.path())), HandClock::new());
    let _ = speaker.status("ja");
    let _ = speaker.status("ja");
    assert_that!(engine.calls().listed).is_equal_to(2);
}

/// Marks the dictation as listening, the user speaking or not.
fn user_speaks(stt: &Stt, speaking: bool) {
    let mut g = stt.state();
    g.state = crate::State::Listening;
    g.speaking = speaking;
    g.phrase_open = speaking;
}

fn spoken_texts(engine: &HandEngine) -> Vec<String> {
    engine.calls().spoken.iter().map(|(_, text, _)| text.clone()).collect()
}

#[test]
fn speech_refused_by_os_voice_keeps_listening() {
    let dir = fresh_dir("speaker-refused");
    let stt = stt(dir.path());
    user_speaks(&stt, false);
    let engine = Arc::new(HandEngine { refuse: true, ..HandEngine::default() });
    let clock = HandClock::new();
    let speaker = Speaker::with_clock(engine, Some(stt.clone()), clock.clone());
    speaker.say(speech("Bonjour"));
    settled(&speaker);
    assert_that!((stt.status().suspended, speaker.speaking(), speaker.hears(clock.now())))
        .is_equal_to((false, false, true));
}

#[test]
fn speech_waits_while_user_speaks_then_reads() {
    let dir = fresh_dir("speaker-hold");
    let stt = stt(dir.path());
    user_speaks(&stt, true);
    let engine = Arc::new(HandEngine::default());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), HandClock::new());
    speaker.say(speech("Diapositive ajoutée"));
    settled(&speaker);
    assert_that!((spoken_texts(&engine).len(), stt.status().suspended)).is_equal_to((0, false));
    user_speaks(&stt, false);
    speaker.tick();
    settled(&speaker);
    assert_that!(spoken_texts(&engine)).is_equal_to(vec!["Diapositive ajoutée".to_owned()]);
}

/// The listener's view mid-sentence: the last frame unvoiced (a gap between words), the phrase open.
fn user_pauses_mid_phrase(stt: &Stt) {
    let mut g = stt.state();
    g.state = crate::State::Listening;
    g.speaking = false;
    g.phrase_open = true;
}

#[test]
fn speech_waits_through_a_gap_between_words() {
    let dir = fresh_dir("speaker-hold-gap");
    let stt = stt(dir.path());
    user_pauses_mid_phrase(&stt);
    let engine = Arc::new(HandEngine::default());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), HandClock::new());
    speaker.say(speech("Diapositive ajoutée"));
    speaker.tick();
    settled(&speaker);
    assert_that!((spoken_texts(&engine).len(), stt.status().suspended)).is_equal_to((0, false));
}

#[test]
fn speech_held_at_most_the_hold_bound() {
    let dir = fresh_dir("speaker-hold-bound");
    let stt = stt(dir.path());
    user_speaks(&stt, true);
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), clock.clone());
    speaker.say(speech("Première"));
    speaker.say(speech("Seconde"));
    settled(&speaker);
    clock.advance(SPEECH_HOLD.checked_sub(Duration::from_millis(1)).unwrap_or_else(|| panic!("hold underflow")));
    speaker.tick();
    settled(&speaker);
    assert_that!(spoken_texts(&engine).len()).is_equal_to(0);
    clock.advance(Duration::from_millis(1));
    speaker.tick();
    settled(&speaker);
    // The newest speech replaced the held one, as it would cut it.
    assert_that!(spoken_texts(&engine)).is_equal_to(vec!["Seconde".to_owned()]);
}

#[test]
fn cut_drops_held_speech() {
    let dir = fresh_dir("speaker-hold-cut");
    let stt = stt(dir.path());
    user_speaks(&stt, true);
    let engine = Arc::new(HandEngine::default());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), HandClock::new());
    let (tx, rx) = mpsc::channel();
    let ended = SpeechEnded::new(move |_| tx.send(()).unwrap_or_else(|e| panic!("ended: {e}")));
    speaker.say(Speech { ended: Some(ended), ..speech("Question ?") });
    speaker.cut();
    settled(&speaker);
    user_speaks(&stt, false);
    speaker.tick();
    settled(&speaker);
    assert_that!((spoken_texts(&engine).len(), rx.try_recv().is_ok())).is_equal_to((0, true));
}

#[cfg(feature = "e2e-fakes")]
#[test]
fn speech_of_confirmation_never_confirms_it() {
    use std::io::Write;

    use crate::State;
    use crate::fake_source::{FakeSource, tick};

    let dir = fresh_dir("speaker-echo");
    let said = dir.path().join("said.txt");
    let stt = stt(dir.path());
    let rx = stt.utterances().unwrap_or_else(|e| panic!("utterances: {e}"));
    let session = {
        let mut g = stt.state();
        g.state = State::Listening;
        g.session
    };
    let append = {
        let said = said.clone();
        move |text: &str| {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&said)
                .unwrap_or_else(|e| panic!("said: {e}"));
            file.write_all(format!("{text}\n").as_bytes()).unwrap_or_else(|e| panic!("said: {e}"));
        }
    };
    append("");
    let mut source = FakeSource::at_end(&stt.storage, said.clone());
    // The speakers' sound reaches the microphone: each speech is heard as it plays.
    let echo = append.clone();
    let engine = Arc::new(HandEngine { echo: Some(Box::new(move |t| echo(t))), ..HandEngine::default() });
    let clock = HandClock::new();
    let speaker = Speaker::with_clock(engine.clone(), Some(stt.clone()), clock.clone());
    speaker.say(speech("Pour confirmer, dites « fusionner », ou non."));
    settled(&speaker);
    assert_that!(tick(&stt.inner, &stt.storage, &mut source, session)).is_true();
    engine.done(1);
    settled(&speaker);
    assert_that!(tick(&stt.inner, &stt.storage, &mut source, session)).is_true();
    clock.advance(SPEECH_GUARD);
    speaker.tick();
    settled(&speaker);
    append("sentinelle");
    assert_that!(tick(&stt.inner, &stt.storage, &mut source, session)).is_true();
    let heard: Vec<String> = rx.try_iter().map(|u| u.text).collect();
    assert_that!(heard).is_equal_to(vec!["sentinelle".to_owned()]);
}
