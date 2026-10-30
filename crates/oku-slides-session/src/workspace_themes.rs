//! Theme tabs in the workspace: opening a library theme in its own tab.
//!
//! An open of theme T runs under T's opening lock (`with_opening`, shared
//! with packs): a second open of T waits, then finds the first one's tab.
//! Under that lock, the workspace and each session are taken in sequence,
//! never nested: a theme tab of T already open is focused, else the
//! requesting tab (any kind, a help tab included) checks the theme exists
//! (`route_theme`), T's lease is taken, and a new theme tab is built outside
//! the workspace lock, right after the requester. The requester becomes the
//! tab's opener, which a close hands the focus back to. Unlike a pack, a
//! theme tab has no preview seed; an agent's open asks the same gate
//! (`TabGate`), never under the workspace guard, and shows the tab only
//! when the requester is its window's active tab. A new theme (`new_theme`)
//! is created empty through the requesting tab's library, then opened; its
//! draft goes if that open fails.

use std::sync::Mutex;

use uuid::Uuid;

use crate::TabDocument;
use crate::agent::{TabGate, TabOpen};
use crate::error::SingleDocumentSnafu;
use crate::library_packs::DraftKind;
use crate::pack_lease::PackLease;

use super::opening::with_opening;
use super::packs::{Ask, Opener, PackOpened};
#[cfg(test)]
use super::tabs::open_failpoint;
use super::{Activate, Created, Opening, Result, TabPlace, Workspace, WorkspaceOutcome};

impl Workspace {
    /// `apply` on `OpenTheme`: the view once theme `theme` has a tab, opened or focused.
    ///
    /// # Errors
    ///
    /// `UnknownTab` for `tab`, `UnknownTheme` for a theme the library lacks,
    /// `ThemeOpenInTab` while a tab no longer found holds the theme, and the
    /// errors of building the tab.
    ///
    /// # Blocking
    ///
    /// Holds the theme's opening lock, reads the library, and builds and renders a session.
    pub(super) fn open_theme_ui(ws: &Mutex<Self>, tab: &str, theme: Uuid) -> Result<WorkspaceOutcome> {
        with_opening(theme, || Self::open_theme(ws, tab, theme, Created::No, Opener::Ui))?;
        Ok(WorkspaceOutcome::Applied(Self::lock(ws).view_json()?))
    }

