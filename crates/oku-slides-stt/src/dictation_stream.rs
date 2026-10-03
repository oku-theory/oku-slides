//! Dictée par un moteur en continu (Apple) : ses résultats, sa langue, un audio complet.
//!
//! `listen_loop` (lib.rs) pousse l'audio dans le [`Stream`](crate::stream::Stream) ;
//! [`Stt::stream_loop`] applique ses événements à l'état, jusqu'à `Done`.
//! À l'arrêt, `listen_loop` lui confie le flux ([`Held`]) : il reste annulable
//! jusqu'à sa fin.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use oku_slides_i18n::Localize;

use crate::error::{AnalyzerSnafu, AssetInstallSnafu, CancelledSnafu, FinishTimeoutSnafu, LocaleUnsupportedSnafu};
use crate::stream::{Availability, Event, Stream};
use crate::{Download, Inner, Model, SAMPLE_RATE, State, Stt, SttError, TimedPhrase, clean, join, results};

/// Flux confié par `listen_loop` après `finish`, gardé jusqu'à sa fin.
pub(crate) type Held = Arc<Mutex<Option<Box<dyn Stream>>>>;

/// Annule le flux confié, s'il y en a un (sans tenir le verrou pendant l'appel).
pub(crate) fn cancel_held(held: &Held) {
    let stream = held.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(mut stream) = stream {
        stream.cancel();
    }
}

impl Stt {
    /// Résultats d'un moteur en continu, jusqu'à la fin de la dictée.
    ///
    /// Une dictée abandonnée ou trop lente annule le flux confié ; il n'est
    /// lâché tel quel qu'après `Done`, `Failed` ou la fermeture du pont.
    pub(crate) fn stream_loop(&self, session: u64, events: mpsc::Receiver<Event>, held: Held) {
        let tick = Duration::from_millis(self.settings.tick_ms);
        let timeout = Duration::from_secs_f32(self.settings.apple_finish_timeout_secs);
        let mut finishing: Option<Instant> = None;
        loop {
            let event = events.recv_timeout(tick);
            let mut g = self.state();
            if g.session != session {
                drop(g);
                cancel_held(&held); // dictée abandonnée
                return;
            }
            match event {
                Ok(Event::Done) => {
                    g.complete();
                    return;
                }
                Ok(Event::Failed(e)) => {
                    self.fail(&mut g, &e);
                    return;
                }
                Ok(event) => {
                    let Inner { phrases, partial, .. } = &mut *g;
                    results::apply(phrases, partial, &event);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.fail(&mut g, &AnalyzerSnafu { reason: "bridge closed" }.build().into());
                    return;
                }
            }
            // Arrêt demandé : le texte final doit arriver à temps.
            if g.state == State::Transcribing {
                let since = *finishing.get_or_insert_with(Instant::now);
                if since.elapsed() >= timeout {
                    let e: SttError = FinishTimeoutSnafu.build().into();
                    tracing::warn!(error = %e, "dictée en continu");
                    g.error = Some(e.localize(&self.localizer));
                    g.complete(); // le texte déjà définitif est gardé
                    drop(g);
                    cancel_held(&held);
                    return;
                }
            }
        }
    }

    /// Le moteur a échoué : message, puis retour au repos sans texte.
    fn fail(&self, g: &mut Inner, e: &SttError) {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(e), "dictée en continu");
        g.error = Some(e.localize(&self.localizer));
        g.reset();
    }

    /// Télécharge la langue du moteur d'Apple (par le système), puis reprend la dictée.
    pub(crate) fn download_language(&self, then_start: bool) {
        let tag = {
            let mut g = self.state();
            let Some((tag, _)) = g.apple.clone() else {
                drop(g);
                // Aucune langue connue du moteur : rien à télécharger, on le dit.
                self.report(LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into());
                return;
            };
            if g.state != State::Idle {
                return;
            }
            g.state = State::Downloading;
            g.download = Some(Download { model: Model::Apple, received: 0, total: 0 });
            g.start_after_download = then_start;
            tag
        };
        self.cancel_download.store(false, Ordering::Relaxed);
        let me = self.clone();
        std::thread::spawn(move || {
            let result = me.install(&tag);
            me.refresh_apple();
            if me.finish_download(Model::Apple, result) {
                me.begin_recording();
            }
        });
    }

    /// Attend l'installation d'une langue ; la progression va dans l'état.
    ///
    /// L'annulation est vue à chaque tour : une progression continue ne la masque pas.
    fn install(&self, tag: &str) -> Result<(), SttError> {
        let events = self.streaming.install(tag);
        loop {
            match events.recv_timeout(Duration::from_millis(self.settings.tick_ms)) {
                Ok(Event::Progress { received, total }) => {
                    self.state().download = Some(Download { model: Model::Apple, received, total });
                }
                Ok(Event::Done) => return Ok(()),
                Ok(Event::Failed(e)) => return Err(e),
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return AssetInstallSnafu { reason: "bridge closed" }.fail()?,
            }
            if self.cancel_download.load(Ordering::Relaxed) {
                return CancelledSnafu.fail()?;
            }
        }
    }

    /// Transcrit un audio complet (mono 16 kHz) par le moteur en continu.
    pub(crate) fn transcribe_stream(&self, audio: &[f32]) -> Result<(String, Vec<TimedPhrase>), SttError> {
        // Un ancien abandon (`cancel`) ne doit pas interrompre cette installation.
        self.cancel_download.store(false, Ordering::Relaxed);
        let (apple, vocabulary) = {
            let g = self.state();
            (g.apple.clone(), g.vocabulary.clone())
        };
        let Some((tag, availability)) = apple else {
            return LocaleUnsupportedSnafu { locale: self.wanted_language() }.fail()?;
        };
        if availability == Availability::Missing {
            let installed = self.install(&tag);
            self.state().download = None;
            installed?;
            self.refresh_apple();
        }
        let (mut stream, events) = self.streaming.open(&tag, &vocabulary);
        for chunk in audio.chunks(SAMPLE_RATE as usize / 10) {
            stream.push(chunk);
        }
        stream.finish();
        let timeout = Duration::from_secs_f32(self.settings.apple_finish_timeout_secs);
        let mut phrases: Vec<TimedPhrase> = Vec::new();
        let ended: Result<(), SttError> = loop {
            match events.recv_timeout(timeout) {
                Ok(Event::Final { text, start, end }) => {
                    let text = clean(&text);
                    if !text.is_empty() {
                        phrases.push((start, end, text));
                    }
                }
                Ok(Event::Done) => break Ok(()),
                Ok(Event::Failed(e)) => break Err(e),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => break Err(FinishTimeoutSnafu.build().into()),
                Err(RecvTimeoutError::Disconnected) => {
                    break Err(AnalyzerSnafu { reason: "bridge closed" }.build().into());
                }
            }
        };
        if ended.is_err() {
            stream.cancel(); // plus rien d'utile n'en viendra
        }
        ended?;
        let text = join(phrases.iter().map(|(_, _, t)| t));
        Ok((text, phrases))
    }
}

#[cfg(test)]
#[path = "dictation_stream_tests.rs"]
mod dictation_stream_tests;
