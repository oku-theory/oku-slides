//! La veille de la dictée : un fil qui signale ce qui change la boîte.

use std::sync::Weak;
use std::time::Duration;

use oku_slides_stt::{Config, Model, State, Status, Stt};

use crate::voice_port::AppVoice;

/// Le rythme de la veille.
const WATCH: Duration = Duration::from_millis(400);

/// Ce qui change la boîte dans l'état de la dictée.
#[derive(Debug, PartialEq)]
pub(crate) struct Fingerprint {
    state: State,
    config: Config,
    models: Vec<(Model, bool, bool)>,
    percent: Option<u64>,
    detected: Option<String>,
}

impl Fingerprint {
    /// L'empreinte d'un état de la dictée.
    pub(crate) fn of(s: &Status) -> Self {
        Self {
            state: s.state,
            config: s.config.clone(),
            models: s.models.iter().map(|m| (m.id, m.installed, m.available)).collect(),
            percent: s.download.as_ref().map(|d| d.received.saturating_mul(100) / d.total.max(1)),
            detected: s.detected.clone(),
        }
    }
}

/// Lance la veille : toutes les 400 ms, un changement d'empreinte fait avancer la révision.
///
/// Le fil s'arrête quand la boîte disparaît. Une panique est journalisée, pas rattrapée.
pub(crate) fn spawn(stt: Stt, voice: Weak<AppVoice>) {
    let started = std::thread::Builder::new().name("oku-voice-watch".into()).spawn(move || {
        let _exit = ExitLog;
        let mut last = Fingerprint::of(&stt.status());
        loop {
            std::thread::sleep(WATCH);
            let Some(voice) = voice.upgrade() else { return };
            let now = Fingerprint::of(&stt.status());
            if now != last {
                last = now;
                voice.bump();
            }
        }
    });
    if let Err(e) = started {
        tracing::warn!(error = %e, "veille de la dictée non lancée : la boîte suit seulement les réglages");
    }
}

/// Journalise la fin du fil de veille quand elle vient d'une panique.
struct ExitLog;

impl Drop for ExitLog {
    fn drop(&mut self) {
        if std::thread::panicking() {
            tracing::error!("veille de la dictée arrêtée par une panique");
        }
    }
}

#[cfg(test)]
#[path = "stt_watch_tests.rs"]
mod stt_watch_tests;
