//! A `.pptx` given by the OS: the view asks its window for the import dialog.
//!
//! Launch argument, second instance or macOS (`RunEvent::Opened`): the file
//! does not open as a deck. It joins the queue of [`ImportPick`]s for the
//! focused window, carried by the view: a page not listening yet (or web mode,
//! without events) finds it on its first read. Several files at once queue in
//! order, and the page offers them one after another. The page that opened the
//! dialog acknowledges it (`ImportPicked`): a reload does not reopen it. The
//! user may dismiss every pick waiting for a window (`DismissImportPicks`). A
//! `.pptx` beyond [`MAX_PICKS`] is dropped, and the active tab of the focused
//! window says so in a notice naming the first [`NAMES_SHOWN`] dropped files.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_i18n::Localizer;
use serde::Serialize;

use super::Workspace;
use crate::agent::bounds::label;

/// Most picks waiting at once; a further `.pptx` is dropped, with a notice.
pub(super) const MAX_PICKS: usize = 32;

/// Dropped files the notice names; the others are only counted.
const NAMES_SHOWN: usize = 3;

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

/// The `.pptx` files a full queue dropped during one `open_files` call.
#[derive(Debug, Default)]
pub(super) struct DroppedPicks {
    /// The first [`NAMES_SHOWN`] dropped, in order: the notice names them.
    first: Vec<PathBuf>,
    /// Every path dropped, each counted once.
    seen: HashSet<PathBuf>,
}

impl DroppedPicks {
    /// Whether `path` was dropped already in this call.
    pub(super) fn contains(&self, path: &Path) -> bool {
        self.seen.contains(path)
    }

    /// Records `path` as dropped.
    pub(super) fn push(&mut self, path: PathBuf) {
        if self.first.len() < NAMES_SHOWN {
            self.first.push(path.clone());
        }
        self.seen.insert(path);
    }

    /// How many distinct files were dropped.
    pub(super) fn len(&self) -> usize {
        self.seen.len()
    }

    /// Whether no file was dropped.
    pub(super) fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// A PowerPoint presentation: the `.pptx` extension, in any case.
pub(super) fn is_pptx(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pptx"))
}

impl Workspace {
    /// Queues `path` for the focused window, unless queued already or the queue is full.
    ///
    /// A path queued already keeps its window. Returns `false` when the full queue drops it.
    pub(super) fn pick_import(&mut self, path: PathBuf) -> bool {
        if self.import_picks.iter().any(|p| p.path == path) {
            tracing::info!(path = %path.display(), "import pick already queued");
            return true;
        }
        if self.import_picks.len() >= MAX_PICKS {
            tracing::warn!(path = %path.display(), max = MAX_PICKS, "import pick dropped: queue full");
            return false;
        }
        let id = self.next_pick_id();
        self.import_picks.push(ImportPick { window: self.layout.focused.clone(), path, id });
        true
    }

    /// `ImportPicked`: forgets pick `id`, shown by its page. The others stay.
    pub(super) fn import_picked(&mut self, id: u64) {
        self.import_picks.retain(|p| p.id != id);
    }

    /// `DismissImportPicks`: forgets the picks of `window` up to id `through`.
    pub(super) fn dismiss_import_picks(&mut self, window: &str, through: u64) {
        self.import_picks.retain(|p| p.window != window || p.id > through);
    }

    /// Tells the focused window's active tab which `.pptx` files did not fit the queue.
    ///
    /// Takes the workspace lock, then posts outside it, as [`Self::post_notice`] requires. The
    /// tab shows one notice: a notice posted earlier in the same `open_files` call (an unreadable
    /// file) gives way to this one, as two unreadable files do to each other.
    pub(super) fn notice_dropped_picks(ws: &Mutex<Self>, dropped: &DroppedPicks) {
        if dropped.is_empty() {
            return;
        }
        let target = {
            let w = Self::lock(ws);
            if w.is_closed() {
                return;
            }
            let l = &w.platform.localizer;
            let (names, others) = dropped_list(l, dropped);
            let text = l.tr_with(
                "notice-import-picks-full",
                &[
                    ("count", dropped.len().into()),
                    ("max", MAX_PICKS.into()),
                    ("names", names.into()),
                    ("others", others.into()),
                ],
            );
            let active = w.layout.windows.iter().find(|x| x.label == w.layout.focused).map(|x| x.active.clone());
            active.and_then(|id| Some((w.session(&id).ok()?, id, text)))
        };
        if let Some((session, id, text)) = target {
            Self::post_notice(ws, &id, &session, text);
        } else {
            tracing::warn!(dropped = dropped.len(), "no active tab to show the full import queue notice");
        }
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

/// The first [`NAMES_SHOWN`] file names of `dropped`, cleaned and joined, and how many are left.
///
/// A file name is untrusted: [`label`] cleans and clips it like any request string (t-1383).
pub(super) fn dropped_list(l: &Localizer, dropped: &DroppedPicks) -> (String, usize) {
    let separator = l.tr("notice-import-picks-separator");
    let names: Vec<String> = dropped
        .first
        .iter()
        .map(|p| label(&p.file_name().map_or_else(|| p.to_string_lossy(), |n| n.to_string_lossy())))
        .collect();
    (names.join(&separator), dropped.len().saturating_sub(dropped.first.len()))
}

#[cfg(test)]
#[path = "workspace_import_pick_tests.rs"]
mod workspace_import_pick_tests;
