//! L'espace de travail vu par l'agent : la session d'un onglet
//! ([`Sessions`]) et l'ouverture d'un fichier à côté de lui.

use std::path::Path;
use std::sync::Mutex;

use oku_slides_platform::storage::resolve;
use snafu::OptionExt;

use crate::Session;
use crate::agent::Sessions;
use crate::error::{Result, UnknownTabSnafu};

use super::{Opening, Workspace};

impl Workspace {
    /// Opens `path` in a new tab of `tab`'s window and returns the file's tab.
    ///
    /// If already open, its tab becomes active. Never replaces a tab, even a
    /// blank one: the agent of `tab` converses there. Like `apply` on
    /// `OpenTab`: the tab is built outside the lock; after
    /// [`Workspace::settle`], rejected like a closed tab.
    ///
    /// # Errors
    ///
    /// Fails after `settle` or for an unknown tab (`UnknownTab`), or if the file's session cannot be
    /// built.
    pub fn open_beside(ws: &Mutex<Self>, tab: &str, path: &Path) -> Result<String> {
        let pending = {
            let mut w = Self::lock(ws);
            if w.is_closed() {
                return Err(UnknownTabSnafu { id: tab }.build().into());
            }
            let window = w.tab_window(tab).context(UnknownTabSnafu { id: tab })?.to_owned();
            let path = resolve(path);
            if let Some(id) = w.tab_with_path(&path) {
                w.activate(&id)?;
                w.save_layout();
                return Ok(id);
            }
            w.pending(Some(window), Opening::File(path), None)
        };
        let session = pending.build()?;
        Self::finish(ws, pending, session).map(|(_, id)| id)
    }
}

/// L'agent atteint les sessions des onglets par l'espace de travail ; la
/// barre à redessiner est ignorée ici (l'hôte qui la veut appelle
/// [`Workspace::with_session`], et redessine après `open_tab`).
impl Sessions for Mutex<Workspace> {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> Result<()> {
        Workspace::with_session(self, tab, |s| f(s)).0
    }

    fn open_elsewhere(&self, tab: &str, path: &Path) -> bool {
        Workspace::lock(self).open_elsewhere(tab, path)
    }

    fn shown(&self, tab: &str) -> bool {
        Workspace::lock(self).tab_shown(tab)
    }

    fn open_tab(&self, tab: &str, path: &Path) -> Result<String> {
        Workspace::open_beside(self, tab, path)
    }
}
