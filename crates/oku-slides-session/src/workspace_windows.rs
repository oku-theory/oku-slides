//! Windows: closing a window with its tabs and the tabs linked to them.

use std::sync::Mutex;

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

    /// Closes `window` and its tabs, with the tabs linked to its decks.
    ///
    /// The only window is refused, untouched; so is a close that would
    /// empty every window (`LastWindow`, nothing closed, a discarded tab
    /// then never flushed at quit and its unsent save dropped). Otherwise the deck-pack tabs it takes
    /// are flushed outside the lock first, then each tab passes
    /// `ensure_closable` (a tab linked to a discarded deck is discarded too),
    /// linked tabs in other windows close as well (an emptied window goes),
    /// and a closed deck-pack tab whose owner stays is unlinked, lock released.
    /// The folders of closed tabs are deleted after `workspace.json` is written.
    pub(super) fn close_window(ws: &Mutex<Self>, window: &str, discard: &[String]) -> Result<WorkspaceOutcome> {
        let (closing, discarded) = {
            let w = Self::lock(ws);
            w.window_index(window)?;
            if w.is_only_window(window) {
                return w.view_json().map(WorkspaceOutcome::LastWindow);
            }
            w.window_closing(window, discard)
        };
        Self::flush_closing(ws, &closing, &discarded);
        let mut w = Self::lock(ws);
        let idx = w.window_index(window)?;
        if w.is_only_window(window) {
            return w.view_json().map(WorkspaceOutcome::LastWindow);
        }
        let (closing, discarded) = w.window_closing(window, discard);
        if w.tab_ids().iter().all(|id| closing.contains(id)) {
            let linked =
                discarded.iter().filter(|id| w.tabs.get(*id).is_some_and(|t| t.info.document.deck_tab().is_some()));
            let linked: Vec<String> = linked.cloned().collect();
            w.discard_at_quit.extend(linked.iter().cloned());
            let json = w.view_json();
            drop(w);
            Self::drop_unsent(ws, &linked);
            return json.map(WorkspaceOutcome::LastWindow);
        }
        for tab in &closing {
            w.ensure_closable(tab, discarded.contains(tab))?;
        }
        let win = w.layout.windows.remove(idx);
        w.bump_layout();
        w.forget_window(window);
        let unlinked: Vec<(String, String)> = closing
            .iter()
            .filter_map(|id| {
                let owner = w.tabs.get(id)?.info.document.deck_tab()?.to_owned();
                (!closing.contains(&owner)).then(|| (owner, id.clone()))
            })
            .collect();
        for tab in closing.iter().filter(|id| !win.tabs.contains(id)) {
            // The window is already out: a failure here must not stop half-way.
            if let Err(e) = w.detach(tab) {
                tracing::warn!(tab = %tab, error = %e.one_line(), "linked deck-pack tab not detached");
            }
        }
        let gone: Vec<(String, Tab)> = closing.iter().filter_map(|id| w.remove_tab(id)).collect();
        w.save_layout();
        let (json, trash) = (w.view_json(), w.trash());
        drop(w);
        trash.empty(gone);
        Self::unlink_closed(ws, &unlinked);
        json.map(WorkspaceOutcome::Applied)
    }
}
