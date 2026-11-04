//! Which window has the system focus, for the voice router (through `AppWorkspace`).
//!
//! Utterances go to the active tab of the window the user looks at: a workspace window, or the
//! front workspace window for any other (the agent window follows it). Once every window lost
//! the focus, the app is in the background: the tab is the front window's, the last one the user
//! focused, and the router only hears the stop word and the wake phrase.

use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_session::Workspace;
use oku_slides_session::agent::{FocusSource, FocusedTab};

/// The window holding the system focus, and the workspace that knows its tabs.
#[derive(Debug)]
pub struct HostFocus {
    /// Label of the focused window; `None` while the app is in the background.
    window: Mutex<Option<String>>,
    workspace: Arc<Mutex<Workspace>>,
}

impl HostFocus {
    /// No window focused yet, over `workspace`.
    pub fn new(workspace: Arc<Mutex<Workspace>>) -> Self {
        Self { window: Mutex::new(None), workspace }
    }

    /// Window `label` got the focus.
    pub fn gained(&self, label: &str) {
        *self.window.lock().unwrap_or_else(PoisonError::into_inner) = Some(label.to_owned());
    }

    /// Window `label` lost the focus (or closed); a later focus elsewhere is kept.
    pub fn lost(&self, label: &str) {
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        if window.as_deref() == Some(label) {
            *window = None;
        }
    }

    /// Label of the focused window, if any.
    fn window(&self) -> Option<String> {
        self.window.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

/// The workspace window whose active tab listens, given the focused one and the front one.
///
/// A workspace window is itself; the agent window, any other one, or none (the background)
/// stands for the front workspace window, the last one focused.
fn listening_window<'a>(focused: Option<&'a str>, front: &'a str) -> &'a str {
    focused.filter(|label| oku_slides_session::is_window_label(label)).unwrap_or(front)
}

impl HostFocus {
    /// The listening tab: the active tab of the focused or front workspace window.
    pub fn focused_tab(&self) -> Option<FocusedTab> {
        // Our lock is let go before the workspace's is taken.
        let focused = self.window();
        let ws = Workspace::lock(&self.workspace);
        let window = listening_window(focused.as_deref(), ws.focused());
        let tab = ws.active_tab(window)?.to_owned();
        Some(FocusedTab { tab, source: FocusSource::Host, focused: focused.is_some() })
    }

    /// The tabs with unsaved changes, counted in the workspace.
    pub fn unsaved_tabs(&self) -> usize {
        Workspace::lock(&self.workspace).unsaved_tabs()
    }
}

#[cfg(test)]
#[path = "host_focus_tests.rs"]
mod host_focus_tests;
