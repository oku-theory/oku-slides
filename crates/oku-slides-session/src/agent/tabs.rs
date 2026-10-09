//! L'agent et les onglets : la session d'un onglet, fournie par l'hôte.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use crate::Session;

/// Les sessions des onglets, vues par l'agent.
///
/// À implémenter par l'hôte : donne aux outils et tours de l'agent la
/// session d'un onglet (l'espace de travail de l'hôte, ou une session seule
/// dans les tests).
pub trait Sessions: Send + Sync + std::fmt::Debug {
    /// Appelle `f` sur la session de `tab` ; bloquant. Onglet inconnu :
    /// erreur, `f` n'est pas appelée.
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()>;

    /// Vrai si `path` est déjà ouvert dans un autre onglet que `tab`.
    ///
    /// Par défaut `false` : un hôte à un seul document n'a pas d'autre onglet.
    fn open_elsewhere(&self, _tab: &str, _path: &Path) -> bool {
        false
    }

    /// Ouvre `path` dans un nouvel onglet à côté de `tab` (déjà ouvert : son
    /// onglet devient actif) et rend l'onglet du fichier ; bloquant.
    /// L'onglet `tab` et son document restent tels quels.
    ///
    /// Par défaut `SingleDocument` : un hôte à un seul document n'a pas d'autre onglet.
    fn open_tab(&self, _tab: &str, _path: &Path) -> crate::Result<String> {
        Err(crate::error::SingleDocumentSnafu.build().into())
    }
}

/// Une seule session, quel que soit l'onglet (tests, hôtes à un document).
impl Sessions for Mutex<Session> {
    fn with(&self, _tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()> {
        f(&mut self.lock().unwrap_or_else(PoisonError::into_inner));
        Ok(())
    }
}

/// Un onglet et l'accès à sa session.
#[derive(Clone, Debug)]
pub struct TabSession {
    tab: String,
    sessions: Arc<dyn Sessions>,
}

impl TabSession {
    /// Un onglet `tab`, dont les sessions sont fournies par `sessions`.
    pub fn new(tab: impl Into<String>, sessions: Arc<dyn Sessions>) -> Self {
        Self { tab: tab.into(), sessions }
    }

    /// Identifiant de l'onglet.
    pub fn tab(&self) -> &str {
        &self.tab
    }

    /// Vrai si `path` est déjà ouvert dans un autre onglet ; bloquant (verrou).
    pub fn open_elsewhere(&self, path: &Path) -> bool {
        self.sessions.open_elsewhere(&self.tab, path)
    }

    /// Ouvre `path` à côté de cet onglet (voir [`Sessions::open_tab`]) ; bloquant.
    pub fn open_tab(&self, path: &Path) -> crate::Result<String> {
        self.sessions.open_tab(&self.tab, path)
    }

    /// `f` sur la session de l'onglet ; bloquant (verrou) : hors du fil
    /// async.
    pub fn with<R>(&self, f: impl FnOnce(&mut Session) -> R) -> crate::Result<R> {
        let mut f = Some(f);
        let mut out = None;
        self.sessions.with(&self.tab, &mut |s| {
            if let Some(f) = f.take() {
                out = Some(f(s));
            }
        })?;
        // `Sessions::with` appelle `f` au plus une fois ; `Ok` sans appel est
        // une faute de l'implémentation.
        out.ok_or_else(|| {
            crate::error::AgentFormSnafu { reason: super::AgentFormReason::SessionNotCalled { tab: self.tab.clone() } }
                .build()
                .into()
        })
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
