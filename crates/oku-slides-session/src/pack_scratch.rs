//! Pack mode scratch: what the session keeps outside the pack.
//!
//! The sources the agent wrote, with whether each widget still holds it, and the preview
//! step that pinned the session version. The example values live in the pack
//! itself (`PackManifest::examples`, `widget_examples.rs`).

use std::collections::BTreeMap;

use oku_slides_core::{PackFiles, WidgetFile};
use uuid::Uuid;

use crate::widgets_edit::renamed_source;
use crate::{Mode, Session};

/// Le brouillon d'une séance du mode pack.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PackScratch {
    /// The source the agent last wrote in each widget (t-1403).
    agent_wrote: BTreeMap<WidgetFile, AgentMark>,
    /// The preview's undo step that pinned the session version; `None`: unknown.
    ///
    /// A later save that lands in that same step makes the replaced version
    /// unreachable; in any other step an undo can bring it back (t-1237).
    /// Kept in memory only: unknown after a resume or an undo, the replaced
    /// version is then kept.
    pub(crate) adopt_step: Option<u64>,
    /// The pack this session first copied, which its opener may switch from.
    pub(crate) copied_from: Option<Uuid>,
}

/// The agent's source for a widget, and whether it is still unseen.
#[derive(Debug, Clone, PartialEq)]
struct AgentMark {
    /// The widget's source as the agent left it.
    source: String,
    /// The widget holds `source` and no user write has told the agent yet.
    live: bool,
}

impl PackScratch {
    /// Widget `from` is now `to`; the agent's source is renamed the same way.
    pub(crate) fn renamed(&mut self, from: &WidgetFile, to: &WidgetFile) {
        if let Some(mark) = self.agent_wrote.remove(from) {
            let source = renamed_source(&mark.source, from.as_str(), to.as_str());
            self.agent_wrote.insert(to.clone(), AgentMark { source, ..mark });
        }
    }

    /// Le widget `name` n'existe plus.
    pub(crate) fn removed(&mut self, name: &WidgetFile) {
        self.agent_wrote.remove(name);
    }

    /// The agent just wrote `source` in `name`.
    pub(crate) fn mark_agent(&mut self, name: &WidgetFile, source: &str) {
        self.agent_wrote.insert(name.clone(), AgentMark { source: source.to_owned(), live: true });
    }

    /// Realigns the marks on `files`, after an undo or redo.
    ///
    /// A mark is live again where the widget holds the agent's source, asleep
    /// elsewhere. A widget `files` lacks loses its mark: a redo that brings it
    /// back does not, so the agent misses a note rather than gets a false one.
    /// Missed too: an undone rename (the mark went with the new name) and an
    /// undo back to an older agent write (only the last source is kept).
    pub(crate) fn kept_in(&mut self, files: &PackFiles) {
        self.agent_wrote.retain(|name, mark| match files.widget(name) {
            Some(source) => {
                mark.live = source == mark.source;
                true
            }
            None => false,
        });
    }

    /// True if `name` held the agent's source unseen; the mark goes to sleep.
    ///
    /// It stays, so an undo of the user's write can wake it (`kept_in`).
    pub(crate) fn take_agent_mark(&mut self, name: &WidgetFile) -> bool {
        self.agent_wrote.get_mut(name).is_some_and(|mark| std::mem::replace(&mut mark.live, false))
    }
}

impl Session {
    /// Vrai en mode pack.
    pub fn in_pack_mode(&self) -> bool {
        self.scratch().is_some()
    }

    /// Le brouillon du mode pack ; `None` ailleurs.
    pub(crate) fn scratch(&self) -> Option<&PackScratch> {
        match &self.mode {
            Mode::Pack { scratch, .. } => Some(scratch),
            _ => None,
        }
    }

    /// Le brouillon du mode pack, modifiable ; `None` ailleurs.
    pub(crate) fn scratch_mut(&mut self) -> Option<&mut PackScratch> {
        match &mut self.mode {
            Mode::Pack { scratch, .. } => Some(scratch),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "pack_scratch_tests.rs"]
mod pack_scratch_tests;
