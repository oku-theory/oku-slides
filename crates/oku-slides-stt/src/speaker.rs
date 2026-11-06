//! The process-wide speaker: the OS voice reads one text at a time, listening suspended.
//!
//! One [`Speaker`] per process (the spec's `say`). It speaks on its own thread, the only one
//! that calls [`SpeechEngine::speak`] and [`SpeechEngine::stop`], so speeches are serialised
//! and no lock is held across them: an engine's `done` only sends on the thread's channel, even
//! when it runs inside `speak` or `stop`.
//!
//! - A [`SuspendGuard`] is held from the start of a speech to [`SPEECH_GUARD`] after its end, so
//!   the microphone never hears Oku. [`Speaker::hears`] also drops an utterance that started in
//!   that window and closed after it.
//! - Barge-in: while listening goes through the duplex with the setting on ([`Stt::barge_voice`],
//!   macOS), its voice reads the speech, echo-cancelled, with no guard nor deaf window; every
//!   [`BARGE_POLL`] the speech is cut once [`Stt::user_speaking`]. Each speech picks its voice.
//! - A watchdog ends a speech whose DONE never comes (macOS with no main run loop, a stuck
//!   daemon) after `chars ÷ (per-script rate × rate) + 3 s`.
//! - [`Speaker::echoes`] drops an utterance that repeats words of a recent muted speech, kept
//!   folded in memory ([`crate::echo`]) and never logged; a barge speech is never kept.
//! - A barge speech the user cuts tells its `ended` the instant it began, not the cut: an
//!   answer spoken over a question never predates it.
//! - A new speech cuts the current one; a late DONE of a cut speech is ignored by its id.
//! - A speech never starts over the user: while [`Stt::user_speaking`] it waits, at most
//!   [`SPEECH_HOLD`]. A newer speech replaces the waiting one.
//! - The voice is resolved from [`SpeechEngine::status`] before each speech, never `None`.
//!
//! Time comes from a [`SpeakerClock`], injected in tests. Spoken text is never logged.

use std::collections::HashMap;
use std::fmt;
use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::{Duration, Instant};

use crate::echo::{Recent, unspaced};
use crate::num::count_f64;
use crate::speak::{SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, VoiceInfo};
use crate::{Stt, SuspendGuard};

/// How long listening stays suspended once a speech ended.
pub const SPEECH_GUARD: Duration = Duration::from_millis(500);

/// How long a speech waits at most while the user is speaking, before it is read anyway.
pub const SPEECH_HOLD: Duration = Duration::from_secs(2);

/// How often a held speech checks whether the user still speaks.
const HOLD_POLL: Duration = Duration::from_millis(50);

/// How often a speech the user may talk over checks whether the user speaks.
pub(crate) const BARGE_POLL: Duration = Duration::from_millis(50);

/// Characters an OS voice reads per second at the default rate, for the watchdog.
///
/// A spaced script (Latin, Cyrillic, Hangul…) reads about 15 a second; an unspaced one (CJK,
/// Thai), where a sign is a syllable or a word, about 6.
const CHARS_PER_SEC: f64 = 15.0;

/// [`CHARS_PER_SEC`] for a text of an unspaced script (CJK, Thai).
const UNSPACED_CHARS_PER_SEC: f64 = 6.0;

/// Added to the watchdog's estimate of a speech's length.
const WATCHDOG_SLACK: Duration = Duration::from_secs(3);

/// The watchdog's bound when the estimate cannot be computed (a bug: the rate is checked).
const WATCHDOG_MAX: Duration = Duration::from_secs(120);

/// Where the speaker reads the time; the system clock outside tests.
pub trait SpeakerClock: Send + Sync + fmt::Debug {
    /// The current instant.
    fn now(&self) -> Instant;
}

/// The system's monotonic clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SpeakerClock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Called once with the instant a speech ended: DONE, cut, watchdog or refusal.
pub struct SpeechEnded(Box<dyn FnOnce(Instant) + Send>);

impl SpeechEnded {
    /// Wraps the closure told when the speech ends.
    pub fn new(f: impl FnOnce(Instant) + Send + 'static) -> Self {
        Self(Box::new(f))
    }

    fn call(self, at: Instant) {
        (self.0)(at);
    }
}

