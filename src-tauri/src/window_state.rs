//! Taille et position des fenêtres, enregistrées une fois le geste fini.
//!
//! Déplacer une fenêtre émet un `WindowEvent::Moved` par pixel : `Saver`
//! n'enregistre qu'après `QUIET` sans mouvement (`Debounce`), sur son propre
//! fil. Une fenêtre détruite entre-temps ne répond plus : sans gravité, elle
//! est ignorée (`is_window_gone`). La sortie de l'app enregistre aussitôt
//! (`save`, depuis `state::shutdown`), sans attendre ce calme.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// Calme attendu avant d'enregistrer.
const QUIET: Duration = Duration::from_millis(500);

/// Lance une action une fois les appels calmés depuis `delay`.
///
/// Un fil dédié ; il s'arrête quand `Debounce` disparaît, après l'action en
/// attente s'il y en a une.
#[derive(Debug)]
pub struct Debounce {
    tx: Sender<()>,
}

impl Debounce {
    /// Démarre le fil ; `action` y tourne.
    pub fn new(delay: Duration, action: impl Fn() + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            while rx.recv().is_ok() {
                loop {
                    match rx.recv_timeout(delay) {
                        Ok(()) => {}
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return action(),
                    }
                }
                action();
            }
        });
        Self { tx }
    }

    /// Un changement : l'action repart pour `delay`.
    pub fn ping(&self) {
        // Fil arrêté : plus rien à enregistrer, l'app s'en va.
        let _ = self.tx.send(());
    }
}

/// L'erreur vient d'une fenêtre qui ne répond plus (en cours de destruction).
pub fn is_window_gone(e: &tauri_plugin_window_state::Error) -> bool {
    matches!(
        e,
        tauri_plugin_window_state::Error::Tauri(tauri::Error::Runtime(tauri_runtime::Error::FailedToReceiveMessage))
    )
}

/// Enregistre tout de suite l'état de toutes les fenêtres.
///
/// Aussi appelée par `state::shutdown`, tant que les fenêtres existent : à
/// `RunEvent::Exit`, le greffon n'en trouve souvent plus aucune, et un
/// mouvement encore en attente ici serait perdu.
pub fn save(app: &AppHandle) {
    match app.save_window_state(StateFlags::all()) {
        Ok(()) => {}
        Err(e) if is_window_gone(&e) => {
            tracing::debug!(error = %e, "état des fenêtres : une fenêtre fermée ignorée");
        }
        Err(e) => tracing::warn!(error = %e, "état de la fenêtre non enregistré"),
    }
}

/// Enregistre l'état de toutes les fenêtres, une fois leurs mouvements calmés.
#[derive(Debug)]
pub struct Saver(Debounce);

impl Saver {
    /// Enregistreur pour les fenêtres de `app`.
    pub fn new(app: AppHandle) -> Self {
        Self(Debounce::new(QUIET, move || save(&app)))
    }

    /// Une fenêtre a bougé ou changé de taille.
    pub fn changed(&self) {
        self.0.ping();
    }
}

#[cfg(test)]
#[path = "window_state_tests.rs"]
mod window_state_tests;
