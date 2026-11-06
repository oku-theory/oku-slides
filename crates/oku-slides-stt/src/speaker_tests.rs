use std::sync::Arc;
use std::time::Duration;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::ECHO_HORIZON;
use crate::hand_voice::{HandClock, HandEngine, user_speaks};
use crate::speak::{SpeechEngine, SpeechId, SpeechRate};
use crate::test_support::{fresh_dir, platform_in};

/// How long a test waits for the speaker thread: an event, never a sleep.
const WITHIN: Duration = Duration::from_secs(10);

/// A dictation in French that never opens a microphone: only its suspension is read.
fn stt(dir: &std::path::Path) -> Stt {
    Stt::new(&platform_in(dir, "fr", |s| s.apple = false))
}

fn speech(text: &str) -> Speech {
    Speech { text: text.into(), lang: "fr".into(), voice: None, rate: SpeechRate::default(), ended: None, asks: false }
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
    assert_that!(watchdog(&"a".repeat(chars), rate)).is_equal_to(Duration::from_millis(millis));
}

#[test]
fn watchdog_unspaced_text_gets_longer_bound() {
    // 30 characters at 6 a second: 5 s, plus 3 s; the same count of Latin letters gets 5 s.
    let (ja, latin) = ("追".repeat(30), "a".repeat(30));
    let bounds = (watchdog(&ja, SpeechRate::default()), watchdog(&latin, SpeechRate::default()));
    assert_that!(bounds).is_equal_to((Duration::from_secs(8), Duration::from_secs(5)));
}

/// A speaker that read `text`, its DONE at `end` from the start: the clock is then at its end.
fn spoke(text: &str, end: Duration) -> (Speaker, Arc<HandClock>, tempfile::TempDir) {
    let dir = fresh_dir("speaker-echo-filter");
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine.clone(), Some(stt(dir.path())), clock.clone());
    speaker.say(speech(text));
    settled(&speaker);
    clock.advance(end);
    engine.done(1);
    settled(&speaker);
    (speaker, clock, dir)
}

#[test]
fn speaker_echoes_within_horizon_true() {
    let (speaker, clock, _dir) = spoke("Diapositive ajoutée sur le thème bleu", Duration::from_secs(2));
    let started = clock.now().checked_add(ECHO_HORIZON).unwrap_or_else(|| panic!("clock overflow"));
    assert_that!(speaker.echoes("diapositive ajoutée", started)).is_true();
}

#[test]
fn speaker_echoes_after_horizon_false() {
    let (speaker, clock, _dir) = spoke("Diapositive ajoutée sur le thème bleu", Duration::from_secs(2));
    let horizon = ECHO_HORIZON.checked_add(Duration::from_millis(1)).unwrap_or_else(|| panic!("overflow"));
    let started = clock.now().checked_add(horizon).unwrap_or_else(|| panic!("clock overflow"));
    assert_that!(speaker.echoes("diapositive ajoutée", started)).is_false();
}

#[test]
fn speaker_echoes_while_speaking_true() {
    let dir = fresh_dir("speaker-echo-playing");
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine, Some(stt(dir.path())), clock.clone());
    speaker.say(speech("Diapositive ajoutée"));
    settled(&speaker);
    assert_that!(speaker.echoes("diapositive ajoutée", clock.now())).is_true();
}

#[test]
fn speaker_echoes_only_the_last_three_speeches() {
    let dir = fresh_dir("speaker-echo-recent");
    let (engine, clock) = (Arc::new(HandEngine::inline()), HandClock::new());
    let speaker = Speaker::with_clock(engine, Some(stt(dir.path())), clock.clone());
    for text in ["Premier texte lu", "Deuxième texte lu", "Troisième texte lu", "Quatrième texte lu"] {
        speaker.say(speech(text));
        settled(&speaker);
    }
    let now = clock.now();
    assert_that!((speaker.echoes("premier texte", now), speaker.echoes("deuxieme texte", now)))
        .is_equal_to((false, true));
}