impl fmt::Debug for SpeechEnded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SpeechEnded(..)")
    }
}

/// One text to read aloud, in one language.
#[derive(Debug)]
pub struct Speech {
    /// What to say, plain text built by the host.
    pub text: String,
    /// BCP 47 language of `text`: the voice is one of this language's.
    pub lang: String,
    /// The preferred voice id; another voice of `lang` when it is not offered.
    pub voice: Option<String>,
    /// The speaking rate.
    pub rate: SpeechRate,
    /// Told when this speech ends, if anyone cares.
    pub ended: Option<SpeechEnded>,
    /// It asks the user something (question, confirmation, permission, choices).
    ///
    /// Its words are what the user answers with, so it is never kept for [`Speaker::echoes`];
    /// the deaf window, or the echo canceller under barge-in, covers its echo. It also forgets
    /// the speeches read before it.
    pub asks: bool,
}

/// What the speaker thread is asked.
enum Msg {
    Say(Speech),
    Cut,
    Done(SpeechId),
    Tick,
    Settle(Sender<()>),
}

/// The window during which the microphone is deaf to utterances that started in it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Deaf {
    /// When the current run of speeches began; `None` while the user may talk over Oku.
    from: Option<Instant>,
    /// When listening may resume; `None` while a speech plays.
    until: Option<Instant>,
    /// A speech plays, the microphone deaf or not (barge-in).
    playing: bool,
}

impl Deaf {
    /// True when an utterance that started at `started` overlaps no speech nor its guard.
    fn hears(self, started: Instant) -> bool {
        match (self.from, self.until) {
            (None, _) => true,
            (Some(from), _) if started < from => true,
            (Some(_), None) => false,
            (Some(_), Some(until)) => started >= until,
        }
    }
}

/// What the speaker and its thread share: the channel and the deaf window.
struct Shared {
    tx: Sender<Msg>,
    deaf: Arc<Mutex<Deaf>>,
    recent: Arc<Mutex<Recent>>,
    engine: Arc<dyn SpeechEngine>,
    voices: Voices,
}

/// The voices listed per language, kept until refreshed: listing them is slow.
#[derive(Clone, Default)]
struct Voices(Arc<Mutex<HashMap<String, Vec<VoiceInfo>>>>);

impl Voices {
    /// The voices for `lang`, listed by `engine` the first time; a language without voice is
    /// listed again each time, so a voice installed meanwhile is found.
    fn status(&self, engine: &dyn SpeechEngine, lang: &str) -> SpeechStatus {
        if let Some(voices) = self.0.lock().unwrap_or_else(PoisonError::into_inner).get(lang) {
            return SpeechStatus::Ready { voices: voices.clone() };
        }
        let status = engine.status(lang);
        if let SpeechStatus::Ready { voices } = &status {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(lang.to_owned(), voices.clone());
        }
        status
    }

    /// Forgets every language: the next status lists its voices again.
    fn clear(&self) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clear();
    }
}

/// The process-wide voice: reads one text at a time, listening suspended meanwhile.
///
/// Clones share one thread. Every call returns at once except [`Speaker::status`] and
/// [`Speaker::settle`]; the thread ends when the last clone drops.
#[derive(Clone)]
pub struct Speaker {
    shared: Arc<Shared>,
}

impl fmt::Debug for Speaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Speaker").field("engine", &self.shared.engine).finish_non_exhaustive()
    }
}

impl Speaker {
    /// A speaker over `engine`, suspending `stt`'s listening while it speaks.
    #[must_use]
    pub fn new(engine: Arc<dyn SpeechEngine>, stt: Option<Stt>) -> Self {
        Self::with_clock(engine, stt, Arc::new(SystemClock))
    }

    /// The host's speaker: the OS voice, or the fake voice `stt.fake_speech` names (e2e only).
    #[must_use]
    pub fn for_platform(platform: &oku_slides_platform::Platform, stt: Option<Stt>) -> Self {
        #[cfg(feature = "e2e-fakes")]
        if let Some(log) = platform.settings.stt.fake_speech.clone() {
            let engine = crate::speak::fake::FakeSpeechEngine::new(platform.storage.clone(), log);
            return Self::new(Arc::new(engine), stt);
        }
        #[cfg(not(feature = "e2e-fakes"))]
        let _ = platform;
        Self::new(crate::speak::system_speech_engine(), stt)
    }

