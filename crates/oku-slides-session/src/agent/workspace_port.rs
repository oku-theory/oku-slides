//! What the agent host reads and changes of the workspace: the tab the user listens from, and
//! its window's tabs and windows.
//!
//! The app answers from its windows' focus events (`FocusSource::Host`); the dev server,
//! which has no OS windows, from the focus its pages report (`FocusSource::PageReported`).
//! A host without windows refuses every change ([`WorkspaceRefusal`], `Unavailable`).

use std::path::Path;

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, IntoError, Snafu};

use super::{TabGate, TabOpen};
use crate::import::ImportTarget;
use crate::{DocumentKind, SessionError};

/// Who says which tab has the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusSource {
    /// The host, from its windows' focus events.
    Host,
    /// A page, from `document.hasFocus()`: good for routing, never for a danger confirmation.
    PageReported,
}

/// The tab utterances go to, whether its window has the focus, and who says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusedTab {
    /// The active tab of the focused window, or of the last focused one in the background.
    pub tab: String,
    /// Who says so.
    pub source: FocusSource,
    /// Its window has the focus; `false`: the app is in the background.
    pub focused: bool,
}

/// One tab of a window's bar, as the agent lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabListing {
    /// Its id, `t-<n>`.
    pub tab: String,
    /// Its title as the tab bar shows it, uncleaned: the tool cleans and clips it.
    pub title: String,
    /// It is its window's active tab.
    pub active: bool,
    /// It holds unsaved work.
    pub dirty: bool,
    /// What it edits.
    pub kind: DocumentKind,
}

/// What an agent's close covers: one tab of the requester's window, or that whole window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseScope<'a> {
    /// That tab, which must be in the requester's window (the requester itself included).
    Tab(&'a str),
    /// The requester's window, with every tab in it.
    Window,
}

/// One tab a close covers, as the tab bar shows it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabRevision {
    /// Its id, `t-<n>`.
    pub tab: String,
    /// Its title as the tab bar shows it, uncleaned.
    pub title: String,
    /// It holds unsaved work.
    pub dirty: bool,
    /// Its dirty revision (`TabInfo::dirty_revision`): `None` while clean.
    pub revision: Option<u64>,
}

/// The tabs a close covers, and the window they are in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseTabs {
    /// The requester's window label, `w-<n>`.
    pub window: String,
    /// The covered tabs, in bar order.
    pub tabs: Vec<TabRevision>,
    /// That window is the workspace's only one.
    pub last_window: bool,
}

/// The most search hits an agent's help call returns.
pub const HELP_HITS: usize = 5;

/// One help slide a search found, as the agent reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpHit {
    /// The help slide's id, for `focus` in the help tab.
    pub slide: String,
    /// Its title; `None`: untitled.
    pub title: Option<String>,
}

/// What an agent's help call did: the help tab, and what its search found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpOpen {
    /// The help tab: opened, focused, `Here` (the requester is the help tab) or refused.
    pub open: TabOpen,
    /// The search's first hits, titles first, at most [`HELP_HITS`]; empty without a query.
    pub hits: Vec<HelpHit>,
}

/// Implement this trait to let a tab's agent act on its window's tabs and windows.
///
/// Called from the voice router's thread, the stack and the tools' blocking tasks:
/// implementations take their own locks briefly and never call back into the stack. A gate
/// is asked with no workspace guard held and never blocks. Every change defaults to
/// `Unavailable`: a host without windows (tests, a single document) has none to change.
pub trait AgentWorkspace: Send + Sync + std::fmt::Debug {
    /// The tab utterances go to, focused or left in the background; `None` without any.
    fn focused_tab(&self) -> Option<FocusedTab>;

    /// How many tabs, in every window, hold unsaved work, as the tab bar shows them.
    fn unsaved_tabs(&self) -> usize;

    /// A page of `tab` reported whether it has the focus; only the dev server keeps it.
    fn page_focus(&self, tab: &str, has_focus: bool) {
        let _ = (tab, has_focus);
    }

