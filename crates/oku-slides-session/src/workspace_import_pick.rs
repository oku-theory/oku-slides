//! A `.pptx` given by the OS: the view asks its window for the import dialog.
//!
//! Launch argument, second instance or macOS (`RunEvent::Opened`): the file
//! does not open as a deck. It joins the queue of [`ImportPick`]s for the
//! focused window, carried by the view: a page not listening yet (or web mode,
//! without events) finds it on its first read. Several files at once queue in
//! order, and the page offers them one after another. The page that opened the
//! dialog acknowledges it (`ImportPicked`): a reload does not reopen it.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::Workspace;

/// Most picks waiting at once; a further `.pptx` is dropped and logged.
pub(super) const MAX_PICKS: usize = 32;

/// A `.pptx` given by the OS, to show in the import dialog.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportPick {
    /// Label of the window that opens the dialog (the focused window).
    pub window: String,
    /// The `.pptx` (normalized on receipt).
    pub path: PathBuf,
    /// Increasing number: the page opens the dialog once per number.
    pub id: u64,
}

/// A PowerPoint presentation: the `.pptx` extension, in any case.
pub(super) fn is_pptx(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pptx"))
}

impl Workspace {
    /// Queues `path` for the focused window, unless queued already or the queue is full.
    ///
    /// A path queued already keeps its window.
    pub(super) fn pick_import(&mut self, path: PathBuf) {
        if self.import_picks.iter().any(|p| p.path == path) {
            tracing::info!(path = %path.display(), "import pick already queued");
            return;
        }
        if self.import_picks.len() >= MAX_PICKS {
            tracing::warn!(path = %path.display(), max = MAX_PICKS, "import pick dropped: queue full");
            return;
        }
        let id = self.next_pick_id();
        self.import_picks.push(ImportPick { window: self.layout.focused.clone(), path, id });
    }

    /// `ImportPicked`: forgets pick `id`, shown by its page. The others stay.
    pub(super) fn import_picked(&mut self, id: u64) {
        self.import_picks.retain(|p| p.id != id);
    }

    /// Gives the picks of `from` to `to` with new ids, so its page offers them as new.
    ///
    /// They move to the end of the queue, which stays ordered by id.
    pub(super) fn move_picks(&mut self, from: &str, to: &str) {
        let (moved, kept) = std::mem::take(&mut self.import_picks).into_iter().partition(|p| p.window == from);
        self.import_picks = kept;
        for mut pick in moved {
            to.clone_into(&mut pick.window);
            pick.id = self.next_pick_id();
            self.import_picks.push(pick);
        }
    }

    fn next_pick_id(&mut self) -> u64 {
        self.last_pick = self.last_pick.saturating_add(1);
        self.last_pick
    }
}

#[cfg(test)]
#[path = "workspace_import_pick_tests.rs"]
mod workspace_import_pick_tests;
