//! The agent and the tabs: a tab's session, provided by the host.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_core::{UsageLedger, UsageRecord};
use uuid::Uuid;

use crate::pack_tab::PackRoute;
use crate::{DocumentKind, Session, TabDocument};

/// What opening a pack or theme tab asks an agent's gate before acting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabGate {
    /// Focusing, or selecting an item in, another tab that edits the pack or theme.
    Focus,
    /// Opening a new pack or theme tab.
    NewTab,
}

/// Where an agent's open of a pack or theme tab landed.
///
/// `shown` is true when the tab became its window's active tab: only when
/// the requesting tab was its window's active tab at that moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabOpen {
    /// The requesting tab edits it (its item selected, if one was named).
    Here,
    /// A new tab opened for it.
    Opened {
        /// The new tab.
        tab: String,
        /// Whether it became its window's active tab.
        shown: bool,
    },
    /// Another tab already edited it and was focused.
    Focused {
        /// That tab.
        tab: String,
        /// Whether it became its window's active tab.
        shown: bool,
    },
    /// The gate refused; nothing changed.
    Refused,
}

/// The tabs' sessions, as the agent sees them.
///
/// Implement this trait to give the agent's tools and turns a tab's session
/// (the host's workspace, or a single session in tests).
pub trait Sessions: Send + Sync + std::fmt::Debug {
    /// Calls `f` on the session of `tab`; blocking.
    ///
    /// Unknown tab: an error, and `f` is not called.
    ///
    /// # Errors
    ///
    /// Fails if tab `tab` is unknown to the host.
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()>;

    /// True if `path` is already open in a tab other than `tab`.
    ///
    /// Defaults to `false`: a single-document host has no other tab.
    fn open_elsewhere(&self, _tab: &str, _path: &Path) -> bool {
        false
    }

    /// True if `tab` is its window's shown tab: only it applies the agent's
    /// events (present, interface, history).
    ///
    /// Defaults to `true`: a single-document host always shows its own. Any
    /// multi-tab host overrides it (the app, `serve`), or a hidden tab would receive them.
    fn shown(&self, _tab: &str) -> bool {
        true
    }

    /// Opens `path` in a new tab next to `tab`, returning that tab; blocking.
    ///
    /// If the file is already open, its tab becomes active. Tab `tab` and its document stay as they
    /// are.
    ///
    /// Defaults to `SingleDocument`: a single-document host has no other tab.
    ///
    /// # Errors
    ///
    /// Fails if the host has only one document (`SingleDocument`, the default) or if the tab does not
    /// open.
    fn open_tab(&self, _tab: &str, _path: &Path) -> crate::Result<String> {
        Err(crate::error::SingleDocumentSnafu.build().into())
    }

