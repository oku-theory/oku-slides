//! The workspace as the agent sees it: a tab's session ([`Sessions`]),
//! a file opened beside it, a pack opened from it, and the dev server's workspace port
//! ([`AgentWorkspace`]: the focus its pages report, its window's tabs, imports, help).

use std::path::Path;
use std::sync::Mutex;

use oku_slides_core::UsageRecord;
use oku_slides_platform::storage::resolve;
use snafu::{OptionExt, ensure};
use uuid::Uuid;

use crate::agent::{
    AgentWorkspace, CloseScope, CloseTabs, FocusSource, FocusedTab, HelpOpen, Sessions, TabGate, TabListing, TabOpen,
    WorkspaceRefusal,
};
use crate::error::{Result, UnknownTabSnafu, UnknownWindowSnafu};
use crate::import::ImportTarget;
use crate::{DocumentKind, Session};

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

    /// The kind of `tab`'s document, read from its cached info: a dormant tab stays asleep.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, or `UnknownWindow` once the workspace is closed.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock alone.
    pub fn document_kind(ws: &Mutex<Self>, tab: &str) -> Result<DocumentKind> {
        let w = Self::lock(ws);
        ensure!(!w.is_closed(), UnknownWindowSnafu { label: tab });
        Ok(w.tabs.get(tab).context(UnknownTabSnafu { id: tab })?.info.document.kind())
    }
}

/// The agent reaches tab sessions through the workspace, deck-pack forwards included.
/// The redraw flag and the touched owners stay with the host, which calls
/// [`Workspace::with_session`] and drains [`Workspace::drain_touched`] itself. A tab
/// set aside during a call keeps its agent here, a limit of the CLI's `serve`, which
/// uses this impl; the app's host wraps the call in [`Workspace::closed_by`] (t-1239).
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
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen> {
        Workspace::open_pack(self, tab, pack, widget, gate)
    }

    fn open_theme(&self, tab: &str, theme: Uuid, gate: &mut dyn FnMut(TabGate) -> bool) -> Result<TabOpen> {
        Workspace::open_theme_for_agent(self, tab, theme, gate)
    }

    fn document_kind(&self, tab: &str) -> Result<DocumentKind> {
        Workspace::document_kind(self, tab)
    }

    fn usage_owner(&self, tab: &str) -> Option<String> {
        Workspace::usage_owner(self, tab)
    }

    fn record_usage(&self, owner: &str, record: UsageRecord) -> Result<()> {
        Workspace::record_usage(self, owner, record)
    }
}

/// The dev server has no OS windows: the focus is the one its pages report.
///
/// The tab of the last page that reported having the focus, while it is open; a page of it that
/// reports losing the focus clears it. Good for routing only: [`FocusSource::PageReported`].
/// Its pages follow the workspace's changes; they have no agent window to detach.
impl AgentWorkspace for Mutex<Workspace> {
    fn focused_tab(&self) -> Option<FocusedTab> {
        let w = Workspace::lock(self);
        let (tab, focused) = w.page_focus.as_ref().filter(|(t, _)| w.tabs.contains_key(t.as_str()))?;
        Some(FocusedTab { tab: tab.clone(), source: FocusSource::PageReported, focused: *focused })
    }

    fn unsaved_tabs(&self) -> usize {
        Workspace::lock(self).unsaved_tabs()
    }

    /// A focused page names its tab; the page losing it keeps its tab, unfocused.
    fn page_focus(&self, tab: &str, has_focus: bool) {
        let mut w = Workspace::lock(self);
        let kept = w.page_focus.as_ref().is_some_and(|(t, _)| t == tab);
        if has_focus || kept {
            w.page_focus = Some((tab.to_owned(), has_focus));
        }
    }

    fn tabs(&self, tab: &str) -> std::result::Result<Vec<TabListing>, WorkspaceRefusal> {
        Workspace::agent_tabs(self, tab)
    }

    fn dirty_revisions(&self, tab: &str, scope: CloseScope<'_>) -> std::result::Result<CloseTabs, WorkspaceRefusal> {
        Workspace::agent_dirty_revisions(self, tab, scope)
    }

    fn activate(
        &self,
        tab: &str,
        target: &str,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> std::result::Result<TabOpen, WorkspaceRefusal> {
        Workspace::agent_activate(self, tab, target, gate)
    }

    fn reorder(&self, tab: &str, target: &str, index: usize) -> std::result::Result<(), WorkspaceRefusal> {
        Workspace::agent_reorder(self, tab, target, index)
    }

    fn move_tab(
        &self,
        tab: &str,
        target: &str,
        window: Option<&str>,
        index: Option<usize>,
    ) -> std::result::Result<(), WorkspaceRefusal> {
        Workspace::agent_move(self, tab, target, window, index)
    }

    fn new_window(&self, tab: &str) -> std::result::Result<(), WorkspaceRefusal> {
        Workspace::agent_new_window(self, tab)
    }

    fn import_tab(
        &self,
        tab: &str,
        path: &Path,
        target: ImportTarget,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> std::result::Result<TabOpen, WorkspaceRefusal> {
        Workspace::agent_import(self, tab, path, target, gate)
    }

    fn open_help(
        &self,
        tab: &str,
        query: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> std::result::Result<HelpOpen, WorkspaceRefusal> {
        Workspace::agent_help(self, tab, query, gate)
    }
}

#[cfg(test)]
#[path = "workspace_agent_tests.rs"]
mod workspace_agent_tests;
