//! A pasted element brings its assets and data from the tab it was copied in (t-1178, t-1690).
//!
//! The two sessions are taken one after the other, never together, and never under the
//! workspace lock: the source is read first, then the paste applies in the target.

use std::sync::Mutex;

use oku_slides_core::Op;

use crate::carried_assets::PasteSource;
use crate::error::Result;

use super::{Workspace, WorkspaceOutcome, relock};

impl Workspace {
    /// `PasteElement`: applies `ops` in `to` with what they name from `from`; `to`'s snapshot.
    ///
    /// `from` is read only if ready: never woken nor rendered to read bytes, so a failed
    /// render there never blocks the paste. Closed, unknown or asleep, it brings nothing:
    /// the paste then refuses an asset (`UnknownAsset`) or a data (`op-data-missing`) `to`
    /// lacks. An unknown `to` is an error.
    pub(super) fn paste_element(
        ws: &Mutex<Self>,
        from: &str,
        to: &str,
        label: &str,
        ops: Vec<Op>,
    ) -> Result<WorkspaceOutcome> {
        let source = Self::paste_source(ws, from, &ops);
        Self::paste_into(ws, to, label, ops, source)
    }

    /// What `ops` name in the tab `tab`, if it is ready; nothing otherwise.
    pub(crate) fn paste_source(ws: &Mutex<Self>, tab: &str, ops: &[Op]) -> PasteSource {
        // The workspace lock drops before the session's.
        let Ok(session) = Self::lock(ws).session(tab) else {
            return PasteSource::default();
        };
        // A tab sleeps only on resume: the one the element was just copied in is ready.
        relock(&session).ready_session().map(|s| s.paste_source(ops)).unwrap_or_default()
    }

    /// Applies the paste `ops` in `to` with `source`, as one undo step; `to`'s snapshot.
    pub(crate) fn paste_into(
        ws: &Mutex<Self>,
        to: &str,
        label: &str,
        ops: Vec<Op>,
        source: PasteSource,
    ) -> Result<WorkspaceOutcome> {
        let snapshot = Self::with_session(ws, to, |s| s.paste(label, ops, source)).0.and_then(|r| r)?;
        Ok(WorkspaceOutcome::Applied(snapshot))
    }
}

#[cfg(test)]
#[path = "workspace_paste_tests.rs"]
mod workspace_paste_tests;
