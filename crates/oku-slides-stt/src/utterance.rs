//! Utterances of the listening mode: phrases kept, closing, numbering, suspension.
//!
//! In listening mode (`State::Listening`) each stretch of speech followed by a silence of
//! `config.silence` becomes one [`Utterance`], handed to the host on the channel of
//! [`Stt::utterances`](crate::Stt::utterances). Its times come from sample offsets on the
//! listening clock ([`ListenClock`]), rebased at each resume. Its text is never logged.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError, Weak};
use std::time::{Duration, Instant};

use crate::num::seconds;
use crate::{Inner, SAMPLE_RATE, State, join};

/// Number of a closed utterance, unique for the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtteranceSeq(pub u64);

/// One closed stretch of speech in listening mode; not Clone, not Serialize.
#[derive(Debug)]
pub struct Utterance {
    /// Its number, never reused.
    pub seq: UtteranceSeq,
    /// What was said, trimmed and never empty.
    pub text: String,
    /// When the first phrase began.
    pub started: Instant,
    /// When the last phrase ended.
    pub ended: Instant,
    /// Seconds of speech in it, silences between phrases left out.
    pub speech_secs: f32,
}

/// Next utterance number of the process.
static NEXT_SEQ: AtomicU64 = AtomicU64::new(0);

/// Takes the next utterance number; `None` once the numbers run out.
fn next_seq() -> Option<UtteranceSeq> {
    NEXT_SEQ.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1)).ok().map(UtteranceSeq)
}

/// A phrase of the current dictation or utterance, and its place on the listening clock.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spoken {
    /// Its text; `None` while Whisper has not transcribed it yet.
    pub text: Option<String>,
    /// First and past-the-end sample, counted from the clock's origin.
    pub start: usize,
    pub end: usize,
}

/// The phrases not yet handed out, numbered from `base` (absolute phrase indices).
///
/// A discarded or closed phrase moves `base`, so a late transcription of it finds no slot.
#[derive(Debug, Default)]
pub(crate) struct Phrases {
    items: VecDeque<Spoken>,
    base: usize,
}

impl Phrases {
    /// Adds a phrase; returns its absolute index, `None` once indices run out.
    pub(crate) fn push(&mut self, spoken: Spoken) -> Option<usize> {
        let index = self.next_index()?;
        self.items.push_back(spoken);
        Some(index)
    }

    /// Index the next phrase will get.
    pub(crate) fn next_index(&self) -> Option<usize> {
        self.base.checked_add(self.items.len())
    }

    /// Stores the text of phrase `index`; false when it was discarded or closed meanwhile.
    pub(crate) fn set(&mut self, index: usize, text: String) -> bool {
        match index.checked_sub(self.base).and_then(|i| self.items.get_mut(i)) {
            Some(slot) => {
                slot.text = Some(text);
                true
            }
            None => false,
        }
    }

    /// The texts kept so far, joined.
    pub(crate) fn committed(&self) -> String {
        join(self.items.iter().filter_map(|s| s.text.as_ref()))
    }

    /// No phrase kept.
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Hands out the phrases before index `upto`, keeping the later ones.
    pub(crate) fn take_upto(&mut self, upto: usize) -> Vec<Spoken> {
        let count = upto.saturating_sub(self.base).min(self.items.len());
        let Some(base) = self.base.checked_add(count) else {
            tracing::error!("phrase indices exhausted: phrases kept");
            return Vec::new();
        };
        self.base = base;
        self.items.drain(..count).collect()
    }

    /// Forgets every phrase kept; their indices are never given again.
    pub(crate) fn discard(&mut self) {
        if let Some(base) = self.base.checked_add(self.items.len()) {
            self.base = base;
        } else {
            // `push` refuses past usize::MAX, so this is a bug; the phrases still go.
            tracing::error!("phrase indices exhausted while discarding");
        }
        self.items.clear();
    }
}

/// The listening clock: sample offset 0 is `origin`; a stream final at `stream_base` is offset 0.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ListenClock {
    pub origin: Instant,
    /// Samples given to a continuous engine before `origin`.
    pub stream_base: usize,
}

