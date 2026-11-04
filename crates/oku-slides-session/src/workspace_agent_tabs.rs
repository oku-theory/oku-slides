//! A tab's agent acting on its window's tabs and windows ([`AgentWorkspace`] hosts call these).
//!
//! Each action is scoped to the requester's window: a target in another window is refused as
//! unknown. Gates are asked with no workspace guard held; the move a gate grants is decided
//! again under the guard that makes it (t-1191: shown only while the requester is).
//!
//! [`AgentWorkspace`]: crate::agent::AgentWorkspace

use std::path::Path;
use std::sync::Mutex;

use oku_slides_platform::storage::resolve;

use crate::agent::{CloseScope, CloseTabs, TabGate, TabListing, TabOpen, TabRevision, WorkspaceRefusal};
use crate::import::ImportTarget;
use crate::{SessionError, SessionErrorKind};

use super::tabs::{Activate, ImportOpening, Opening};
use super::{TabPlace, Workspace, WorkspaceRequest};

impl Workspace {
    /// The tabs of `tab`'s window, in bar order, titles uncleaned.
    ///
    /// # Errors
    ///
    /// `UnknownTab` for a tab this workspace does not hold, `Closing` once settled.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock alone; wakes no dormant tab.
    pub fn agent_tabs(ws: &Mutex<Self>, tab: &str) -> Result<Vec<TabListing>, WorkspaceRefusal> {
        let w = Self::lock(ws);
        let label = w.agent_window(tab)?.to_owned();
        let view = w.view();
        let window = view.windows.into_iter().find(|v| v.label == label).ok_or_else(WorkspaceRefusal::unknown_tab)?;
        let active = window.active;
        Ok(window
            .tabs
            .into_iter()
            .map(|t| TabListing {
                active: t.id == active,
                tab: t.id,
                title: t.info.title,
                dirty: t.info.dirty,
                kind: t.info.document.kind(),
            })
            .collect())
    }

    /// The tabs `scope` covers in `tab`'s window, with their dirty revisions, for an agent's close.
    ///
    /// # Errors
    ///
    /// `UnknownTab` (either tab, or a target of another window), `Closing` once settled.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock alone; wakes no dormant tab.
    pub fn agent_dirty_revisions(
        ws: &Mutex<Self>,
        tab: &str,
        scope: CloseScope<'_>,
    ) -> Result<CloseTabs, WorkspaceRefusal> {
        let w = Self::lock(ws);
        let label = match scope {
            CloseScope::Tab(target) => w.agent_sibling(tab, target)?,
            CloseScope::Window => w.agent_window(tab)?.to_owned(),
        };
        let view = w.view();
        let last_window = view.windows.len() == 1;
        let window = view.windows.into_iter().find(|v| v.label == label).ok_or_else(WorkspaceRefusal::unknown_tab)?;
        let tabs = window
            .tabs
            .into_iter()
            .filter(|t| match scope {
                CloseScope::Tab(target) => t.id == target,
                CloseScope::Window => true,
            })
            .map(|t| TabRevision {
                tab: t.id,
                title: t.info.title,
                dirty: t.info.dirty,
                revision: t.info.dirty_revision,
            })
            .collect();
        Ok(CloseTabs { window: label, tabs, last_window })
    }

    /// Makes `target` its window's active tab for `tab`'s agent, once `gate` grants a `Focus`.
    ///
    /// `tab` itself is `Here` (no gate asked). The target is activated only if `tab` is its
    /// window's active tab under the guard that activates it; else `Focused` unshown, unchanged.
    ///
    /// # Errors
    ///
    /// `UnknownTab` (either tab, or a target of another window), `Closing`.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock twice, never across the gate.
    pub fn agent_activate(
        ws: &Mutex<Self>,
        tab: &str,
        target: &str,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        Self::lock(ws).agent_sibling(tab, target)?;
        if target == tab {
            return Ok(TabOpen::Here);
        }
        if !gate(TabGate::Focus) {
            return Ok(TabOpen::Refused);
        }
        let mut w = Self::lock(ws);
        w.agent_sibling(tab, target)?;
        let shown = w.tab_shown(tab);
        if shown {
            w.activate(target).map_err(|e| w.agent_refusal(e))?;
            w.save_layout();
        }
        Ok(TabOpen::Focused { tab: target.to_owned(), shown })
    }

    /// Moves `target`, a tab of `tab`'s window, to `index` in its bar (at most the bar's length).
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `IndexOutOfRange`, `Closing`.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock twice, through [`Workspace::apply`] for the change.
    pub fn agent_reorder(ws: &Mutex<Self>, tab: &str, target: &str, index: usize) -> Result<(), WorkspaceRefusal> {
        {
            let w = Self::lock(ws);
            let label = w.agent_sibling(tab, target)?;
            w.agent_index(&label, index)?;
        }
        Self::agent_apply(ws, WorkspaceRequest::Reorder { tab: target.to_owned(), index })
    }

