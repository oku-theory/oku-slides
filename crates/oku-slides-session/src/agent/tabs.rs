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
    /// Calls `f` on the session of `tab`; blocking.
    ///
    /// Unknown tab: an error, and `f` is not called.
    ///
    /// # Errors
    ///
    /// Fails if tab `tab` is unknown to the host.
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()>;

    /// Vrai si `path` est déjà ouvert dans un autre onglet que `tab`.
    ///
    /// Par défaut `false` : un hôte à un seul document n'a pas d'autre onglet.
    fn open_elsewhere(&self, _tab: &str, _path: &Path) -> bool {
        false
    }

    /// Vrai si `tab` est l'onglet affiché de sa fenêtre : seul il applique les
    /// évènements de l'agent (présenter, interface, historique).
    ///
    /// Par défaut `true` : un hôte à un seul document affiche toujours le sien. Tout hôte à
    /// plusieurs onglets la remplace (l'app, `serve`), sans quoi un onglet caché les recevrait.
    fn shown(&self, _tab: &str) -> bool {
        true
    }

    /// Opens `path` in a new tab next to `tab`, returning that tab; blocking.
    ///
    /// If the file is already open, its tab becomes active. Tab `tab` and its document stay as they
    /// are.
    ///
    /// Defaults to `SingleDocument`: a single-document host has no other tab.
    ///
    /// # Errors
    ///
    /// Fails if the host has only one document (`SingleDocument`, the default) or if the tab does not
    /// open.
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
    #[must_use]
    pub fn tab(&self) -> &str {
        &self.tab
    }

    /// Vrai si `path` est déjà ouvert dans un autre onglet ; bloquant (verrou).
    #[must_use]
    pub fn open_elsewhere(&self, path: &Path) -> bool {
        self.sessions.open_elsewhere(&self.tab, path)
    }

    /// L'onglet est-il affiché (voir [`Sessions::shown`]) ? Bloquant (verrou).
    #[must_use]
    pub fn shown(&self) -> bool {
        self.sessions.shown(&self.tab)
    }

    /// Opens `path` next to this tab (see [`Sessions::open_tab`]); blocking.
    ///
    /// # Errors
    ///
    /// Fails like [`Sessions::open_tab`]: single-document host, or a tab or file that does not open.
    pub fn open_tab(&self, path: &Path) -> crate::Result<String> {
        self.sessions.open_tab(&self.tab, path)
    }

    /// Runs `f` on the tab's session; blocking (lock), so off the async thread.
    ///
    /// # Errors
    ///
    /// Fails if the tab is unknown to the host, or if the host did not call `f`.
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
