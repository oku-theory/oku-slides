//! The workspace as the agent sees it: a tab's session ([`Sessions`]),
//! a file opened beside it, and a pack opened from it.

use std::path::Path;
use std::sync::Mutex;

use oku_slides_platform::storage::resolve;
use snafu::OptionExt;
use uuid::Uuid;

use crate::Session;
use crate::agent::{PackGate, PackOpen, Sessions};
use crate::error::{Result, UnknownTabSnafu};

use super::{Opening, TabPlace, Workspace};

impl Workspace {
    /// The window of `tab`, which opens something; refused once closed.
    ///
    /// The prelude of `open_beside` and `open_pack_with`, under the workspace lock.
    pub(super) fn opening_window(&self, tab: &str) -> Result<String> {
        if self.is_closed() {
            return Err(UnknownTabSnafu { id: tab }.build().into());
        }
        Ok(self.tab_window(tab).context(UnknownTabSnafu { id: tab })?.to_owned())
    }

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
            let window = w.opening_window(tab)?;
            let path = resolve(path);
            if let Some(id) = w.tab_with_path(&path) {
                w.activate(&id)?;
                w.save_layout();
                return Ok(id);
            }
            w.pending(Some(window), Opening::File(path), TabPlace::End)
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

    fn open_pack(
        &self,
        tab: &str,
        pack: Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(PackGate) -> bool,
    ) -> Result<PackOpen> {
        Workspace::open_pack(self, tab, pack, widget, gate)
    }
}
