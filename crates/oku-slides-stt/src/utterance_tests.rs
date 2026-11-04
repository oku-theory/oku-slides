use std::time::{Duration, Instant};

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::State;
use crate::test_support::{Part, fresh_dir, listening_stt};

fn spoken(text: &str, start: usize, end: usize) -> Spoken {
    Spoken { text: Some(text.to_owned()), start, end }
}

/// nextest runs each test in its own process, so the process-wide numbers are this test's alone.
#[test]
fn utterance_seq_never_reused_across_suspend() {
    let dir = fresh_dir("utterance-seq");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    let rx = stt.utterances().unwrap();
    stt.state().state = State::Listening;
    stt.state().close_utterance(&[spoken("Un.", 0, 16_000)], 0.3);
    // An utterance in progress, discarded by the suspension, then a new one after the resume.
    stt.state().phrases.push(spoken("Perdu.", 16_000, 32_000));
    let guard = stt.suspend();
    assert_that!(stt.state().phrases.is_empty()).is_true();
    drop(guard);
    stt.state().close_utterance(&[spoken("Deux.", 0, 16_000)], 0.3);
    let (first, second) = (rx.try_recv().unwrap(), rx.try_recv().unwrap());
    assert_that!(second.text).is_equal_to("Deux.".to_owned());
    assert_that!(second.seq).is_equal_to(UtteranceSeq(first.seq.0 + 1));
}

/// Each test runs in its own process under nextest: the numbers are contiguous here.
#[rstest]
#[case::empty("", 16_000)]
#[case::spaces("   ", 16_000)]
#[case::too_short("Oui.", 3_200)]
fn short_utterance_is_dropped_without_seq(#[case] text: &str, #[case] samples: usize) {
    let dir = fresh_dir("utterance-noise");
    let (stt, _engine) = listening_stt(dir.path(), &[Part::Silence(1.0)], 1.5);
    let rx = stt.utterances().unwrap();
    stt.state().close_utterance(&[spoken("Avant.", 0, 16_000)], 0.3);
    stt.state().close_utterance(&[spoken(text, 0, samples)], 0.3);
    stt.state().close_utterance(&[spoken("Après.", 0, 16_000)], 0.3);
    let (before, after) = (rx.try_recv().unwrap(), rx.try_recv().unwrap());
    assert_that!(after.text).is_equal_to("Après.".to_owned());
    assert_that!(after.seq).is_equal_to(UtteranceSeq(before.seq.0 + 1));
}

#[test]
fn draft_of_noise_is_none() {
    assert_that!(draft(&[spoken("Oui.", 0, 3_200)], Instant::now(), 0.3)).is_none();
}

#[test]
fn utterance_times_from_sample_offsets_are_checked() {
    let origin = Instant::now();
    let d = draft(&[spoken("Un.", 8_000, 24_000), spoken("Deux.", 40_000, 56_000)], origin, 0.3).unwrap();
    assert_that!(d.started).is_equal_to(origin + Duration::from_millis(500));
    assert_that!(d.ended).is_equal_to(origin + Duration::from_millis(3_500));
    assert_that!(d.speech_secs).is_equal_to(2.0);
    assert_that!(instant_at(origin, 16_000 * 3_600 + 1)).is_equal_to(origin.checked_add(Duration::new(3_600, 62_500)));
    assert_that!(draft(&[spoken("Un.", 16_000, 8_000)], origin, 0.0)).is_none();
}

#[test]
fn phrases_take_upto_keeps_the_later_ones() {
    let mut phrases = Phrases::default();
    phrases.push(spoken("Un.", 0, 1));
    phrases.push(spoken("Deux.", 1, 2));
    assert_that!(phrases.take_upto(1)).is_equal_to(vec![spoken("Un.", 0, 1)]);
    assert_that!(phrases.committed()).is_equal_to("Deux.".to_owned());
    assert_that!(phrases.next_index()).is_equal_to(Some(2));
}

#[test]
fn phrases_set_below_base_is_dropped() {
    let mut phrases = Phrases::default();
    let index = phrases.push(Spoken { text: None, start: 0, end: 1 }).unwrap();
    phrases.discard();
    assert_that!(phrases.set(index, "Tard.".into())).is_false();
    assert_that!(phrases.next_index()).is_equal_to(Some(1));
}
