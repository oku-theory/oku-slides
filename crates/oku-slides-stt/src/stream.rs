//! Moteurs de dictée en continu : on leur pousse l'audio, ils répondent par des événements.
//!
//! - [`StreamEngine`] : un moteur (Apple sur macOS 26+, voir `apple.rs`).
//! - [`Stream`] : une dictée ouverte sur ce moteur.
//! - [`Event`] : ses résultats, traduits du pont par [`Event::from_raw`].

use std::sync::{Arc, mpsc};

use oku_slides_platform::config::SttSettings;

#[cfg(any(test, target_os = "macos"))]
use crate::error::{AnalyzerSnafu, AssetInstallSnafu, SpeechDeniedSnafu};
use crate::error::{LocaleUnsupportedSnafu, SttError};
#[cfg(any(test, target_os = "macos"))]
use crate::num::{narrow_f32, trunc_u64};

/// Disponibilité d'une langue pour un moteur en continu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Availability {
    /// Moteur absent, ou langue inconnue de lui.
    Unsupported,
    /// Langue connue, à télécharger.
    Missing,
    /// Language installed (only the Apple engine, or a test fake, has one).
    #[cfg(any(test, target_os = "macos"))]
    Ready,
}

impl Availability {
    /// The language is installed (never off macOS, outside tests).
    pub(crate) fn is_ready(self) -> bool {
        match self {
            #[cfg(any(test, target_os = "macos"))]
            Availability::Ready => true,
            Availability::Unsupported | Availability::Missing => false,
        }
    }
}

/// Ce qu'un moteur en continu rend.
#[derive(Debug)]
pub(crate) enum Event {
    /// Texte provisoire de ce qui suit le dernier texte définitif.
    #[cfg(any(test, target_os = "macos"))]
    Partial(String),
    /// Texte définitif, et sa place dans l'audio (s).
    #[cfg(any(test, target_os = "macos"))]
    Final {
        text: String,
        start: f32,
        end: f32,
    },
    /// Téléchargement d'une langue.
    #[cfg(any(test, target_os = "macos"))]
    Progress {
        received: u64,
        total: u64,
    },
    /// Plus rien ne viendra.
    #[cfg(any(test, target_os = "macos"))]
    Done,
    Failed(SttError),
}

/// Codes du pont Swift (`swift/Sources/OkuSpeech/OkuSpeech.swift`).
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod raw {
    pub(crate) const PARTIAL: i32 = 0;
    pub(crate) const FINAL: i32 = 1;
    pub(crate) const PROGRESS: i32 = 2;
    pub(crate) const DONE: i32 = 3;
    pub(crate) const ERROR: i32 = 4;
    pub(crate) const ERR_ANALYZER: f64 = 1.0;
    pub(crate) const ERR_LOCALE: f64 = 2.0;
    pub(crate) const ERR_ASSET: f64 = 3.0;
    pub(crate) const ERR_DENIED: f64 = 4.0;
}

#[cfg(any(test, target_os = "macos"))]
impl Event {
    /// Traduit un événement brut du pont (`a`, `b` : temps, compteurs ou code d'erreur).
    pub(crate) fn from_raw(kind: i32, text: String, a: f64, b: f64) -> Event {
        match kind {
            raw::PARTIAL => Event::Partial(text),
            raw::FINAL => Event::Final { text, start: narrow_f32(a), end: narrow_f32(b) },
            raw::PROGRESS => Event::Progress { received: trunc_u64(a.max(0.0)), total: trunc_u64(b.max(0.0)) },
            raw::DONE => Event::Done,
            raw::ERROR => Event::Failed(bridge_error(a, text)),
            _ => Event::Failed(AnalyzerSnafu { reason: format!("unknown bridge event {kind}: {text}") }.build().into()),
        }
    }
}

/// Erreur du pont : son code, et son message (une langue pour `ERR_LOCALE`).
#[cfg(any(test, target_os = "macos"))]
fn bridge_error(code: f64, message: String) -> SttError {
    if code == raw::ERR_LOCALE {
        LocaleUnsupportedSnafu { locale: message }.build().into()
    } else if code == raw::ERR_ASSET {
        AssetInstallSnafu { reason: message }.build().into()
    } else if code == raw::ERR_DENIED {
        SpeechDeniedSnafu.build().into()
    } else if code == raw::ERR_ANALYZER {
        AnalyzerSnafu { reason: message }.build().into()
    } else {
        AnalyzerSnafu { reason: format!("{message} (bridge code {code})") }.build().into()
    }
}

/// Une dictée ouverte sur un moteur en continu.
pub(crate) trait Stream: Send {
    /// Audio mono à 16 kHz.
    fn push(&mut self, samples: &[f32]);
    /// Fin de l'audio : les derniers textes définitifs, puis `Done`.
    fn finish(&mut self);
    /// Abandon : plus rien d'utile ne viendra.
    fn cancel(&mut self);
}

/// Un moteur de dictée en continu.
///
/// Implémentez ce trait pour brancher un moteur qui transcrit au fil de
/// l'audio (le moteur d'Apple ; un moteur factice dans les tests).
pub(crate) trait StreamEngine: Send + Sync {
    /// Le moteur existe sur cette machine.
    fn supported(&self) -> bool;
    /// Disponibilité de la langue `locale` (`fr`, `fr-FR`).
    fn availability(&self, locale: &str) -> Availability;
    /// Installe la langue : `Progress`…, puis `Done` ou `Failed`.
    fn install(&self, locale: &str) -> mpsc::Receiver<Event>;
    /// Ouvre une dictée ; ses résultats arrivent sur le canal rendu.
    fn open(&self, locale: &str, vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>);
}

/// Le moteur en continu de cette machine, selon les réglages.
pub(crate) fn engine(settings: &SttSettings) -> Arc<dyn StreamEngine> {
    #[cfg(target_os = "macos")]
    if settings.apple {
        return Arc::new(crate::apple::AppleEngine);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = settings;
    Arc::new(NoStreamEngine)
}

/// Pas de moteur en continu (hors macOS, ou `stt.apple = false`).
#[derive(Debug)]
pub(crate) struct NoStreamEngine;

impl StreamEngine for NoStreamEngine {
    fn supported(&self) -> bool {
        false
    }

    fn availability(&self, _locale: &str) -> Availability {
        Availability::Unsupported
    }

    fn install(&self, locale: &str) -> mpsc::Receiver<Event> {
        unsupported(locale)
    }

    fn open(&self, locale: &str, _vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>) {
        (Box::new(NoStream), unsupported(locale))
    }
}

/// Un canal qui ne dit qu'une chose : langue inconnue.
fn unsupported(locale: &str) -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    tx.send(Event::Failed(LocaleUnsupportedSnafu { locale }.build().into())).ok();
    rx
}

#[derive(Debug)]
struct NoStream;

impl Stream for NoStream {
    fn push(&mut self, _samples: &[f32]) {}
    fn finish(&mut self) {}
    fn cancel(&mut self) {}
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;
