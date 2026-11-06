//! Starting a capture: dictation or listening, after the model download when one is missing.
//!
//! [`Stt::launch`] checks the engine (Apple language, API settings, model on disk), then
//! [`Stt::begin_recording`] opens the microphone and the engine's sink and spawns `listen_loop`.
//! A missing model goes through [`Stt::fetch`]; [`Stt::finish_download`] says what starts after it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use oku_slides_i18n::Localize;

use crate::dictation_stream::Held;
use crate::error::{ApiConfigSnafu, ApiKeyMissingSnafu, LocaleUnsupportedSnafu, NotDownloadableSnafu};
use crate::utterance::ListenClock;
use crate::{Download, Mode, Model, Sink, State, Stt, SttError, SttErrorKind, locale, models, valid_api_access};

impl Stt {
    /// Starts dictation (downloading the model first if missing).
    pub fn start(&self, vocabulary: Vec<String>) {
        self.launch(vocabulary, Mode::Dictate, State::Idle);
    }

    /// Starts hands-free listening; a no-op while listening, refused while busy.
    ///
    /// The model is downloaded first if missing, as for [`Self::start`].
    pub fn listen(&self, vocabulary: Vec<String>) {
        self.launch(vocabulary, Mode::Listen, State::Idle);
    }

    /// Starts a capture in `mode` from state `from`, after the model download if needed.
    ///
    /// `from` is `Idle`, or `Listening` for a restart after new settings.
    pub(crate) fn launch(&self, vocabulary: Vec<String>, mode: Mode, from: State) {
        let (model, apple, api) = {
            let mut g = self.state();
            if g.state != from {
                return;
            }
            g.error = None;
            g.vocabulary = vocabulary;
            (g.config.model, g.apple.clone(), g.config.api.clone())
        };
        #[cfg(feature = "e2e-fakes")]
        if mode == Mode::Listen
            && let Some(path) = self.settings.fake_utterances.clone()
        {
            self.begin_fake(path, from);
            return;
        }
        if model == Model::Apple {
            match apple {
                Some((_, a)) if a.is_ready() => self.begin_recording(mode, from),
                Some(_) => {
                    // Nothing more to say: the failure is already in the state.
                    let _started = self.download_language(Some(mode));
                }
                None => self.refuse(&LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into()),
            }
            return;
        }
        if model == Model::Api {
            if !valid_api_access(&api) {
                self.refuse(&ApiConfigSnafu.build().into());
            } else if let Some(var) = api.missing_variable() {
                self.refuse(&ApiKeyMissingSnafu { var }.build().into());
            } else {
                self.begin_recording(mode, from);
            }
            return;
        }
        if model.is_installed(&self.storage, &self.dir) {
            self.begin_recording(mode, from);
        } else {
            // Nothing more to say: the failure is already in the state.
            let _started = self.fetch(model, Some(mode));
        }
    }

    /// A capture cannot start: listening being restarted ends, then the error is shown.
    pub(crate) fn refuse(&self, e: &SttError) {
        {
            let mut g = self.state();
            if g.state == State::Listening {
                g.reset();
            }
        }
        self.report(e);
    }

    /// Starts a model download; false if nothing started.
    ///
    /// While listening, capture stops for the download and starts again after it, whatever its result.
    #[must_use = "false: nothing started (busy, already installed, API)"]
    pub fn download(&self, model: Model, then_start: bool) -> bool {
        self.fetch(model, then_start.then_some(Mode::Dictate))
    }

    /// Downloads `model`, then starts `then` on success; listening, if on, stops and resumes instead.
    pub(crate) fn fetch(&self, model: Model, then: Option<Mode>) -> bool {
        if model == Model::Apple {
            return self.download_language(then);
        }
        if model == Model::Api {
            self.report(&NotDownloadableSnafu.build().into());
            return false;
        }
        {
            let mut g = self.state();
            if !g.state.accepts_settings() || model.is_installed(&self.storage, &self.dir) {
                return false;
            }
            g.pause_for_download(then);
            g.state = State::Downloading;
            g.download = Some(Download { model, received: 0, total: model.size() });
        }
        self.cancel_download.store(false, Ordering::Relaxed);
        let (me, dir, cancel) = (self.clone(), self.dir.clone(), self.cancel_download.clone());
        std::thread::spawn(move || {
            let mut last = Instant::now();
            let result = models::download(model, &dir, &me.models, &me.storage, |received, total| {
                if last.elapsed() > Duration::from_millis(100) {
                    last = Instant::now();
                    me.state().download = Some(Download { model, received, total });
                }
                !cancel.load(Ordering::Relaxed)
            });
            if let Some(mode) = me.finish_download(model, result.map(|_| ())) {
                me.begin_recording(mode, State::Idle);
            }
        });
        true
    }

