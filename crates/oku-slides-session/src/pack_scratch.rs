//! Pack mode scratch: what the session keeps outside the pack.
//!
//! The widgets the agent wrote since the user's last write, and the preview
//! step that pinned the session version. The example values live in the pack
//! itself (`PackManifest::examples`, `widget_examples.rs`).

use std::collections::BTreeSet;

use oku_slides_core::WidgetFile;
use uuid::Uuid;

use crate::{Mode, Session};

/// Le brouillon d'une séance du mode pack.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PackScratch {
    /// Widgets écrits par l'agent depuis la dernière écriture de l'utilisateur.
    agent_wrote: BTreeSet<WidgetFile>,
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

impl PackScratch {
    /// Le widget `from` s'appelle désormais `to`.
    pub(crate) fn renamed(&mut self, from: &WidgetFile, to: &WidgetFile) {
        if self.agent_wrote.remove(from) {
            self.agent_wrote.insert(to.clone());
        }
    }

    /// Le widget `name` n'existe plus.
    pub(crate) fn removed(&mut self, name: &WidgetFile) {
        self.agent_wrote.remove(name);
    }

    /// L'agent vient d'écrire `name`.
    pub(crate) fn mark_agent(&mut self, name: &WidgetFile) {
        self.agent_wrote.insert(name.clone());
    }

    /// Vrai si l'agent avait écrit `name` ; la marque est retirée.
    pub(crate) fn take_agent_mark(&mut self, name: &WidgetFile) -> bool {
        self.agent_wrote.remove(name)
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
