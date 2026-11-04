//! The listening loop: microphone, 16 kHz, voice detection, then the engine.
//!
//! A dictation (`State::Recording`) stops by itself after a silence. Hands-free listening
//! (`State::Listening`) never stops by itself: it closes an utterance after each silence of
//! `config.silence` (see `utterance.rs`), drops the samples captured while suspended, and
//! rebases its clock whenever the utterance in progress is discarded.

use std::sync::PoisonError;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::audio::{self, SAMPLE_RATE_F32};
use crate::dictation_stream::cancel_held;
use crate::dictation_whisper::Job;
use crate::num::count_f32;
use crate::utterance::ListenClock;
use crate::vad::Segmenter;
use crate::{Sink, State, Stt};

/// What the listening loop keeps from one tick to the next.
struct Ear {
    seg: Segmenter,
    resampler: Option<audio::Resampler>,
    fresh: Vec<f32>,
    /// End of the last preview sent (sample).
    previewed: usize,
    /// Samples given to a continuous engine so far.
    pushed: usize,
    /// Listening: end of the last phrase closed into an utterance (sample).
    closed_upto: usize,
    /// Listening: end of the last phrase kept (sample).
    last_phrase_end: usize,
    /// Listening: the epoch the clock and the segmenter belong to.
    epoch: Option<u64>,
}

impl Stt {
    /// Listens: microphone → 16 kHz → voice detection → engine.
    pub(crate) fn listen_loop(&self, session: u64, mut recorder: audio::Recorder, mut sink: Sink) {
        let s = &self.settings;
        let mut ear = Ear {
            seg: Segmenter::new(s.vad),
            resampler: None,
            fresh: Vec::new(),
            previewed: 0,
            pushed: 0,
            closed_upto: 0,
            last_phrase_end: 0,
            // No epoch yet: the first listening tick rebases, dropping the samples captured
            // before it (about one tick), so offsets count from that tick.
            epoch: None,
        };
        loop {
            std::thread::sleep(Duration::from_millis(s.tick_ms.get()));
            let (state, auto_stop, silence_limit, suspended, epoch) = {
                let g = self.state();
                if g.session != session {
                    drop(g);
                    // Dictation abandoned: the streaming engine stops too.
                    if let Sink::Stream { stream, .. } = &mut sink {
                        stream.cancel();
                    }
                    return;
                }
                (g.state, g.config.auto_stop, g.config.silence, g.suspends > 0, g.epoch)
            };
            if state == State::Listening {
                self.listen_tick(&recorder, &mut ear, &mut sink, silence_limit, (suspended, epoch));
                continue;
            }
            self.pull(&recorder, &mut ear, &mut sink);

            if state != State::Recording {
                // Stop asked: last samples, last sentence.
                recorder.finish();
                self.pull(&recorder, &mut ear, &mut sink);
                match sink {
                    Sink::Whisper { jobs, .. } => {
                        if let Some(phrase) = ear.seg.close() {
                            self.send_final(&ear.seg, phrase, &jobs);
                        }
                        jobs.send(Job::Done).ok();
                    }
                    Sink::Stream { mut stream, held } => {
                        stream.finish();
                        // Handed to the results thread; if it is already gone, it is cancelled here.
                        *held.lock().unwrap_or_else(PoisonError::into_inner) = Some(stream);
                        if !self.alive(session) {
                            cancel_held(&held);
                        }
                    }
                }
                return;
            }

            // Preview of the sentence in progress (local Whisper), when the transcription thread is free.
            let seg = &ear.seg;
            let current = seg.current();
            if let Sink::Whisper { jobs, busy, previews: true } = &sink
                && let Some(p) = current
                && !busy.load(Ordering::Relaxed)
                && count_f32(p.end.saturating_sub(ear.previewed.max(p.start))) >= s.preview_step_secs * SAMPLE_RATE_F32
            {
                ear.previewed = p.end;
                busy.store(true, Ordering::Relaxed);
                jobs.send(Job::Preview { audio: seg.slice(p).to_vec() }).ok();
            }

            // Automatic stop after a silence (or if nothing is said).
            let silence = seg.silence();
            let limit = if seg.heard { silence_limit } else { s.nothing_heard_secs };
            let stop_in = (auto_stop && current.is_none() && seg.heard && silence > s.countdown_after_secs)
                .then(|| (limit - silence).max(0.0));
            let stop = auto_stop && silence >= limit;
            {
                let mut g = self.state();
                g.level = recorder.level();
                g.speaking = seg.speaking;
                g.phrase_open = current.is_some();
                g.stop_in = stop_in;
                // Whisper's preview follows the sentence in progress; the streaming engine's, its results.
                if current.is_none() && matches!(sink, Sink::Whisper { .. }) {
                    g.partial.clear();
                }
                if stop && g.state == State::Recording {
                    g.state = State::Transcribing;
                    g.started = None;
                }
            }
        }
    }

