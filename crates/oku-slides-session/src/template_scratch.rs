//! Brouillon du mode modèles : ce que la séance garde hors du pack.
//!
//! Les widgets écrits par l'agent depuis la dernière écriture de
//! l'utilisateur. Les valeurs d'exemple, elles, sont dans le pack
//! (`PackManifest::examples`, `widget_examples.rs`).

use std::collections::BTreeSet;

use oku_slides_core::WidgetFile;

use crate::{Mode, Session};

/// Le brouillon d'une séance du mode modèles.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TemplateScratch {
    /// Widgets écrits par l'agent depuis la dernière écriture de l'utilisateur.
    agent_wrote: BTreeSet<WidgetFile>,
}

impl TemplateScratch {
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
    /// Vrai en mode modèles.
    pub fn in_templates(&self) -> bool {
        self.scratch().is_some()
    }

    /// Le brouillon du mode modèles ; `None` ailleurs.
    pub(crate) fn scratch(&self) -> Option<&TemplateScratch> {
        match &self.mode {
            Mode::Templates { scratch, .. } => Some(scratch),
            _ => None,
        }
    }

    /// Le brouillon du mode modèles, modifiable ; `None` ailleurs.
    pub(crate) fn scratch_mut(&mut self) -> Option<&mut TemplateScratch> {
        match &mut self.mode {
            Mode::Templates { scratch, .. } => Some(scratch),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "template_scratch_tests.rs"]
mod template_scratch_tests;
