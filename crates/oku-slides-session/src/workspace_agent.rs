//! L'espace de travail vu par l'agent : la session d'un onglet
//! ([`Sessions`]) et l'ouverture d'un fichier à côté de lui.

use std::path::Path;
use std::sync::Mutex;

use oku_slides_platform::storage::resolve;
use snafu::OptionExt;

use crate::Session;
use crate::agent::Sessions;
use crate::error::{Result, UnknownTabSnafu};

use super::Workspace;

impl Workspace {
    /// Ouvre `path` dans un nouvel onglet de la fenêtre de `tab` ; déjà
    /// ouvert, son onglet devient actif. Rend l'onglet du fichier.
    ///
    /// Jamais d'onglet remplacé, même vierge : l'agent de `tab` y converse.
    /// Comme `apply` sur `OpenTab` : l'onglet se construit hors du verrou ;
    /// après [`Workspace::settle`], refusé comme un onglet fermé.
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
            w.pending(Some(window), Some(path), None)
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

    fn open_tab(&self, tab: &str, path: &Path) -> Result<String> {
        Workspace::open_beside(self, tab, path)
    }
}
