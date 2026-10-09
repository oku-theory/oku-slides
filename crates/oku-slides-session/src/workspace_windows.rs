//! Les fenêtres : fermer une fenêtre et ses onglets.

use std::sync::MutexGuard;

use super::{Result, Tab, Workspace, WorkspaceOutcome};

impl Workspace {
    /// L'agent est-il dans sa propre fenêtre ?
    pub fn agent_detached(&self) -> bool {
        self.layout.agent_detached
    }

    /// Détache (`true`) ou rattache l'agent ; écrit par `apply`, comme tout changement.
    pub(super) fn set_agent_detached(&mut self, detached: bool) {
        self.layout.agent_detached = detached;
    }

    /// Ferme `window` et ses onglets, sauf si c'est la seule (refusée, sans y toucher).
    ///
    /// Prend le verrou : les dossiers des onglets fermés sont supprimés
    /// après l'écriture de `workspace.json`, verrou rendu.
    pub(super) fn close_window(
        mut w: MutexGuard<'_, Self>,
        window: &str,
        discard: &[String],
    ) -> Result<WorkspaceOutcome> {
        let idx = w.window_index(window)?;
        if w.is_only_window(window) {
            return w.view_json().map(WorkspaceOutcome::LastWindow);
        }
        for tab in &w.layout.windows[idx].tabs {
            w.ensure_closable(tab, discard.contains(tab))?;
        }
        let win = w.layout.windows.remove(idx);
        w.forget_window(window);
        let gone: Vec<(String, Tab)> =
            win.tabs.into_iter().filter_map(|id| w.tabs.remove(&id).map(|t| (id, t))).collect();
        w.save_layout();
        let (json, trash) = (w.view_json(), w.trash());
        drop(w);
        trash.empty(gone);
        json.map(WorkspaceOutcome::Applied)
    }
}
