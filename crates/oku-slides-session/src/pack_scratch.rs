//! Brouillon du mode pack : ce que la séance garde hors du pack.
//!
//! Les widgets écrits par l'agent depuis la dernière écriture de
//! l'utilisateur. Les valeurs d'exemple, elles, sont dans le pack
//! (`PackManifest::examples`, `widget_examples.rs`).

use std::collections::BTreeSet;

use oku_slides_core::WidgetFile;

use crate::{Mode, Session};

/// Le brouillon d'une séance du mode pack.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PackScratch {
    /// Widgets écrits par l'agent depuis la dernière écriture de l'utilisateur.
    agent_wrote: BTreeSet<WidgetFile>,
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