    /// Moves `target`, a tab of `tab`'s window, to `window` (`None`: a new one) at `index`.
    ///
    /// `index` is at most the length of the bar it lands in; `None` puts it at the end.
    ///
    /// # Errors
    ///
    /// `UnknownTab` (a tab or the window), `IndexOutOfRange`, `Closing`.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock twice, through [`Workspace::apply`] for the change.
    pub fn agent_move(
        ws: &Mutex<Self>,
        tab: &str,
        target: &str,
        window: Option<&str>,
        index: Option<usize>,
    ) -> Result<(), WorkspaceRefusal> {
        {
            let w = Self::lock(ws);
            w.agent_sibling(tab, target)?;
            if let Some(label) = window {
                w.window_index(label).map_err(|_| WorkspaceRefusal::unknown_window())?;
            }
            if let Some(index) = index {
                // A new window's bar is empty: only 0 fits.
                match window {
                    Some(label) => w.agent_index(label, index)?,
                    None if index == 0 => {}
                    None => return Err(WorkspaceRefusal::index_out_of_range()),
                }
            }
        }
        let req = WorkspaceRequest::MoveTab { tab: target.to_owned(), window: window.map(str::to_owned), index };
        Self::agent_apply(ws, req)
    }

    /// Opens a new window holding a blank tab, for `tab`'s agent.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `Closing`, or the blank tab's build failure.
    ///
    /// # Blocking
    ///
    /// Builds the blank tab's session outside the lock.
    pub fn agent_new_window(ws: &Mutex<Self>, tab: &str) -> Result<(), WorkspaceRefusal> {
        Self::lock(ws).agent_window(tab)?;
        Self::agent_apply(ws, WorkspaceRequest::NewWindow)
    }

    /// Detaches the agent into its own window, or attaches it back (`attach`).
    ///
    /// # Errors
    ///
    /// `Closing`.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock.
    pub fn agent_detach(ws: &Mutex<Self>, attach: bool) -> Result<(), WorkspaceRefusal> {
        let req = if attach { WorkspaceRequest::AttachAgent } else { WorkspaceRequest::DetachAgent };
        Self::agent_apply(ws, req)
    }

    /// Imports `path` to `target` in a new tab of `tab`'s window, once `gate` grants a `NewTab`.
    ///
    /// The gate is asked before any lock. The new tab goes at the end of the bar, never
    /// replacing one, even a blank one: `tab`'s agent converses there. It is shown only if
    /// `tab` is its window's active tab once built. The host's fine-tuning is not asked here.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `Closing` (also when settled during the build); the import's own error.
    ///
    /// # Blocking
    ///
    /// Reads the presentation and builds the tab's session, outside the lock.
    pub fn agent_import(
        ws: &Mutex<Self>,
        tab: &str,
        path: &Path,
        target: ImportTarget,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        if !gate(TabGate::NewTab) {
            return Ok(TabOpen::Refused);
        }
        let pending = {
            let mut w = Self::lock(ws);
            let window = w.agent_window(tab)?.to_owned();
            let opening = Opening::Import(Box::new(ImportOpening { path: resolve(path), target, tune: false }));
            let mut pending = w.pending(Some(window), opening, TabPlace::End);
            pending.activate = Activate::IfShown(tab.to_owned());
            pending
        };
        let session = pending.build().map_err(WorkspaceRefusal::session)?;
        let (_, id, shown) = Self::finish_shown(ws, pending, session).map_err(|e| Self::lock(ws).agent_refusal(e))?;
        Ok(TabOpen::Opened { tab: id, shown })
    }

    /// Applies an agent's checked change; its errors as refusals.
    fn agent_apply(ws: &Mutex<Self>, req: WorkspaceRequest) -> Result<(), WorkspaceRefusal> {
        if Self::lock(ws).is_closed() {
            return Err(WorkspaceRefusal::closing());
        }
        Self::apply(ws, req).map(drop).map_err(|e| Self::lock(ws).agent_refusal(e))
    }

    /// A workspace error as the agent's refusal: unknown tabs and windows, closing once
    /// settled, else the session's own.
    pub(super) fn agent_refusal(&self, e: SessionError) -> WorkspaceRefusal {
        match e.kind() {
            _ if self.is_closed() => WorkspaceRefusal::closing(),
            SessionErrorKind::UnknownTab => WorkspaceRefusal::unknown_tab(),
            SessionErrorKind::UnknownWindow => WorkspaceRefusal::unknown_window(),
            _ => WorkspaceRefusal::session(e),
        }
    }

    /// The window of the requester `tab`, under the guard.
    pub(super) fn agent_window(&self, tab: &str) -> Result<&str, WorkspaceRefusal> {
        if self.is_closed() {
            return Err(WorkspaceRefusal::closing());
        }
        self.tab_window(tab).ok_or_else(WorkspaceRefusal::unknown_tab)
    }

    /// The window `tab` and `target` share; `UnknownTab` when they do not.
    fn agent_sibling(&self, tab: &str, target: &str) -> Result<String, WorkspaceRefusal> {
        let label = self.agent_window(tab)?;
        if self.tab_window(target) != Some(label) {
            return Err(WorkspaceRefusal::unknown_tab());
        }
        Ok(label.to_owned())
    }

    /// `index` addresses window `label`'s bar: at most its length.
    fn agent_index(&self, label: &str, index: usize) -> Result<(), WorkspaceRefusal> {
        let at = self.window_index(label).map_err(|_| WorkspaceRefusal::unknown_window())?;
        if index > self.layout.windows[at].tabs.len() {
            return Err(WorkspaceRefusal::index_out_of_range());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "workspace_agent_tabs_tests.rs"]
mod workspace_agent_tabs_tests;