#[test]
fn speech_that_asks_forgets_earlier_speeches() {
    let dir = fresh_dir("speaker-echo-asks");
    let (engine, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(engine, Some(stt(dir.path())), clock.clone());
    speaker.say(speech("Diapositive ajoutée sur le thème bleu"));
    settled(&speaker);
    speaker.say(Speech { asks: true, ..speech("Pour enregistrer, dites oui, ou non.") });
    settled(&speaker);
    // The receipt and the question share one deaf window: an answer repeating it is heard.
    assert_that!(speaker.echoes("diapositive ajoutée", clock.now())).is_false();
}

#[test]
fn inline_done_does_not_deadlock() {
    let dir = fresh_dir("speaker-inline");
    let stt = stt(dir.path());
    let engine = Arc::new(HandEngine::inline());
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
    let engine = Arc::new(HandEngine::inline());
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

fn spoken_texts(engine: &HandEngine) -> Vec<String> {
    engine.calls().spoken.iter().map(|(_, text, _)| text.clone()).collect()
}

#[test]
fn speech_refused_by_os_voice_keeps_listening() {
    let dir = fresh_dir("speaker-refused");
    let stt = stt(dir.path());
    user_speaks(&stt, false);
    let engine = Arc::new(HandEngine::refusing());
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

/// Listening through the duplex, whose voice is `voice`, with the barge-in setting `on`.
fn listening_through_duplex(stt: &Stt, voice: Option<Arc<HandEngine>>, on: bool) {
    let mut g = stt.state();
    g.state = crate::State::Listening;
    g.barge = voice.map(|v| v as Arc<dyn SpeechEngine>);
    g.config.speech.barge_in = on;
}

/// A speaker over the OS voice `system`, listening through the duplex voice `duplex` or not.
fn barge_speaker(
    name: &str,
    duplex: Option<Arc<HandEngine>>,
    on: bool,
) -> (Speaker, Stt, Arc<HandEngine>, Arc<HandClock>, tempfile::TempDir) {
    let dir = fresh_dir(name);
    let stt = stt(dir.path());
    listening_through_duplex(&stt, duplex, on);
    let (system, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(system.clone(), Some(stt.clone()), clock.clone());
    (speaker, stt, system, clock, dir)
}

#[test]
fn barge_in_user_speaks_cuts_speech() {
    let duplex = Arc::new(HandEngine::default());
    let (speaker, stt, system, clock, _dir) = barge_speaker("speaker-barge", Some(duplex.clone()), true);
    speaker.say(speech("Diapositive ajoutée sur le thème bleu"));
    settled(&speaker);
    // Through the duplex, the microphone stays open and hears the user from the start.
    assert_that!((spoken_texts(&duplex).len(), spoken_texts(&system).len())).is_equal_to((1, 0));
    assert_that!((stt.status().suspended, speaker.speaking(), speaker.hears(clock.now())))
        .is_equal_to((false, true, true));
    user_speaks(&stt, true);
    clock.advance(BARGE_POLL);
    speaker.tick();
    settled(&speaker);
    assert_that!((duplex.calls().stopped.clone(), speaker.speaking())).is_equal_to((vec![SpeechId::new(1)], false));
}

#[test]
fn barge_in_cut_question_reports_its_start() {
    let duplex = Arc::new(HandEngine::default());
    let (speaker, stt, _system, clock, _dir) = barge_speaker("speaker-barge-ask", Some(duplex), true);
    let (tx, rx) = mpsc::channel();
    let ended = SpeechEnded::new(move |at| tx.send(at).unwrap_or_else(|e| panic!("ended: {e}")));
    let start = clock.now();
    speaker.say(Speech { ended: Some(ended), asks: true, ..speech("Pour enregistrer, dites oui, ou non.") });
    settled(&speaker);
    clock.advance(Duration::from_secs(1));
    user_speaks(&stt, true);
    speaker.tick();
    settled(&speaker);
    // Cut by the answer itself: the question ends where it started, so the answer is not early.
    assert_that!(rx.try_recv().ok()).is_equal_to(Some(start));
}

#[test]
fn barge_in_cut_by_a_new_speech_reports_now() {
    let duplex = Arc::new(HandEngine::default());
    let (speaker, _stt, _system, clock, _dir) = barge_speaker("speaker-barge-ask-cut", Some(duplex), true);
    let (tx, rx) = mpsc::channel();
    let ended = SpeechEnded::new(move |at| tx.send(at).unwrap_or_else(|e| panic!("ended: {e}")));
    speaker.say(Speech { ended: Some(ended), asks: true, ..speech("Pour enregistrer, dites oui, ou non.") });
    settled(&speaker);
    clock.advance(Duration::from_secs(1));
    speaker.say(speech("Diapositive ajoutée"));
    settled(&speaker);
    assert_that!(rx.try_recv().ok()).is_equal_to(Some(clock.now()));
}

#[test]
fn barge_in_quiet_user_lets_speech_play() {
    let duplex = Arc::new(HandEngine::default());
    let (speaker, _stt, _system, clock, _dir) = barge_speaker("speaker-barge-quiet", Some(duplex.clone()), true);
    speaker.say(speech("Diapositive ajoutée"));
    settled(&speaker);
    clock.advance(BARGE_POLL);
    speaker.tick();
    settled(&speaker);
    assert_that!((duplex.calls().stopped.len(), speaker.speaking())).is_equal_to((0, true));
}

#[test]
fn barge_speech_is_not_kept_for_echoes() {
    let duplex = Arc::new(HandEngine::default());
    let (speaker, _stt, _system, clock, _dir) = barge_speaker("speaker-barge-echo", Some(duplex), true);
    speaker.say(speech("Diapositive ajoutée sur le thème bleu"));
    settled(&speaker);
    // The echo canceller removes it: the user may talk over it with its own words.
    assert_that!(speaker.echoes("diapositive ajoutée", clock.now())).is_false();
}

#[test]
fn barge_in_off_keeps_mic_suspended() {
    let duplex = Arc::new(HandEngine::default());
    let (speaker, stt, system, clock, _dir) = barge_speaker("speaker-barge-off", Some(duplex.clone()), false);
    speaker.say(speech("Diapositive ajoutée"));
    settled(&speaker);
    user_speaks(&stt, true);
    clock.advance(BARGE_POLL);
    speaker.tick();
    settled(&speaker);
    assert_that!((spoken_texts(&system).len(), spoken_texts(&duplex).len())).is_equal_to((1, 0));
    assert_that!((stt.status().suspended, system.calls().stopped.len(), speaker.hears(clock.now())))
        .is_equal_to((true, 0, false));
}

#[test]
fn barge_in_unavailable_keeps_mic_suspended() {
    let (speaker, stt, system, clock, _dir) = barge_speaker("speaker-barge-unavailable", None, true);
    speaker.say(speech("Diapositive ajoutée"));
    settled(&speaker);
    user_speaks(&stt, true);
    clock.advance(BARGE_POLL);
    speaker.tick();
    settled(&speaker);
    assert_that!((spoken_texts(&system).len(), stt.status().suspended, system.calls().stopped.len()))
        .is_equal_to((1, true, 0));
}

#[test]
fn barge_in_speech_after_muted_one_cuts_it_on_its_voice() {
    let duplex = Arc::new(HandEngine::default());
    let (speaker, stt, system, _clock, _dir) = barge_speaker("speaker-barge-switch", Some(duplex.clone()), false);
    speaker.say(speech("Première"));
    settled(&speaker);
    stt.state().config.speech.barge_in = true;
    speaker.say(speech("Seconde"));
    settled(&speaker);
    // Another voice speaks the second: the first is cut on its own, and the mic opens again.
    assert_that!((system.calls().stopped.clone(), spoken_texts(&duplex), stt.status().suspended)).is_equal_to((
        vec![SpeechId::new(1)],
        vec!["Seconde".to_owned()],
        false,
    ));
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
    let engine = Arc::new(HandEngine::echoing(Box::new(move |t| echo(t))));
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