/// `origin` plus `offset` samples at 16 kHz; `None` on overflow.
pub(crate) fn instant_at(origin: Instant, offset: usize) -> Option<Instant> {
    let offset = u64::try_from(offset).ok()?;
    let rate = u64::from(SAMPLE_RATE);
    let rest = u32::try_from(offset % rate).ok()?;
    // 62 500 ns per sample at 16 kHz.
    let nanos = rest.checked_mul(1_000_000_000 / SAMPLE_RATE)?;
    origin.checked_add(Duration::new(offset / rate, nanos))
}

/// An utterance before its number: text, times and speech length.
#[derive(Debug, PartialEq)]
pub(crate) struct Draft {
    pub text: String,
    pub started: Instant,
    pub ended: Instant,
    pub speech_secs: f32,
}

/// Builds an utterance from its phrases; `None` for noise, empty text or bad offsets.
///
/// Noise is less than `min_secs` of speech (`stt.min_phrase_secs`).
pub(crate) fn draft(spoken: &[Spoken], origin: Instant, min_secs: f32) -> Option<Draft> {
    let text = join(spoken.iter().filter_map(|s| s.text.as_ref()));
    if text.is_empty() {
        return None;
    }
    let speech = spoken.iter().try_fold(0usize, |sum, s| sum.checked_add(s.end.checked_sub(s.start)?))?;
    let speech_secs = seconds(speech);
    if speech_secs < min_secs {
        return None;
    }
    let started = instant_at(origin, spoken.first()?.start)?;
    let ended = instant_at(origin, spoken.last()?.end)?;
    Some(Draft { text, started, ended, speech_secs })
}

impl Inner {
    /// Numbers a draft and sends it to the host; the text is never logged.
    pub(crate) fn deliver(&self, draft: Draft) {
        let Some(seq) = next_seq() else {
            tracing::error!("utterance numbers exhausted: utterance dropped");
            return;
        };
        let Draft { text, started, ended, speech_secs } = draft;
        if self.utterances.send(Utterance { seq, text, started, ended, speech_secs }).is_err() {
            tracing::debug!(seq = seq.0, "utterance closed with nobody listening");
        }
    }

    /// Closes the given phrases into an utterance, unless they are noise.
    pub(crate) fn close_utterance(&self, spoken: &[Spoken], min_secs: f32) {
        if let Some(draft) = draft(spoken, self.clock.origin, min_secs) {
            self.deliver(draft);
        }
    }

    /// Drops the utterance in progress (suspension, resume): its late results find no slot.
    pub(crate) fn discard_utterance(&mut self) {
        self.phrases.discard();
        self.partial.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }
}

/// Keeps listening suspended while alive; nested guards are counted.
#[must_use = "listening resumes as soon as the guard drops"]
#[derive(Debug)]
pub struct SuspendGuard {
    inner: Weak<Mutex<Inner>>,
    /// False when the count could not grow: dropping it then changes nothing.
    counted: bool,
}

impl SuspendGuard {
    /// Suspends listening on `inner` until the guard drops.
    pub(crate) fn new(inner: Weak<Mutex<Inner>>) -> Self {
        let Some(strong) = inner.upgrade() else { return Self { inner, counted: false } };
        let mut g = strong.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(count) = g.suspends.checked_add(1) else {
            tracing::error!("too many listening suspensions: this one is not counted");
            return Self { inner: Weak::new(), counted: false };
        };
        if g.suspends == 0 && g.state == State::Listening {
            g.discard_utterance();
        }
        g.suspends = count;
        drop(g);
        Self { inner, counted: true }
    }
}

impl Drop for SuspendGuard {
    fn drop(&mut self) {
        if !self.counted {
            return;
        }
        let Some(strong) = self.inner.upgrade() else { return };
        let mut g = strong.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(count) = g.suspends.checked_sub(1) else {
            tracing::error!("listening resumed more often than suspended");
            return;
        };
        g.suspends = count;
        if count == 0 && g.state == State::Listening {
            g.discard_utterance();
        }
    }
}

#[cfg(test)]
#[path = "utterance_tests.rs"]
mod utterance_tests;
