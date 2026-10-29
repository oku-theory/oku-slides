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
//! theme tab has no preview seed and no agent gate. A new theme (`new_theme`)
//! is created empty through the requesting tab's library, then opened; its
//! draft goes if that open fails.

use std::sync::Mutex;

use uuid::Uuid;

use crate::TabDocument;
use crate::error::SingleDocumentSnafu;
use crate::library_packs::DraftKind;
use crate::pack_lease::PackLease;

use super::opening::with_opening;
#[cfg(test)]
use super::tabs::open_failpoint;
use super::{Created, Opening, Result, TabPlace, Workspace, WorkspaceOutcome};

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
        with_opening(theme, || Self::open_theme(ws, tab, theme, Created::No))?;
        Ok(WorkspaceOutcome::Applied(Self::lock(ws).view_json()?))
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
        let opened = with_opening(id, || Self::open_theme(ws, tab, id, Created::Yes));
        if opened.is_err() {
            Self::roll_back(&library, DraftKind::Theme, id);
        }
        opened?;
        Ok(WorkspaceOutcome::Applied(Self::lock(ws).view_json()?))
    }

    /// Focuses the theme tab of `theme`, or opens one after `tab`: its id.
    fn open_theme(ws: &Mutex<Self>, tab: &str, theme: Uuid, created: Created) -> Result<String> {
        #[cfg(test)]
        open_failpoint()?;
        let wanted = TabDocument::Theme { theme };
        let (window, candidates) = Self::lock(ws).candidates(tab, std::slice::from_ref(&wanted))?;
        for candidate in &candidates {
            if Self::focus_theme_tab(ws, tab, &wanted, candidate)? {
                return Ok(candidate.clone());
            }
        }
        let (route, _changed) = Self::with_quiet(ws, tab, |s| s.route_theme(theme));
        let (scope, name) = route??;
        let lease = match scope.map(|scope| PackLease::acquire(&scope, theme, &name, &name)).transpose() {
            Ok(lease) => lease,
            Err(refused) => {
                let live = Self::lock(ws).live_tab(&wanted);
                return match live {
                    Some(holder) if Self::focus_theme_tab(ws, tab, &wanted, &holder)? => Ok(holder),
                    Some(_) | None => Err(refused),
                };
            }
        };
        let pending = {
            let mut w = Self::lock(ws);
            // The requesting tab closed meanwhile: its window, at the end and without opener.
            let place = if w.tab_window(tab).is_some() { TabPlace::After(tab.to_owned()) } else { TabPlace::End };
            let window = w.tab_window(tab).map_or(window, str::to_owned);
            w.pending(Some(window), Opening::Theme { theme, created }, place)
        };
        let session = pending.build_theme(lease)?;
        Self::finish(ws, pending, session).map(|(_, id)| id)
    }

    /// Focuses `candidate` if its live session edits `wanted`; false: skipped.
    ///
    /// A focus from another tab makes it the theme tab's opener.
    fn focus_theme_tab(ws: &Mutex<Self>, tab: &str, wanted: &TabDocument, candidate: &str) -> Result<bool> {
        let (live, _changed) = Self::with_quiet(ws, candidate, |s| s.tab_info().document);
        match live {
            Ok(document) if document == *wanted => {}
            Ok(_) => return Ok(false),
            Err(e) => {
                tracing::warn!(tab = candidate, error = %e.one_line(), "theme tab candidate not woken: skipped");
                return Ok(false);
            }
        }
        let mut w = Self::lock(ws);
        if w.window_of(candidate).is_err() {
            return Ok(false);
        }
        w.activate(candidate)?;
        if candidate != tab
            && w.tabs.contains_key(tab)
            && let Some(t) = w.tabs.get_mut(candidate)
        {
            t.opener = Some(tab.to_owned());
        }
        w.save_layout();
        Ok(true)
    }
}

#[cfg(test)]
#[path = "workspace_themes_tests.rs"]
mod workspace_themes_tests;