    /// A speaker that reads the time from `clock`; tests drive it with [`Speaker::tick`].
    #[must_use]
    pub fn with_clock(engine: Arc<dyn SpeechEngine>, stt: Option<Stt>, clock: Arc<dyn SpeakerClock>) -> Self {
        let (tx, rx) = mpsc::channel();
        let deaf = Arc::new(Mutex::new(Deaf::default()));
        let voices = Voices::default();
        let recent = Arc::new(Mutex::new(Recent::default()));
        let shared = Arc::new(Shared {
            tx,
            deaf: Arc::clone(&deaf),
            recent: Arc::clone(&recent),
            engine: Arc::clone(&engine),
            voices: voices.clone(),
        });
        let me = Arc::downgrade(&shared);
        let core = Core { engine, stt, clock, deaf, recent, voices, me, next: 0, phase: Phase::Quiet };
        let spawned = std::thread::Builder::new().name("oku-speaker".into()).spawn(move || run(&rx, core));
        if let Err(e) = spawned {
            tracing::warn!(error = %e, "speaker thread not started: nothing is read aloud");
        }
        Self { shared }
    }

    /// Reads `speech`, cutting the speech in progress.
    pub fn say(&self, speech: Speech) {
        self.send(Msg::Say(speech));
    }

    /// Cuts the speech in progress, if any.
    pub fn cut(&self) {
        self.send(Msg::Cut);
    }

    /// Checks the watchdog and the guard against the clock now.
    pub fn tick(&self) {
        self.send(Msg::Tick);
    }

    /// Waits until the thread handled every request sent before; false after `within`.
    #[must_use]
    pub fn settle(&self, within: Duration) -> bool {
        let (tx, rx) = mpsc::channel();
        self.send(Msg::Settle(tx));
        rx.recv_timeout(within).is_ok()
    }

    /// The voices for `lang`, or why none can speak. Blocking: call it off async workers.
    #[must_use]
    pub fn status(&self, lang: &str) -> SpeechStatus {
        self.shared.voices.status(self.shared.engine.as_ref(), lang)
    }

    /// Forgets the voices listed so far: the next speech or status lists them again.
    pub fn refresh_voices(&self) {
        self.shared.voices.clear();
    }

    /// True when an utterance that began at `started` overlaps no speech nor its guard.
    #[must_use]
    pub fn hears(&self, started: Instant) -> bool {
        self.shared.deaf.lock().unwrap_or_else(PoisonError::into_inner).hears(started)
    }

    /// True when `said` repeats, in a row, words of a speech that ended within [`ECHO_HORIZON`].
    ///
    /// A speech still playing counts too; only the last three speeches are compared, never one
    /// that asks the user something ([`Speech::asks`]) nor one read under barge-in.
    ///
    /// [`ECHO_HORIZON`]: crate::ECHO_HORIZON
    #[must_use]
    pub fn echoes(&self, said: &str, started: Instant) -> bool {
        self.shared.recent.lock().unwrap_or_else(PoisonError::into_inner).echoes(said, started)
    }

    /// True while a speech plays.
    #[must_use]
    pub fn speaking(&self) -> bool {
        self.shared.deaf.lock().unwrap_or_else(PoisonError::into_inner).playing
    }

    fn send(&self, msg: Msg) {
        if self.shared.tx.send(msg).is_err() {
            tracing::debug!("speaker thread gone: request dropped");
        }
    }
}

/// The watchdog's bound for `text` read at `rate`: `chars ÷ (per-script rate × rate) + 3 s`.
///
/// A higher rate reads faster, so the bound is shorter; an unspaced script reads fewer
/// characters a second ([`UNSPACED_CHARS_PER_SEC`]), so its bound is longer.
#[must_use]
pub fn watchdog(text: &str, rate: SpeechRate) -> Duration {
    let chars = text.chars().count();
    let script = if unspaced(text) { UNSPACED_CHARS_PER_SEC } else { CHARS_PER_SEC };
    let per_sec = script * f64::from(rate.get());
    Duration::try_from_secs_f64(count_f64(chars) / per_sec)
        .ok()
        .and_then(|d| d.checked_add(WATCHDOG_SLACK))
        .unwrap_or_else(|| {
            tracing::error!(chars, "speech watchdog not computable: its maximum applies");
            WATCHDOG_MAX
        })
}

