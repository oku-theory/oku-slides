//! La langue de l'interface, changée à chaud et lue à chaque message.

use std::sync::{Arc, PoisonError, RwLock};

use crate::Localizer;

/// Langue courante de l'interface, partagée entre ses clones.
///
/// Un changement de langue (`set`) vaut pour tous les clones : chaque lecture
/// (`current`) rend la langue du moment, jamais celle du lancement.
#[derive(Debug, Clone)]
pub struct LiveLocalizer(Arc<RwLock<Arc<Localizer>>>);

impl LiveLocalizer {
    /// Une langue courante qui commence par `localizer`.
    pub fn new(localizer: Arc<Localizer>) -> Self {
        Self(Arc::new(RwLock::new(localizer)))
    }

    /// La langue du moment ; un changement ensuite ne la modifie pas.
    pub fn current(&self) -> Arc<Localizer> {
        self.0.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Passe toute l'app à `localizer`, pour les messages suivants.
    pub fn set(&self, localizer: Arc<Localizer>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = localizer;
    }
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod live_tests;