    /// Opens `pack` from `tab` for an agent; says where it landed.
    ///
    /// `gate` is asked at most one `Focus` (another tab edits the pack), then
    /// at most one `NewTab`, then possibly one `Focus` again when the new
    /// tab's lease is refused by a live holder (see [`TabGate`]). It is asked
    /// with only the pack's opening lock held and must never block (no user
    /// prompt, no wait). The tab is shown (made active) only if `tab` is its
    /// window's active tab (see [`TabOpen`]).
    ///
    /// # Errors
    ///
    /// `UnknownTab`, the `route_pack` refusals, `PackOpenInTab`, a session build failure.
    ///
    /// # Blocking
    ///
    /// Wakes, builds and renders sessions.
    fn open_pack(
        &self,
        tab: &str,
        pack: Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> crate::Result<TabOpen>;

    /// Opens `theme` from `tab` for an agent; says where it landed.
    ///
    /// The gate order and the `shown` rule are those of [`Self::open_pack`]:
    /// at most one `Focus`, then at most one `NewTab`, then possibly one
    /// `Focus` after a lease refusal; never asked when `tab` edits `theme`.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `UnknownTheme`, `ThemeOpenInTab`, `SingleDocument` for a
    /// host without tabs, a session build failure.
    ///
    /// # Blocking
    ///
    /// Wakes, builds and renders sessions.
    fn open_theme(&self, tab: &str, theme: Uuid, gate: &mut dyn FnMut(TabGate) -> bool) -> crate::Result<TabOpen>;

    /// The kind of `tab`'s document; wakes no dormant tab.
    ///
    /// # Errors
    ///
    /// Fails if tab `tab` is unknown to the host or the host is closing.
    ///
    /// # Blocking
    ///
    /// Takes the host's lock.
    fn document_kind(&self, tab: &str) -> crate::Result<DocumentKind>;

    /// The deck tab whose ledger takes `tab`'s agent usage; `None`: not counted.
    ///
    /// Required: a multi-tab host routes a pack or theme tab's turns to its
    /// deck; a single-document host answers `tab` itself.
    ///
    /// # Blocking
    ///
    /// Takes the host's lock; wakes no dormant tab.
    fn usage_owner(&self, tab: &str) -> Option<String>;
}

/// A single session, whatever the tab (tests, single-document hosts).
impl Sessions for Mutex<Session> {
    fn with(&self, _tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()> {
        f(&mut self.lock().unwrap_or_else(PoisonError::into_inner));
        Ok(())
    }

    /// A host without tabs opens only the pack its session edits.
    ///
    /// No gate is asked.
    ///
    /// Any other pack would need a tab of its own: `SingleDocument`.
    fn open_pack(
        &self,
        _tab: &str,
        pack: Uuid,
        widget: Option<&str>,
        _gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> crate::Result<TabOpen> {
        match self.lock().unwrap_or_else(PoisonError::into_inner).route_pack(pack, widget)? {
            PackRoute::Here => Ok(TabOpen::Here),
            PackRoute::Tab { .. } | PackRoute::DeckTab { .. } => Err(crate::error::SingleDocumentSnafu.build().into()),
        }
    }

    /// A host without tabs opens only the theme its session edits.
    ///
    /// No gate is asked. Any other theme would need a tab of its own: `SingleDocument`.
    fn open_theme(&self, _tab: &str, theme: Uuid, _gate: &mut dyn FnMut(TabGate) -> bool) -> crate::Result<TabOpen> {
        let document = self.lock().unwrap_or_else(PoisonError::into_inner).tab_info().document;
        if document == (TabDocument::Theme { theme }) {
            Ok(TabOpen::Here)
        } else {
            Err(crate::error::SingleDocumentSnafu.build().into())
        }
    }

    fn document_kind(&self, _tab: &str) -> crate::Result<DocumentKind> {
        Ok(self.lock().unwrap_or_else(PoisonError::into_inner).document_kind())
    }

    /// The session itself: `Session::record_usage` refuses outside a deck.
    fn usage_owner(&self, tab: &str) -> Option<String> {
        Some(tab.to_owned())
    }
}

/// A tab and the access to its session.
#[derive(Clone, Debug)]
pub struct TabSession {
    tab: String,
    sessions: Arc<dyn Sessions>,
}

impl TabSession {
    /// A tab `tab`, whose sessions `sessions` provides.
    pub fn new(tab: impl Into<String>, sessions: Arc<dyn Sessions>) -> Self {
        Self { tab: tab.into(), sessions }
    }

    /// The tab's id.
    #[must_use]
    pub fn tab(&self) -> &str {
        &self.tab
    }

    /// True if `path` is already open in another tab; blocking (lock).
    #[must_use]
    pub fn open_elsewhere(&self, path: &Path) -> bool {
        self.sessions.open_elsewhere(&self.tab, path)
    }

    /// Whether the tab is shown (see [`Sessions::shown`]); blocking (lock).
    #[must_use]
    pub fn shown(&self) -> bool {
        self.sessions.shown(&self.tab)
    }

    /// Opens `path` next to this tab (see [`Sessions::open_tab`]); blocking.
    ///
    /// # Errors
    ///
    /// Fails like [`Sessions::open_tab`]: single-document host, or a tab or file that does not open.
    pub fn open_tab(&self, path: &Path) -> crate::Result<String> {
        self.sessions.open_tab(&self.tab, path)
    }

    /// Opens `pack` from this tab for an agent (see [`Sessions::open_pack`]).
    ///
    /// # Errors
    ///
    /// As [`Sessions::open_pack`].
    ///
    /// # Blocking
    ///
    /// As [`Sessions::open_pack`].
    pub fn open_pack(
        &self,
        pack: Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> crate::Result<TabOpen> {
        self.sessions.open_pack(&self.tab, pack, widget, gate)
    }

    /// Opens `theme` from this tab for an agent (see [`Sessions::open_theme`]).
    ///
    /// # Errors
    ///
    /// As [`Sessions::open_theme`].
    ///
    /// # Blocking
    ///
    /// As [`Sessions::open_theme`].
    pub fn open_theme(&self, theme: Uuid, gate: &mut dyn FnMut(TabGate) -> bool) -> crate::Result<TabOpen> {
        self.sessions.open_theme(&self.tab, theme, gate)
    }

    /// The kind of this tab's document (see [`Sessions::document_kind`]); blocking.
    ///
    /// # Errors
    ///
    /// As [`Sessions::document_kind`].
    pub fn document_kind(&self) -> crate::Result<DocumentKind> {
        self.sessions.document_kind(&self.tab)
    }

    /// Records a turn's usage in the deck that counts this tab's turns; blocking.
    ///
    /// Never fails the turn: a tab no deck counts records nothing, and an
    /// owner tab that is gone is logged.
    pub fn record_usage(&self, record: UsageRecord) {
        let Some(owner) = self.sessions.usage_owner(&self.tab) else { return };
        let mut record = Some(record);
        let done = self.sessions.with(&owner, &mut |s| {
            // An owner that is no deck (a host routing badly) loses the turn: logged.
            if let Some(r) = record.take()
                && !s.record_usage(r)
                && s.usage_ledger().is_none()
            {
                tracing::warn!(owner = %owner, "agent usage owner holds no deck: turn not recorded");
            }
        });
        if let Err(e) = done {
            tracing::warn!(tab = %self.tab, owner = %owner, error = %e.one_line(), "agent usage not recorded");
        }
    }

    /// A copy of the usage ledger of the deck counting this tab's turns.
    ///
    /// Blocking.
    ///
    /// `None`: no deck counts them (a help tab, a pack or theme tab without opener).
    ///
    /// # Errors
    ///
    /// Fails if the owner tab is unknown to the host.
    pub fn usage_ledger(&self) -> crate::Result<Option<UsageLedger>> {
        let Some(owner) = self.sessions.usage_owner(&self.tab) else { return Ok(None) };
        let mut out = None;
        self.sessions.with(&owner, &mut |s| out = s.usage_ledger().cloned())?;
        Ok(out)
    }

    /// Runs `f` on the tab's session; blocking (lock), so off the async thread.
    ///
    /// # Errors
    ///
    /// Fails if the tab is unknown to the host, or if the host did not call `f`.
    pub fn with<R>(&self, f: impl FnOnce(&mut Session) -> R) -> crate::Result<R> {
        let mut f = Some(f);
        let mut out = None;
        self.sessions.with(&self.tab, &mut |s| {
            if let Some(f) = f.take() {
                out = Some(f(s));
            }
        })?;
        // `Sessions::with` calls `f` at most once; `Ok` without a call is
        // an implementation bug.
        out.ok_or_else(|| {
            crate::error::AgentFormSnafu { reason: super::AgentFormReason::SessionNotCalled { tab: self.tab.clone() } }
                .build()
                .into()
        })
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