/// `at` plus `by`; past the clock's range, `at` itself, logged as the bug it would be.
pub(crate) fn later(at: Instant, by: Duration) -> Instant {
    at.checked_add(by).unwrap_or_else(|| {
        tracing::error!(?by, "instant out of range: the bound ends now");
        at
    })
}

/// The voice of `voices` to speak with: the preferred one if offered, else the first.
fn pick<'a>(voices: &'a [VoiceInfo], preferred: Option<&str>) -> Option<&'a str> {
    preferred.and_then(|p| voices.iter().find(|v| v.id == p)).or_else(|| voices.first()).map(|v| v.id.as_str())
}

/// Where the speaker stands.
#[derive(Debug)]
enum Phase {
    /// Nothing plays, listening is not suspended by the speaker.
    Quiet,
    /// `speech` waits for the user to stop speaking, at most until `until`.
    Holding { speech: Speech, until: Instant },
    /// Speech `id`, begun at `since`, plays on `engine` until DONE, a cut or `deadline`;
    /// `barge`: the user may cut it.
    Speaking {
        id: SpeechId,
        since: Instant,
        deadline: Instant,
        ended: Option<SpeechEnded>,
        guard: Option<SuspendGuard>,
        engine: Arc<dyn SpeechEngine>,
        barge: bool,
    },
    /// The speech ended; listening resumes at `until`.
    Settling { until: Instant, guard: Option<SuspendGuard> },
}

/// The speaker thread's state: only this thread calls the engine.
struct Core {
    engine: Arc<dyn SpeechEngine>,
    stt: Option<Stt>,
    clock: Arc<dyn SpeakerClock>,
    deaf: Arc<Mutex<Deaf>>,
    /// The last speeches' folded texts, for [`Speaker::echoes`].
    recent: Arc<Mutex<Recent>>,
    voices: Voices,
    /// The shared half, for the engine's `done`: it sends on the channel while the speaker lives.
    me: Weak<Shared>,
    next: u64,
    phase: Phase,
}