    /// End of a download: state, message, and the capture to start, if any.
    ///
    /// Listening paused by the download resumes whatever the result, the error still shown (R6).
    /// A capture asked for from idle starts only on success: its model would be missing again.
    pub(crate) fn finish_download(&self, model: Model, result: Result<(), SttError>) -> Option<Mode> {
        let mut g = self.state();
        g.download = None;
        g.state = State::Idle;
        g.installed = Model::ALL.map(|m| m.is_installed(&self.storage, &self.dir));
        let listen = std::mem::take(&mut g.listen_after_download).then_some(Mode::Listen);
        let then = match result {
            Ok(()) => {
                tracing::info!(?model, "{}", self.localizer.tr("log-model-downloaded"));
                g.start_after_download.take()
            }
            Err(e) if e.kind() == SttErrorKind::Cancelled => {
                g.start_after_download = None;
                None
            }
            Err(e) => {
                tracing::warn!(?model, error = %snafu::Report::from_error(&e), "model download");
                g.error = Some(e.localize(&self.localizer));
                g.start_after_download = None;
                None
            }
        };
        listen.or(then)
    }

    /// Opens the microphone and captures in `mode`, unless the state left `from` meanwhile.
    pub(crate) fn begin_recording(&self, mode: Mode, from: State) {
        let (recorder, barge) = match self.open_microphone(mode) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %snafu::Report::from_error(&e), "micro indisponible");
                let mut g = self.state();
                if g.state == State::Listening {
                    g.reset(); // a restart that cannot capture: listening ends, visibly
                }
                g.error = Some(e.localize(&self.localizer));
                return;
            }
        };
        let (session, model, apple, vocabulary) = {
            let mut g = self.state();
            if g.state != from {
                // Another capture started meanwhile: the recorder drops, its microphone closes.
                return;
            }
            g.session += 1;
            g.state = match mode {
                Mode::Dictate => State::Recording,
                Mode::Listen => State::Listening,
            };
            g.phrases.discard();
            g.partial.clear();
            g.detected = None;
            if g.config.model == Model::Apple {
                g.detected = g.apple.as_ref().map(|(tag, _)| locale::language(tag));
            }
            g.stop_in = None;
            g.barge = barge;
            g.started = Some(Instant::now());
            // A clock left by an earlier listening would misplace this stream's finals.
            g.clock = ListenClock { origin: Instant::now(), stream_base: 0 };
            (g.session, g.config.model, g.apple.clone(), g.vocabulary.clone())
        };
        let sink = match (model, apple) {
            (Model::Apple, Some((tag, _))) => {
                let timeout = match self.apple_finish_timeout() {
                    Ok(timeout) => timeout,
                    Err(e) => {
                        self.state().reset();
                        self.report(&e);
                        return;
                    }
                };
                let (stream, events) = self.streaming.open(&tag, &vocabulary);
                let held = Held::default();
                let (worker, worker_held) = (self.clone(), held.clone());
                std::thread::spawn(move || worker.stream_loop(session, &events, &worker_held, timeout));
                Sink::Stream { stream, held }
            }
            (Model::Apple, None) => {
                // `start` prevents it; to be safe, no listening without a language.
                self.state().reset();
                self.report(&LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into());
                return;
            }
            _ => {
                let (jobs, rx) = mpsc::channel();
                let busy = Arc::new(AtomicBool::new(false));
                let worker = self.clone();
                let worker_busy = busy.clone();
                std::thread::spawn(move || worker.transcribe_loop(session, &rx, &worker_busy));
                Sink::Whisper { jobs, busy, previews: model.previews() }
            }
        };
        let listener = self.clone();
        std::thread::spawn(move || listener.listen_loop(session, recorder, sink));
    }
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod capture_tests;
