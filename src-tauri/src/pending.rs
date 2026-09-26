//! File des fichiers reçus avant que l'app ne soit prête.
//!
//! `RunEvent::Opened` (macOS) et le rappel de `tauri-plugin-single-instance`
//! tournent hors du fil principal, dès que le greffon démarre : ils peuvent
//! précéder la fin de `setup` (attente de reprise, encore plus sur un
//! lancement à froid). Sans file, un fichier reçu dans cette fenêtre est
//! perdu (`AppState` pas encore géré). `ready`, appelée à la toute fin de
//! `setup`, fait basculer la file en mode direct pour tout ce qui arrive
//! ensuite : plus jamais de fichier mis en attente sans personne pour le
//! reprendre.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

/// Fichiers acceptés par [`Pending::receive`] pour un seul appel ; l'excédent
/// est journalisé et abandonné (protège d'un argument abusif, ou d'une liste
/// macOS anormalement longue).
pub const MAX_RECEIVED: usize = 64;

/// `Some` : app pas encore prête, les fichiers reçus s'y accumulent. `None` :
/// prête (`ready` déjà appelée), les fichiers reçus repartent aussitôt vers
/// leur appelant.
pub struct Pending(Mutex<Option<Vec<PathBuf>>>);

impl Default for Pending {
    fn default() -> Self {
        Self(Mutex::new(Some(Vec::new())))
    }
}

impl Pending {
    /// Fichiers reçus (seconde instance, `RunEvent::Opened`).
    ///
    /// App pas encore prête : mis en file, `None` renvoyé (l'appelant n'a
    /// rien de plus à faire, `ready` les reprendra). Prête : renvoyés tels
    /// quels, à ouvrir par l'appelant (le verrou est déjà rendu).
    ///
    /// Plafonnés à [`MAX_RECEIVED`] par appel.
    pub fn receive(&self, mut files: Vec<PathBuf>) -> Option<Vec<PathBuf>> {
        if files.len() > MAX_RECEIVED {
            tracing::warn!(
                received = files.len(),
                cap = MAX_RECEIVED,
                "trop de fichiers reçus d'un coup : le reste est abandonné"
            );
            files.truncate(MAX_RECEIVED);
        }
        let mut queue = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match queue.as_mut() {
            Some(pending) => {
                pending.extend(files);
                None
            }
            None => Some(files),
        }
    }

    /// Vide la file et bascule en mode direct ; à appeler une seule fois, à
    /// la toute fin de `setup`.
    ///
    /// Rappelée ensuite (jamais en pratique, sinon par erreur) : liste vide,
    /// jamais les mêmes fichiers deux fois.
    pub fn ready(&self) -> Vec<PathBuf> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take().unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "pending_tests.rs"]
mod pending_tests;
