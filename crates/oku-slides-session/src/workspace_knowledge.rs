//! A deck's knowledge tab: a view over its deck tab, holding no session.
//!
//! At most one per deck tab, placed after it in its window, titled as it,
//! never dirty. It closes with its owner, in any window; restored only
//! after its owner, when that is a restored deck tab.

use std::collections::HashSet;
use std::sync::Mutex;

use snafu::{OptionExt, ensure};

use crate::agent::bounds::label;
use crate::agent::{TabOpen, WorkspaceRefusal};
use crate::error::{Result, UnknownTabSnafu, WrongDocumentSnafu};
use crate::{DocumentKind, TabDocument, TabInfo};

use super::{Tab, TabPlace, Workspace};

/// The info of deck tab `owner`'s knowledge tab: the owner's title, nothing else of it.
fn knowledge_info(owner: &str, title: String) -> TabInfo {
    TabInfo {
        title,
        dirty: false,
        dirty_revision: None,
        pristine: false,
        theme: None,
        document: TabDocument::Knowledge { deck_tab: owner.to_owned() },
        created: None,
        knowledge: None,
    }
}

impl Workspace {
    /// Opens deck tab `deck_tab`'s knowledge tab after it and activates it, or activates the one it has.
    ///
    /// Returns the knowledge tab and whether it is new.
    ///
    /// # Errors
    ///
    /// `UnknownTab` for an unknown owner (or once settled), `WrongDocument` for one that is not a deck tab.
    ///
    /// # Blocking
    ///
    /// Runs under the caller's workspace guard; reads no session.
    pub(crate) fn open_knowledge(&mut self, deck_tab: &str) -> Result<(String, bool)> {
        // As a closed tab: once settled, nothing opens.
        ensure!(!self.is_closed(), UnknownTabSnafu { id: deck_tab });
        self.knowledge_in(deck_tab, true)
    }

    /// Detaches and removes deck tab `owner`'s knowledge tab, once the owner is out.
    ///
    /// A detach failure is logged: the owner is gone, so it must not stop half-way.
    pub(super) fn remove_knowledge_of(&mut self, owner: &str) -> Option<(String, Tab)> {
        let t = self.knowledge_tab_of(owner)?;
        if let Err(e) = self.detach(&t) {
            tracing::warn!(tab = %t, error = %e.one_line(), "knowledge tab not detached");
        }
        self.remove_tab(&t)
    }

    /// The knowledge tab of deck tab `owner`, if open.
    #[must_use]
    pub(crate) fn knowledge_tab_of(&self, owner: &str) -> Option<String> {
        self.tabs
            .iter()
            .find(|(_, t)| matches!(&t.info.document, TabDocument::Knowledge { deck_tab } if deck_tab == owner))
            .map(|(id, _)| id.clone())
    }

    /// Opens, or focuses, `tab`'s knowledge tab for its agent: `Opened` or `Focused`, never else.
    ///
    /// Either is shown only if `tab` is its window's active tab under the guard that opens it
    /// (t-1191). No gate asks first: a read-only view holds no work to protect.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, `Closing`; `WrongDocument` (the session's) outside a deck tab.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock once.
    pub fn agent_open_knowledge(ws: &Mutex<Self>, tab: &str) -> Result<TabOpen, WorkspaceRefusal> {
        let mut w = Self::lock(ws);
        w.agent_window(tab)?;
        let shown = w.tab_shown(tab);
        let (id, new) = w.knowledge_in(tab, shown).map_err(|e| w.agent_refusal(e))?;
        Ok(if new { TabOpen::Opened { tab: id, shown } } else { TabOpen::Focused { tab: id, shown } })
    }

    /// Mirrors deck tab `owner`'s new `title` onto its knowledge tab; `true` if it changed.
    pub(super) fn mirror_title(&mut self, owner: &str, title: String) -> bool {
        let Some(id) = self.knowledge_tab_of(owner) else { return false };
        let Some(t) = self.tabs.get_mut(&id) else { return false };
        if t.info.title == title {
            return false;
        }
        t.info.title = title;
        true
    }

    /// Restores the knowledge entries `(id, owner)` whose owner is a restored deck tab without one yet.
    ///
    /// Each other entry is dropped (logged): it holds nothing to keep. So is one whose id another
    /// tab already holds, which it would replace.
    pub(super) fn restore_knowledge(&mut self, entries: Vec<(String, String)>, restored: &mut HashSet<String>) {
        for (id, owner) in entries {
            let title = self
                .tabs
                .get(&owner)
                .filter(|t| restored.contains(&owner) && matches!(t.info.document, TabDocument::Deck { .. }))
                .map(|t| t.info.title.clone());
            match title {
                Some(title) if self.knowledge_tab_of(&owner).is_none() && !self.tabs.contains_key(&id) => {
                    self.tabs.insert(id.clone(), Tab::view(knowledge_info(&owner, title)));
                    restored.insert(id);
                }
                _ => {
                    tracing::warn!(tab = %id, owner = %label(&owner), "knowledge tab dropped at resume: no owner deck, a duplicate, or its id taken");
                }
            }
        }
    }

    /// Refuses an owner that is not a deck tab (`UnknownTab`, `WrongDocument`).
    fn ensure_deck_owner(&self, owner: &str) -> Result<String> {
        let t = self.tabs.get(owner).context(UnknownTabSnafu { id: owner })?;
        let kind = t.info.document.kind();
        ensure!(kind == DocumentKind::Deck, WrongDocumentSnafu { kind });
        Ok(t.info.title.clone())
    }

    /// Opens `owner`'s knowledge tab after it, or activates the one it has, when `activate`.
    ///
    /// Not activated, a new tab still is in a window without a valid active tab.
    fn knowledge_in(&mut self, owner: &str, activate: bool) -> Result<(String, bool)> {
        let title = self.ensure_deck_owner(owner)?;
        if let Some(id) = self.knowledge_tab_of(owner) {
            if activate {
                self.activate(&id)?;
                self.save_layout();
            }
            return Ok((id, false));
        }
        let window = self.tab_window(owner).context(UnknownTabSnafu { id: owner })?.to_owned();
        let id = self.next_tab_id();
        let tab = Tab::view(knowledge_info(owner, title));
        self.insert_tab_with(Some(&window), id.clone(), tab, &TabPlace::After(owner.to_owned()), activate);
        self.save_layout();
        Ok((id, true))
    }
}

#[cfg(test)]
#[path = "workspace_knowledge_tests.rs"]
mod workspace_knowledge_tests;
