//! The app's [`AgentWorkspace`]: the focus from [`HostFocus`], tab and window changes synced.
//!
//! Each change goes through the session workspace's `agent_*` functions, then the windows
//! follow (`windows::sync`) from the tool's blocking task, never the main thread. The tool
//! claims its turn's tab move before calling: no gate here asks the hub under a lock.

use std::path::Path;
use std::sync::{Arc, Mutex};

use oku_slides_session::Workspace;
use oku_slides_session::agent::{
    AgentWorkspace, CloseScope, CloseTabs, FocusedTab, HelpOpen, TabGate, TabListing, TabOpen, WorkspaceRefusal,
};
use oku_slides_session::import::ImportTarget;
use tauri::AppHandle;

use crate::host_focus::HostFocus;

/// The workspace port the app gives the agent stack.
pub struct AppWorkspace {
    app: AppHandle,
    workspace: Arc<Mutex<Workspace>>,
    /// Which window has the system focus.
    focus: Arc<HostFocus>,
}

impl AppWorkspace {
    /// The port over `workspace`, its focus read from `focus`, its windows those of `app`.
    pub fn new(app: AppHandle, workspace: Arc<Mutex<Workspace>>, focus: Arc<HostFocus>) -> Self {
        Self { app, workspace, focus }
    }

    /// The windows follow a successful change; `focus` brings the active window forward.
    fn synced<T>(&self, done: Result<T, WorkspaceRefusal>, focus: bool) -> Result<T, WorkspaceRefusal> {
        if done.is_ok() {
            crate::windows::sync(&self.app, None, focus);
        }
        done
    }
}

impl std::fmt::Debug for AppWorkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppWorkspace").finish_non_exhaustive()
    }
}

impl AgentWorkspace for AppWorkspace {
    fn focused_tab(&self) -> Option<FocusedTab> {
        self.focus.focused_tab()
    }

    fn unsaved_tabs(&self) -> usize {
        self.focus.unsaved_tabs()
    }

    fn tabs(&self, tab: &str) -> Result<Vec<TabListing>, WorkspaceRefusal> {
        Workspace::agent_tabs(&self.workspace, tab)
    }

    /// A shown target comes forward in its window.
    fn activate(
        &self,
        tab: &str,
        target: &str,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        let opened = Workspace::agent_activate(&self.workspace, tab, target, gate)?;
        if let TabOpen::Focused { shown: true, .. } = opened {
            crate::windows::sync(&self.app, None, true);
        }
        Ok(opened)
    }

    fn reorder(&self, tab: &str, target: &str, index: usize) -> Result<(), WorkspaceRefusal> {
        self.synced(Workspace::agent_reorder(&self.workspace, tab, target, index), false)
    }

    fn move_tab(
        &self,
        tab: &str,
        target: &str,
        window: Option<&str>,
        index: Option<usize>,
    ) -> Result<(), WorkspaceRefusal> {
        self.synced(Workspace::agent_move(&self.workspace, tab, target, window, index), false)
    }

    fn dirty_revisions(&self, tab: &str, scope: CloseScope<'_>) -> Result<CloseTabs, WorkspaceRefusal> {
        Workspace::agent_dirty_revisions(&self.workspace, tab, scope)
    }

    fn new_window(&self, tab: &str) -> Result<(), WorkspaceRefusal> {
        self.synced(Workspace::agent_new_window(&self.workspace, tab), false)
    }

    /// The agent window opens or closes with the workspace's state (`windows::sync`).
    fn detach_agent(&self, attach: bool) -> Result<(), WorkspaceRefusal> {
        self.synced(Workspace::agent_detach(&self.workspace, attach), false)
    }

    /// The new tab comes forward only when shown.
    fn import_tab(
        &self,
        tab: &str,
        path: &Path,
        target: ImportTarget,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        let opened = Workspace::agent_import(&self.workspace, tab, path, target, gate)?;
        if let TabOpen::Opened { shown, .. } = opened {
            crate::windows::sync(&self.app, None, shown);
        }
        Ok(opened)
    }

    /// The help tab comes forward only when shown.
    fn open_help(
        &self,
        tab: &str,
        query: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<HelpOpen, WorkspaceRefusal> {
        let help = Workspace::agent_help(&self.workspace, tab, query, gate)?;
        if let TabOpen::Opened { shown, .. } | TabOpen::Focused { shown, .. } = help.open {
            crate::windows::sync(&self.app, None, shown);
        }
        Ok(help)
    }
}
