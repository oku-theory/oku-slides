//! Téléchargement du modèle de l'agent local, sur un fil à part, annulable.
//!
//! Le panneau lit l'état ([`ModelDownload::state`]) ; le fil le met à jour et prévient à
//! chaque pourcent puis à la fin, pour que la pile réémette le panneau et la disponibilité.
//! Ces réémissions ne vont qu'à l'onglet qui a lancé le téléchargement (le lot 5 le change).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use oku_slides_i18n::Localizer;
use oku_slides_platform::config::ModelsSettings;
use oku_slides_platform::models::{self, DownloadErrorKind, PinnedFile};
use oku_slides_platform::storage::Storage;

/// Où en est le téléchargement du modèle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DownloadState {
    /// Rien ne tourne : jamais lancé, ou fini avec succès.
    #[default]
    Idle,
    /// En cours : `received` octets sur `total`.
    Running { received: u64, total: u64 },
    /// Le dernier téléchargement a échoué (ou a été annulé).
    Failed(DownloadErrorKind),
}

/// Le pourcentage reçu, de 0 à 100 ; un total nul compte pour un octet.
pub(crate) fn percent(received: u64, total: u64) -> u8 {
    // Borné à 100 : le cast ne tronque pas.
    (received.saturating_mul(100) / total.max(1)).min(100) as u8
}

/// Le téléchargement du modèle, partagé entre le panneau et son fil.
#[derive(Debug, Clone, Default)]
pub(crate) struct ModelDownload {
    state: Arc<Mutex<DownloadState>>,
    cancel: Arc<AtomicBool>,
}

impl ModelDownload {
    /// Lance le téléchargement si rien ne tourne ; `on_change` est appelé à chaque pourcent et à la fin.
    ///
    /// Faux si un téléchargement tourne déjà : rien n'est lancé.
    #[expect(clippy::too_many_arguments, reason = "tout ce que `models::download` lit, passé au fil")]
    pub(crate) fn start(
        &self,
        files: &'static [PinnedFile],
        dir: PathBuf,
        total: u64,
        settings: ModelsSettings,
        storage: Storage,
        on_change: impl Fn() + Send + 'static,
    ) -> bool {
        {
            let mut state = self.lock();
            if matches!(*state, DownloadState::Running { .. }) {
                return false;
            }
            *state = DownloadState::Running { received: 0, total };
            self.cancel.store(false, Ordering::Relaxed);
        }
        let (state, cancel) = (self.state.clone(), self.cancel.clone());
        std::thread::spawn(move || {
            let mut shown = 0u8;
            let done = models::download(files, &dir, total, &settings, &storage, |received, total| {
                *state.lock().unwrap_or_else(PoisonError::into_inner) = DownloadState::Running { received, total };
                let percent = percent(received, total);
                if percent != shown {
                    shown = percent;
                    on_change();
                }
                !cancel.load(Ordering::Relaxed)
            });
            let end = match done {
                Ok(()) => DownloadState::Idle,
                Err(e) => {
                    tracing::warn!(dir = %dir.display(), error = %snafu::Report::from_error(&e), "téléchargement du modèle");
                    DownloadState::Failed(e.kind())
                }
            };
            *state.lock().unwrap_or_else(PoisonError::into_inner) = end;
            on_change();
        });
        true
    }

    /// Demande l'arrêt du téléchargement en cours ; il finit en `Failed(Cancelled)`.
    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// L'état courant.
    pub(crate) fn state(&self) -> DownloadState {
        *self.lock()
    }

    fn lock(&self) -> MutexGuard<'_, DownloadState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Le texte montré pour un téléchargement échoué de nature `kind`.
pub(crate) fn failure_text(kind: DownloadErrorKind, l: &Localizer) -> String {
    match kind {
        DownloadErrorKind::Network | DownloadErrorKind::Write => l.tr("agent-model-download-failed"),
        DownloadErrorKind::Integrity => l.tr("agent-model-download-integrity"),
        DownloadErrorKind::Incomplete { got, expected } => {
            l.tr_with("agent-model-download-incomplete", &[("got", got.into()), ("expected", expected.into())])
        }
        DownloadErrorKind::Cancelled => l.tr("agent-model-download-cancelled"),
    }
}

#[cfg(test)]
#[path = "model_download_tests.rs"]
mod model_download_tests;