    /// The tabs of `tab`'s window, in bar order.
    ///
    /// # Errors
    ///
    /// `Unavailable` without windows, `UnknownTab` for a tab this host does not hold, `Closing`.
    fn tabs(&self, tab: &str) -> Result<Vec<TabListing>, WorkspaceRefusal> {
        let _ = tab;
        Err(WorkspaceRefusal::unavailable())
    }

    /// The tabs `scope` covers in `tab`'s window, with their dirty revisions, for a close.
    ///
    /// # Errors
    ///
    /// `Unavailable` without windows, `UnknownTab` (either tab, or another window's), `Closing`.
    fn dirty_revisions(&self, tab: &str, scope: CloseScope<'_>) -> Result<CloseTabs, WorkspaceRefusal> {
        let _ = (tab, scope);
        Err(WorkspaceRefusal::unavailable())
    }

    /// Makes `target`, a tab of `tab`'s window, its active tab once `gate` grants a `Focus`.
    ///
    /// It is shown only if `tab` is its window's active tab then (t-1191); `tab` itself is `Here`.
    ///
    /// # Errors
    ///
    /// `Unavailable`, `UnknownTab` (either tab, or another window's), `Closing`.
    fn activate(
        &self,
        tab: &str,
        target: &str,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        let _ = (tab, target, gate);
        Err(WorkspaceRefusal::unavailable())
    }

    /// Moves `target`, a tab of `tab`'s window, to `index` in its bar; `index` is at most its length.
    ///
    /// # Errors
    ///
    /// `Unavailable`, `UnknownTab`, `IndexOutOfRange`, `Closing`.
    fn reorder(&self, tab: &str, target: &str, index: usize) -> Result<(), WorkspaceRefusal> {
        let _ = (tab, target, index);
        Err(WorkspaceRefusal::unavailable())
    }

    /// Moves `target`, a tab of `tab`'s window, to `window` (`None`: a new one), at `index`.
    ///
    /// `index` is at most the length of the bar it lands in; `None` puts it at the end.
    ///
    /// # Errors
    ///
    /// `Unavailable`, `UnknownTab` (a tab or a window), `IndexOutOfRange`, `Closing`.
    fn move_tab(
        &self,
        tab: &str,
        target: &str,
        window: Option<&str>,
        index: Option<usize>,
    ) -> Result<(), WorkspaceRefusal> {
        let _ = (tab, target, window, index);
        Err(WorkspaceRefusal::unavailable())
    }

    /// Opens a new window holding a blank tab, for `tab`'s agent.
    ///
    /// # Errors
    ///
    /// `Unavailable`, `UnknownTab`, `Closing`.
    fn new_window(&self, tab: &str) -> Result<(), WorkspaceRefusal> {
        let _ = tab;
        Err(WorkspaceRefusal::unavailable())
    }

    /// Detaches the agent into its own window, or attaches it back (`attach`).
    ///
    /// # Errors
    ///
    /// `Unavailable` (the dev server too: its pages have no agent window), `Closing`.
    fn detach_agent(&self, attach: bool) -> Result<(), WorkspaceRefusal> {
        let _ = attach;
        Err(WorkspaceRefusal::unavailable())
    }

    /// Opens the help for `tab`'s agent, or shows its tab, once `gate` grants the move.
    ///
    /// With `query`, the help session is searched and its tab shows the first hit, else its
    /// first slide; without, the slide it shows stays. `tab` itself being the help is `Here`.
    ///
    /// # Errors
    ///
    /// `Unavailable`, `UnknownTab`, `Closing`; a help tab that did not build is the session's error.
    ///
    /// # Blocking
    ///
    /// Builds the help tab's session, or searches the one open.
    fn open_help(
        &self,
        tab: &str,
        query: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<HelpOpen, WorkspaceRefusal> {
        let _ = (tab, query, gate);
        Err(WorkspaceRefusal::unavailable())
    }

    /// Imports `path` to `target` in a new tab of `tab`'s window, once `gate` grants a `NewTab`.
    ///
    /// The new tab never replaces one, even a blank one; it is shown only if `tab` is its
    /// window's active tab once built (t-1191).
    ///
    /// # Errors
    ///
    /// `Unavailable`, `UnknownTab`, `Closing`; a failed import is the session's error.
    ///
    /// # Blocking
    ///
    /// Reads the presentation and builds the tab's session.
    fn import_tab(
        &self,
        tab: &str,
        path: &Path,
        target: ImportTarget,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        let _ = (tab, path, target, gate);
        Err(WorkspaceRefusal::unavailable())
    }
}

/// A workspace with no focused tab, for hosts and tests without windows.
///
/// It refuses every change with `Unavailable`.
#[derive(Debug, Default)]
pub struct NoWorkspace;

impl AgentWorkspace for NoWorkspace {
    fn focused_tab(&self) -> Option<FocusedTab> {
        None
    }