    /// Opens `theme` from `tab` for an agent; says where it landed.
    ///
    /// # Errors
    ///
    /// As [`crate::agent::Sessions::open_theme`].
    ///
    /// # Blocking
    ///
    /// As [`crate::agent::Sessions::open_theme`].
    pub fn open_theme_for_agent(
        ws: &Mutex<Self>,
        tab: &str,
        theme: Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen> {
        with_opening(theme, || Self::open_theme(ws, tab, theme, Created::No, Opener::Agent(gate))).map(Into::into)
    }

    /// `apply` on `NewTheme`: an empty library theme, opened in its own tab after `tab`.
    ///
    /// Any tab with a theme lease scope may ask (a help tab included); it
    /// creates the theme through its library, in its language. An open that
    /// fails deletes the draft again.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `SingleDocument` without a theme lease scope, the
    /// creation's and the open's errors.
    ///
    /// # Blocking
    ///
    /// Writes the theme library, then builds and renders the new tab's session.
    pub(super) fn new_theme(ws: &Mutex<Self>, tab: &str) -> Result<WorkspaceOutcome> {
        let (created, _changed) = Self::with_quiet(ws, tab, |s| {
            snafu::ensure!(s.library.theme_scope().is_some(), SingleDocumentSnafu);
            let name = s.localizer.tr("theme-default-name");
            Ok::<_, crate::SessionError>((s.library.create_empty_theme(&name)?, s.library.clone()))
        });
        let (id, library) = created??;
        let opened = with_opening(id, || Self::open_theme(ws, tab, id, Created::Yes, Opener::Ui));
        if opened.is_err() {
            Self::roll_back(&library, DraftKind::Theme, id);
        }
        opened?;
        Ok(WorkspaceOutcome::Applied(Self::lock(ws).view_json()?))
    }

    /// Focuses the theme tab of `theme`, or opens one after `tab`; holds the theme's opening lock.
    ///
    /// An agent's gate is asked `Focus` once before focusing another tab,
    /// `NewTab` before building one, and `Focus` again before focusing the
    /// live holder of a refused lease; never with the workspace guard held.
    fn open_theme(
        ws: &Mutex<Self>,
        tab: &str,
        theme: Uuid,
        created: Created,
        opener: Opener<'_>,
    ) -> Result<PackOpened> {
        #[cfg(test)]
        open_failpoint()?;
        let mut gate = match opener {
            Opener::Ui => None,
            Opener::Agent(gate) => Some(gate),
        };
        let ui = gate.is_none();
        let ask: &mut Ask<'_> = &mut |g: TabGate| gate.as_mut().is_none_or(|gate| gate(g));
        let wanted = TabDocument::Theme { theme };
        let (window, candidates) = Self::lock(ws).candidates(tab, std::slice::from_ref(&wanted))?;
        let mut asked = false;
        for candidate in &candidates {
            if candidate != tab && !asked {
                if !ask(TabGate::Focus) {
                    return Ok(PackOpened::Refused);
                }
                asked = true;
            }
            if !Self::theme_tab_edits(ws, candidate, &wanted) {
                continue;
            }
            if let Some(shown) = Self::activate_theme_tab(ws, tab, candidate, ui)? {
                return Ok(if candidate == tab {
                    PackOpened::Here
                } else {
                    PackOpened::Focused { tab: candidate.clone(), shown }
                });
            }
        }
        let (route, _changed) = Self::with_quiet(ws, tab, |s| s.route_theme(theme));
        let (scope, name) = route??;
        if !ask(TabGate::NewTab) {
            return Ok(PackOpened::Refused);
        }
        let lease = match scope.map(|scope| PackLease::acquire(&scope, theme, &name, &name)).transpose() {
            Ok(lease) => lease,
            Err(refused) => {
                // Read in its own statement: the gate is never asked under the workspace guard.
                let live = Self::lock(ws).live_tab(&wanted);
                let Some(holder) = live else { return Err(refused) };
                if !ask(TabGate::Focus) {
                    return Ok(PackOpened::Refused);
                }
                if !Self::theme_tab_edits(ws, &holder, &wanted) {
                    return Err(refused);
                }
                return match Self::activate_theme_tab(ws, tab, &holder, ui)? {
                    Some(shown) => Ok(PackOpened::Focused { tab: holder, shown }),
                    None => Err(refused),
                };
            }
        };
        let pending = {
            let mut w = Self::lock(ws);
            // The requesting tab closed meanwhile: its window, at the end and without opener.
            let place = if w.tab_window(tab).is_some() { TabPlace::After(tab.to_owned()) } else { TabPlace::End };
            let window = w.tab_window(tab).map_or(window, str::to_owned);
            let mut pending = w.pending(Some(window), Opening::Theme { theme, created }, place);
            if !ui {
                pending.activate = Activate::IfShown(tab.to_owned());
            }
            pending
        };
        let session = pending.build_theme(lease)?;
        Self::finish_opened(ws, pending, session)
    }

    /// Whether `candidate`'s live session edits `wanted`, waking it; false: skipped.
    fn theme_tab_edits(ws: &Mutex<Self>, candidate: &str, wanted: &TabDocument) -> bool {
        match Self::with_quiet(ws, candidate, |s| s.tab_info().document).0 {
            Ok(document) => document == *wanted,
            Err(e) => {
                tracing::warn!(tab = candidate, error = %e.one_line(), "theme tab candidate not woken: skipped");
                false
            }
        }
    }

    /// Focuses theme tab `candidate`: whether it was shown; `None`: it left its window meanwhile.
    ///
    /// It becomes its window's active tab for the user (`ui`), or when `tab`
    /// is its window's active tab under this guard. The user's focus from
    /// another tab makes it the theme tab's opener; an agent's never does, as for packs.
    fn activate_theme_tab(ws: &Mutex<Self>, tab: &str, candidate: &str, ui: bool) -> Result<Option<bool>> {
        let mut w = Self::lock(ws);
        if w.window_of(candidate).is_err() {
            return Ok(None);
        }
        let shown = ui || w.tab_shown(tab);
        if shown {
            w.activate(candidate)?;
        }
        if ui
            && candidate != tab
            && w.tabs.contains_key(tab)
            && let Some(t) = w.tabs.get_mut(candidate)
        {
            t.opener = Some(tab.to_owned());
        }
        w.save_layout();
        Ok(Some(shown))
    }
}

#[cfg(test)]
#[path = "workspace_themes_tests.rs"]
mod workspace_themes_tests;