impl Core {
    /// When the thread must wake up by itself: the watchdog or the guard's end.
    fn deadline(&self) -> Option<Instant> {
        match &self.phase {
            Phase::Quiet => None,
            Phase::Holding { until, .. } => Some((*until).min(later(self.clock.now(), HOLD_POLL))),
            Phase::Speaking { deadline, barge: true, .. } => Some((*deadline).min(later(self.clock.now(), BARGE_POLL))),
            Phase::Speaking { deadline, .. } => Some(*deadline),
            Phase::Settling { until, .. } => Some(*until),
        }
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Say(speech) => self.start(speech),
            Msg::Cut => {
                if matches!(self.phase, Phase::Holding { .. }) {
                    // Never started: told it ended, and nothing else to undo.
                    self.take_guard(self.clock.now());
                }
                if let Phase::Speaking { id, ref engine, .. } = self.phase {
                    if let Err(e) = Arc::clone(engine).stop(id) {
                        tracing::warn!(kind = ?e.kind(), "speech not cut");
                    }
                    self.end(self.clock.now());
                }
            }
            Msg::Done(done) => {
                if matches!(self.phase, Phase::Speaking { id, .. } if id == done) {
                    self.end(self.clock.now());
                } else {
                    tracing::debug!(id = done.get(), "late end of a cut speech: ignored");
                }
            }
            Msg::Tick => self.tick(),
            Msg::Settle(reply) => {
                let _ = reply.send(());
            }
        }
    }

    /// Ends what is due by now: a held speech, a speech whose DONE never came, then the guard.
    fn tick(&mut self) {
        let now = self.clock.now();
        if let Phase::Holding { until, .. } = self.phase
            && (now >= until || !self.user_speaking())
            && let Phase::Holding { speech, .. } = std::mem::replace(&mut self.phase, Phase::Quiet)
        {
            self.play(speech);
        }
        if matches!(self.phase, Phase::Speaking { barge: true, .. })
            && self.user_speaking()
            && let Phase::Speaking { id, since, ref engine, ref mut ended, .. } = self.phase
        {
            tracing::debug!(id = id.get(), "barge-in: the user speaks over the speech, cut");
            if let Err(e) = Arc::clone(engine).stop(id) {
                tracing::warn!(kind = ?e.kind(), "speech not cut by barge-in");
            }
            // Cut by the user's own words, maybe the answer to it: the speech is told it ended
            // where it began, so that answer never predates it. The echo canceller, not a deaf
            // window, kept the microphone clear of it meanwhile.
            if let Some(ended) = ended.take() {
                ended.call(since);
            }
            self.end(now);
        }
        if let Phase::Speaking { id, deadline, ref engine, .. } = self.phase
            && now >= deadline
        {
            tracing::warn!(id = id.get(), "speech watchdog: no end reported, listening resumes");
            if let Err(e) = Arc::clone(engine).stop(id) {
                tracing::debug!(kind = ?e.kind(), "speech not cut by the watchdog");
            }
            self.end(now);
        }
        if let Phase::Settling { until, .. } = self.phase
            && now >= until
        {
            self.phase = Phase::Quiet;
        }
    }

    /// The guard of the current phase, the speech in progress ended (cut by the next one).
    fn take_guard(&mut self, now: Instant) -> Option<SuspendGuard> {
        match std::mem::replace(&mut self.phase, Phase::Quiet) {
            Phase::Quiet => None,
            Phase::Holding { speech, .. } => {
                if let Some(ended) = speech.ended {
                    ended.call(now);
                }
                None
            }
            Phase::Speaking { id, ended, guard, .. } => {
                self.recent.lock().unwrap_or_else(PoisonError::into_inner).ended(id, now);
                if let Some(ended) = ended {
                    ended.call(now);
                }
                guard
            }
            Phase::Settling { guard, .. } => guard,
        }
    }

    /// True while the dictation hears the user speak.
    fn user_speaking(&self) -> bool {
        self.stt.as_ref().is_some_and(Stt::user_speaking)
    }

    /// Reads `speech` now, or holds it while the user speaks (a held one is replaced).
    fn start(&mut self, speech: Speech) {
        let now = self.clock.now();
        let until = match &self.phase {
            Phase::Holding { until, .. } => Some(*until),
            Phase::Quiet if self.user_speaking() => Some(later(now, SPEECH_HOLD)),
            Phase::Quiet | Phase::Speaking { .. } | Phase::Settling { .. } => None,
        };
        match until {
            Some(until) => {
                // The newer speech replaces the held one, which is told it ended.
                self.take_guard(now);
                tracing::debug!("read-aloud: held while the user speaks");
                self.phase = Phase::Holding { speech, until };
            }
            None => self.play(speech),
        }
    }

    /// Reads `speech` at once, cutting the one in progress.
    fn play(&mut self, speech: Speech) {
        let now = self.clock.now();
        let (cut, busy) = match &self.phase {
            Phase::Speaking { id, engine, .. } => (Some((*id, Arc::clone(engine))), true),
            Phase::Settling { .. } => (None, true),
            Phase::Quiet | Phase::Holding { .. } => (None, false),
        };
        let guard = self.take_guard(now);
        let barge = self.stt.as_ref().and_then(Stt::barge_voice);
        let engine = barge.clone().unwrap_or_else(|| Arc::clone(&self.engine));
        let status = self.voices.status(self.engine.as_ref(), &speech.lang);
        let voice = match &status {
            SpeechStatus::Ready { voices } => pick(voices, speech.voice.as_deref()),
            SpeechStatus::Unavailable { reason } => {
                tracing::debug!(?reason, "no voice for the language: nothing read aloud");
                None
            }
        };
        let (Some(voice), Some(next)) = (voice, self.next.checked_add(1)) else {
            // Nothing plays this one: the cut speech stops, then the guard runs from now.
            if let Some((id, engine)) = &cut
                && let Err(e) = engine.stop(*id)
            {
                tracing::debug!(kind = ?e.kind(), "speech not cut");
            }
            if let Some(ended) = speech.ended {
                ended.call(now);
            }
            if busy {
                self.settle_from(now, guard);
            }
            return;
        };
        self.next = next;
        let id = SpeechId::new(next);
        let deadline = later(now, watchdog(&speech.text, speech.rate));
        // Another voice than the cut speech's does not cut it by speaking: stopped here.
        if let Some((old, old_engine)) = &cut
            && !Arc::ptr_eq(old_engine, &engine)
            && let Err(e) = old_engine.stop(*old)
        {
            tracing::debug!(kind = ?e.kind(), "speech not cut");
        }
        let before = {
            let mut deaf = self.deaf.lock().unwrap_or_else(PoisonError::into_inner);
            let before = *deaf;
            if barge.is_some() {
                // The user may talk over this one: the microphone hears all of it.
                *deaf = Deaf { from: None, until: None, playing: true };
            } else {
                // Past the guard, a new deaf window opens, even if the tick to `Quiet` has not
                // run yet: an utterance begun in the old one is then judged heard. Unreachable
                // in practice, since capture stays suspended until that tick.
                if deaf.until.is_none_or(|until| now >= until) && cut.is_none() {
                    deaf.from = Some(now);
                }
                deaf.until = None;
                deaf.playing = true;
            }
            before
        };
        // With barge-in, the guard of a muted speech it cuts goes: listening opens at once.
        let guard = if barge.is_some() { None } else { guard };
        let barge = barge.is_some();
        self.phase = Phase::Speaking {
            id,
            since: now,
            deadline,
            ended: speech.ended,
            guard,
            engine: Arc::clone(&engine),
            barge,
        };
        let me = self.me.clone();
        let done = SpeechDone::new(move |id| {
            if let Some(shared) = me.upgrade() {
                let _ = shared.tx.send(Msg::Done(id));
            }
        });
        // `done` may run inside `speak`: it only sends, the thread reads it next.
        match engine.speak(id, &speech.text, Some(voice), speech.rate, done) {
            // Suspended once the voice took it, so a refusal discards nothing. The deaf window
            // opened before `speak`: what the microphone hears meanwhile is dropped by `hears`.
            Ok(()) => {
                let mut recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
                if speech.asks {
                    // Its answer may repeat the receipt read just before: muted, both share
                    // one deaf window, so forgetting the receipt loses no echo protection.
                    recent.clear();
                } else if !barge {
                    // The echo canceller removes a barge speech: the user may repeat its words.
                    recent.started(id, &speech.text);
                }
                drop(recent);
                if let Phase::Speaking { guard: guard @ None, barge: false, .. } = &mut self.phase {
                    *guard = self.stt.as_ref().map(Stt::suspend);
                }
            }
            Err(e) => {
                tracing::warn!(kind = ?e.kind(), "speech refused by the OS voice");
                // The voice listed may be gone: the next speech lists them again.
                self.voices.clear();
                if busy {
                    self.end(now);
                } else {
                    // Nothing played nor was suspended: listening goes on at once.
                    self.take_guard(now);
                    *self.deaf.lock().unwrap_or_else(PoisonError::into_inner) = before;
                }
            }
        }
    }

    /// The speech in progress ended at `at`: told, then the guard runs.
    fn end(&mut self, at: Instant) {
        let guard = self.take_guard(at);
        self.settle_from(at, guard);
    }

    /// Listening resumes [`SPEECH_GUARD`] after `at`; `guard` is held until then.
    fn settle_from(&mut self, at: Instant, guard: Option<SuspendGuard>) {
        let until = later(at, SPEECH_GUARD);
        {
            let mut deaf = self.deaf.lock().unwrap_or_else(PoisonError::into_inner);
            if deaf.from.is_some() {
                deaf.until = Some(until);
            }
            deaf.playing = false;
        }
        self.phase = Phase::Settling { until, guard };
    }
}

/// Feeds the core the requests and its own deadlines until every speaker is gone.
///
/// A panic is caught and logged: the guard drops with the core, so listening resumes.
fn run(rx: &Receiver<Msg>, mut core: Core) {
    let looped = std::panic::catch_unwind(AssertUnwindSafe(|| {
        loop {
            let msg = match core.deadline() {
                Some(at) => match rx.recv_timeout(at.saturating_duration_since(core.clock.now())) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => Msg::Tick,
                    Err(RecvTimeoutError::Disconnected) => return,
                },
                None => match rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => return,
                },
            };
            core.handle(msg);
        }
    }));
    if looped.is_err() {
        tracing::error!("speaker thread panicked: nothing more is read aloud");
    }
}

#[cfg(test)]
#[path = "speaker_tests.rs"]
mod speaker_tests;
