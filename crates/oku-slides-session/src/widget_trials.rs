//! Brouillon du mode modèles : les valeurs d'essai des widgets.
//!
//! Par widget, en mémoire, jamais écrit dans le pack. L'aperçu, les réglages
//! de l'hôte et le contexte de l'agent les lisent ; leur révision relance
//! les aperçus.

use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::WidgetFile;
use serde_json::{Map, Value};

use crate::{Mode, Session};

/// Essais par widget, et leur révision.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TemplateScratch {
    trials: BTreeMap<WidgetFile, Map<String, Value>>,
    revision: u64,
    /// Widgets écrits par l'agent depuis la dernière écriture de l'utilisateur.
    agent_wrote: BTreeSet<WidgetFile>,
}

impl TemplateScratch {
    /// Les essais de `name`, s'il en a.
    pub(crate) fn trials_of(&self, name: &WidgetFile) -> Option<&Map<String, Value>> {
        self.trials.get(name)
    }

    /// Fixe (`Some`) ou efface (`None`) l'essai de `param`.
    pub(crate) fn set_trial(&mut self, name: &WidgetFile, param: &str, value: Option<Value>) {
        let changed = match value {
            Some(v) if self.trials.get(name).and_then(|m| m.get(param)) == Some(&v) => false,
            Some(v) => {
                self.trials.entry(name.clone()).or_default().insert(param.to_owned(), v);
                true
            }
            None => {
                let gone = self.trials.get_mut(name).and_then(|m| m.remove(param)).is_some();
                self.trials.retain(|_, m| !m.is_empty());
                gone
            }
        };
        if changed {
            self.revision += 1;
        }
    }

    /// Les essais de `from` passent à `to`.
    pub(crate) fn renamed(&mut self, from: &WidgetFile, to: &WidgetFile) {
        if self.agent_wrote.remove(from) {
            self.agent_wrote.insert(to.clone());
        }
        if let Some(m) = self.trials.remove(from) {
            self.trials.insert(to.clone(), m);
            self.revision += 1;
        }
    }

    /// Le widget `name` n'existe plus.
    pub(crate) fn removed(&mut self, name: &WidgetFile) {
        self.agent_wrote.remove(name);
        if self.trials.remove(name).is_some() {
            self.revision += 1;
        }
    }

    /// Ne garde que les essais acceptés par `keep` (paramètre, valeur).
    pub(crate) fn pruned(&mut self, name: &WidgetFile, keep: impl Fn(&str, &Value) -> bool) {
        let Some(m) = self.trials.get_mut(name) else { return };
        let before = m.len();
        m.retain(|k, v| keep(k, v));
        let after = m.len();
        self.trials.retain(|_, m| !m.is_empty());
        if after != before {
            self.revision += 1;
        }
    }

    /// L'agent vient d'écrire `name`.
    pub(crate) fn mark_agent(&mut self, name: &WidgetFile) {
        self.agent_wrote.insert(name.clone());
    }

    /// Vrai si l'agent avait écrit `name` ; la marque est retirée.
    pub(crate) fn take_agent_mark(&mut self, name: &WidgetFile) -> bool {
        self.agent_wrote.remove(name)
    }

    /// Augmente à chaque changement effectif.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
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
#[path = "widget_trials_tests.rs"]
mod widget_trials_tests;
