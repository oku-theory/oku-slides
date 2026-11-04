//! L'onglet d'aide : un seul par langue, rouvert plutôt que dupliqué, au
//! thème du deck d'où on l'ouvre. An agent opens it searched (`agent_help`), at the first hit.

use std::sync::Mutex;

use uuid::Uuid;

use super::tabs::Activate;
use super::{Opening, Pending, TabPlace, Workspace};
use crate::Session;
use crate::agent::{HELP_HITS, HelpHit, HelpOpen, TabGate, TabOpen, WorkspaceRefusal};
use crate::help::help_locale;

impl Workspace {
    /// La langue de l'aide à montrer : celle de l'interface, sinon l'anglais.
    pub(super) fn help_language(&self) -> &'static str {
        help_locale(oku_slides_i18n::resolve(self.chosen_locale))
    }

    /// The tab that already shows the help in `locale`.
    pub(super) fn tab_with_help(&self, locale: &str) -> Option<String> {
        self.tabs.iter().find(|(_, t)| t.info.document.help_locale() == Some(locale)).map(|(id, _)| id.clone())
    }

    /// The theme of the deck in `window`'s active tab; `None`: the help itself, or no theme.
    pub(super) fn help_theme(&self, window: &str) -> Option<Uuid> {
        let active = &self.layout.windows.iter().find(|w| w.label == window)?.active;
        let info = &self.tabs.get(active)?.info;
        info.document.help_locale().is_none().then_some(info.theme).flatten()
    }

    /// Prepares the help tab in `locale`, in theme `theme`, at the end of `window`, showing `slide`.
    pub(super) fn help_pending(
        &mut self,
        window: String,
        locale: &'static str,
        theme: Option<Uuid>,
        slide: Option<String>,
    ) -> Pending {
        self.pending(Some(window), Opening::Help { locale, theme, slide }, TabPlace::End)
    }

    /// The help tab `tab` shows `slide`; an unknown slide or a failure is logged.
    pub(super) fn show_help_slide(ws: &Mutex<Self>, tab: &str, slide: &str) {
        match Self::with_quiet(ws, tab, |s| s.show_help_slide(slide)).0 {
            Ok(true) => {}
            Ok(false) => tracing::debug!(tab, "help slide unknown: the help stays where it was"),
            Err(e) => tracing::warn!(tab, error = %e.one_line(), "help slide not shown"),
        }
    }

    /// L'onglet d'aide `tab` prend le thème `theme` ; un échec est journalisé.
    pub(super) fn retheme_help(ws: &Mutex<Self>, tab: &str, theme: Uuid) {
        match Self::with_quiet(ws, tab, |s| s.set_help_theme(theme)).0 {
            Ok(Ok(())) => {}
            Ok(Err(e)) | Err(e) => tracing::warn!(tab, %theme, error = %e.one_line(), "thème de l'aide inchangé"),
        }
    }
}

impl Workspace {
    /// Opens or shows the help for `tab`'s agent, searched for `query`, once `gate` grants it.
    ///
    /// The help tab is the workspace's one in the interface language, wherever it is; a new
    /// one goes at the end of `tab`'s window. It is activated only if `tab` is its window's
    /// active tab under the guard that activates it. With `query`, the help session is
    /// searched (once built, for a new tab) and shows the first hit, else its first slide.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `Closing`; a help tab that did not build is the session's error.
    ///
    /// # Blocking
    ///
    /// Builds the help tab's session outside the lock, or searches the one open.
    pub fn agent_help(
        ws: &Mutex<Self>,
        tab: &str,
        query: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<HelpOpen, WorkspaceRefusal> {
        let (window, locale, theme, existing) = {
            let w = Self::lock(ws);
            let window = w.agent_window(tab)?.to_owned();
            let locale = w.help_language();
            let theme = w.help_theme(&window);
            (window, locale, theme, w.tab_with_help(locale))
        };
        if existing.as_deref() == Some(tab) {
            let (hits, _) = Self::with_quiet(ws, tab, |s| searched(s, query)).0.map_err(WorkspaceRefusal::session)?;
            return Ok(HelpOpen { open: TabOpen::Here, hits });
        }
        let asked = if existing.is_some() { TabGate::Focus } else { TabGate::NewTab };
        if !gate(asked) {
            return Ok(HelpOpen { open: TabOpen::Refused, hits: Vec::new() });
        }
        match existing {
            Some(help) => Self::agent_help_reused(ws, tab, &help, theme, query),
            None => Self::agent_help_new(ws, tab, (window, locale, theme), query),
        }
    }

    /// [`Self::agent_help`] on the help tab `help` already open: searched, then activated.
    fn agent_help_reused(
        ws: &Mutex<Self>,
        tab: &str,
        help: &str,
        theme: Option<Uuid>,
        query: Option<&str>,
    ) -> Result<HelpOpen, WorkspaceRefusal> {
        let searched = Self::with_quiet(ws, help, |s| searched(s, query)).0;
        let (hits, _) = searched.map_err(|e| Self::lock(ws).agent_refusal(e))?;
        let shown = {
            let mut w = Self::lock(ws);
            w.agent_window(tab)?;
            if !w.tabs.contains_key(help) {
                return Err(WorkspaceRefusal::unknown_tab());
            }
            let shown = w.tab_shown(tab);
            if shown {
                w.activate(help).map_err(|e| w.agent_refusal(e))?;
                w.save_layout();
            }
            shown
        };
        // As the menu's help: it takes the theme of the deck it is opened from.
        if let Some(theme) = theme {
            Self::retheme_help(ws, help, theme);
        }
        Ok(HelpOpen { open: TabOpen::Focused { tab: help.to_owned(), shown }, hits })
    }

    /// [`Self::agent_help`] with no help tab yet: one is built, searched, then inserted.
    fn agent_help_new(
        ws: &Mutex<Self>,
        tab: &str,
        (window, locale, theme): (String, &'static str, Option<Uuid>),
        query: Option<&str>,
    ) -> Result<HelpOpen, WorkspaceRefusal> {
        let mut pending = {
            let mut w = Self::lock(ws);
            w.agent_window(tab)?;
            let mut pending = w.help_pending(window, locale, theme, None);
            pending.activate = Activate::IfShown(tab.to_owned());
            pending
        };
        let mut session = pending.build().map_err(WorkspaceRefusal::session)?;
        let (hits, slide) = searched(&mut session, query);
        // Another open may insert the help meanwhile: this one then joins it, at this slide.
        if let Opening::Help { slide: at, .. } = &mut pending.opening {
            *at = slide;
        }
        let (_, id, shown) = Self::finish_shown(ws, pending, session).map_err(|e| Self::lock(ws).agent_refusal(e))?;
        Ok(HelpOpen { open: TabOpen::Opened { tab: id, shown }, hits })
    }
}

/// Searches help session `s` for `query` and shows the first hit, else the first slide.
///
/// Returns the hits and the slide shown; without `query`, nothing is searched or shown.
fn searched(s: &mut Session, query: Option<&str>) -> (Vec<HelpHit>, Option<String>) {
    let Some(query) = query else { return (Vec::new(), None) };
    let Some(hits) = s.search_help(query, HELP_HITS) else {
        // Built as the help just before: a bug, told, and the tab opens with no hit.
        tracing::error!("help search outside the help session");
        return (Vec::new(), None);
    };
    let slide = hits.first().map(|h| h.slide.clone()).or_else(|| s.first_help_slide());
    if let Some(slide) = &slide {
        s.show_help_slide(slide);
    }
    (hits, slide)
}

#[cfg(test)]
#[path = "workspace_help_tests.rs"]
mod workspace_help_tests;