    /// No tab, so none unsaved.
    fn unsaved_tabs(&self) -> usize {
        0
    }
}

/// Why the host refused a workspace change (opaque Snafu with a Fluent key).
#[derive(Debug, Snafu)]
pub struct WorkspaceRefusal(RefusalKind);

/// The refusals of an [`AgentWorkspace`].
#[derive(Debug, Snafu)]
enum RefusalKind {
    #[snafu(display("no windows to change in this host"))]
    Unavailable { backtrace: Backtrace },
    #[snafu(display("no such tab here"))]
    UnknownTab { backtrace: Backtrace },
    #[snafu(display("no such window here"))]
    UnknownWindow { backtrace: Backtrace },
    #[snafu(display("index past the end of the tab bar"))]
    IndexOutOfRange { backtrace: Backtrace },
    #[snafu(display("the workspace is closing"))]
    Closing { backtrace: Backtrace },
    #[snafu(display("the workspace refused the change"))]
    Session { source: SessionError, backtrace: Backtrace },
}

impl WorkspaceRefusal {
    /// This host has no windows to change: tests, the dev server for its agent window.
    #[must_use]
    pub fn unavailable() -> Self {
        Self(UnavailableSnafu.build())
    }

    /// A tab this host does not hold, or that is not in the requester's window.
    #[must_use]
    pub fn unknown_tab() -> Self {
        Self(UnknownTabSnafu.build())
    }

    /// A window label this host does not hold, or a malformed one.
    #[must_use]
    pub fn unknown_window() -> Self {
        Self(UnknownWindowSnafu.build())
    }

    /// An index past the end of the bar it addresses.
    #[must_use]
    pub fn index_out_of_range() -> Self {
        Self(IndexOutOfRangeSnafu.build())
    }

    /// The workspace is closing (the app quits).
    #[must_use]
    pub fn closing() -> Self {
        Self(ClosingSnafu.build())
    }

    /// The session or workspace refused the change: an import that failed, a tab that did not build.
    #[must_use]
    pub fn session(source: SessionError) -> Self {
        Self(SessionSnafu.into_error(source))
    }

    /// The session's own error, for a refusal that wraps one.
    #[must_use]
    pub fn session_error(&self) -> Option<&SessionError> {
        self.wording().err()
    }

    /// The Fluent key of the refusal's message; `None` for a session error, worded by its own.
    #[cfg(test)]
    pub(crate) fn key(&self) -> Option<&'static str> {
        self.wording().ok()
    }

    /// The refusal's Fluent key, or the session error that words itself.
    fn wording(&self) -> Result<&'static str, &SessionError> {
        match &self.0 {
            RefusalKind::Unavailable { .. } => Ok("agent-error-workspace-unavailable"),
            RefusalKind::UnknownTab { .. } => Ok("agent-error-workspace-unknown-tab"),
            RefusalKind::UnknownWindow { .. } => Ok("agent-error-workspace-unknown-window"),
            RefusalKind::IndexOutOfRange { .. } => Ok("agent-error-workspace-index"),
            RefusalKind::Closing { .. } => Ok("agent-error-workspace-closing"),
            RefusalKind::Session { source, .. } => Err(source),
        }
    }
}

impl Localize for WorkspaceRefusal {
    fn localize(&self, l: &Localizer) -> String {
        match self.wording() {
            Ok(key) => l.tr(key),
            Err(e) => e.localize(l),
        }
    }
}

#[cfg(test)]
#[path = "workspace_port_tests.rs"]
mod workspace_port_tests;