    /// One tick of hands-free listening: no auto-stop, no preview, utterances closed on silence.
    ///
    /// `(suspended, epoch)`: read with the state. A new epoch rebases the clock first.
    fn listen_tick(
        &self,
        recorder: &audio::Recorder,
        ear: &mut Ear,
        sink: &mut Sink,
        silence_limit: f32,
        (suspended, epoch): (bool, u64),
    ) {
        if ear.epoch != Some(epoch) {
            self.rebase(recorder, ear, epoch);
        }
        if suspended {
            drop(recorder.take()); // captured while suspended: never heard
        } else {
            self.pull(recorder, ear, sink);
            self.close_if_silent(ear, sink, silence_limit, epoch);
        }
        let mut g = self.state();
        g.level = if suspended { 0.0 } else { recorder.level() };
        g.speaking = !suspended && ear.seg.speaking;
        g.phrase_open = !suspended && ear.seg.current().is_some();
        g.stop_in = None;
    }

    /// Starts the listening clock again: offsets count from now, earlier samples are dropped.
    fn rebase(&self, recorder: &audio::Recorder, ear: &mut Ear, epoch: u64) {
        drop(recorder.take());
        ear.seg = Segmenter::new(self.settings.vad);
        ear.previewed = 0;
        ear.closed_upto = 0;
        ear.last_phrase_end = 0;
        ear.epoch = Some(epoch);
        let mut g = self.state();
        g.clock = ListenClock { origin: Instant::now(), stream_base: ear.pushed };
        // A late final placed on the old clock before this rebase is not part of what follows.
        g.phrases.discard();
        g.partial.clear();
    }

    /// Closes the utterance once a phrase came after the last close and silence reached the limit.
    fn close_if_silent(&self, ear: &mut Ear, sink: &Sink, silence_limit: f32, epoch: u64) {
        if ear.seg.current().is_some() || ear.seg.silence() < silence_limit {
            return;
        }
        let mut g = self.state();
        if g.epoch != epoch {
            return; // discarded meanwhile: the next tick rebases
        }
        let Some(upto) = g.phrases.next_index() else { return };
        match sink {
            Sink::Whisper { jobs, .. } => {
                if ear.last_phrase_end <= ear.closed_upto {
                    return;
                }
                ear.closed_upto = ear.last_phrase_end;
                drop(g);
                // Closed once the phrases before `upto` are transcribed.
                jobs.send(Job::Flush { upto, epoch }).ok();
            }
            Sink::Stream { .. } => {
                // The finals since the last close; the stream itself goes on.
                if !g.phrases.is_empty() {
                    let spoken = g.phrases.take_upto(upto);
                    g.close_utterance(&spoken, self.settings.min_phrase_secs);
                }
            }
        }
    }

    /// Samples arrived: 16 kHz, voice detection, then the engine.
    fn pull(&self, recorder: &audio::Recorder, ear: &mut Ear, sink: &mut Sink) {
        let (samples, rate) = recorder.take();
        if samples.is_empty() {
            return;
        }
        ear.fresh.clear();
        ear.resampler.get_or_insert_with(|| audio::Resampler::new(rate)).push(&samples, &mut ear.fresh);
        let phrases = ear.seg.push(&ear.fresh);
        match sink {
            Sink::Whisper { jobs, .. } => {
                for phrase in phrases {
                    if self.send_final(&ear.seg, phrase, jobs) {
                        ear.last_phrase_end = ear.last_phrase_end.max(phrase.end);
                    }
                }
            }
            Sink::Stream { stream, .. } => {
                stream.push(&ear.fresh);
                // Saturating: usize::MAX samples is millions of years at 16 kHz.
                ear.pushed = ear.pushed.saturating_add(ear.fresh.len());
            }
        }
    }
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod listen_tests;
