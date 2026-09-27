//! Annuler / rétablir : l'`UndoManager` de Loro, qui ne défait que les
//! changements de ce pair, avec le libellé de chaque transaction.

use std::sync::{Arc, Mutex, PoisonError};

use loro::{LoroDoc, LoroValue, UndoItemMeta, UndoManager, UndoOrRedo};

/// Origine des commits non annulables.
pub(super) const SILENT: &str = "oku-silent";

pub(super) struct Undo {
    manager: UndoManager,
    /// Libellé du prochain commit local.
    next: Arc<Mutex<Option<String>>>,
}

fn take<T>(m: &Mutex<Option<T>>) -> Option<T> {
    m.lock().unwrap_or_else(PoisonError::into_inner).take()
}

impl Undo {
    pub(super) fn new(doc: &LoroDoc) -> Self {
        let mut manager = UndoManager::new(doc);
        // Une application = une étape (spec) : jamais de fusion de commits
        // rapprochés, quel que soit le défaut de Loro.
        manager.set_merge_interval(0);
        manager.add_exclude_origin_prefix(SILENT);
        let next: Arc<Mutex<Option<String>>> = Arc::default();
        // `on_pop` précède le `on_push` de l'autre pile : le libellé passe de l'une à l'autre.
        let popped: Arc<Mutex<Option<LoroValue>>> = Arc::default();
        let (n, p) = (Arc::clone(&next), Arc::clone(&popped));
        manager.set_on_push(Some(Box::new(move |kind, _span, _event| {
            let value = match kind {
                UndoOrRedo::Undo => take(&n).map(LoroValue::from).or_else(|| take(&p)),
                UndoOrRedo::Redo => take(&p),
            };
            UndoItemMeta { value: value.unwrap_or(LoroValue::Null), cursors: Vec::new() }
        })));
        let p = Arc::clone(&popped);
        manager.set_on_pop(Some(Box::new(move |_kind, _span, meta| {
            *p.lock().unwrap_or_else(PoisonError::into_inner) = Some(meta.value);
        })));
        Self { manager, next }
    }

    pub(super) fn label_next(&self, label: &str) {
        *self.next.lock().unwrap_or_else(PoisonError::into_inner) = Some(label.to_owned());
    }

    /// Après un commit : un commit vide n'a rien empilé, son libellé ne doit pas fuir.
    pub(super) fn clear_next(&self) {
        take(&self.next);
    }

    pub(super) fn manager(&mut self) -> &mut UndoManager {
        &mut self.manager
    }

    pub(super) fn label(v: Option<LoroValue>) -> Option<String> {
        match v {
            Some(LoroValue::String(s)) => Some(s.to_string()),
            _ => None,
        }
    }

    pub(super) fn top_undo(&self) -> Option<String> {
        Self::label(self.manager.top_undo_value())
    }

    pub(super) fn top_redo(&self) -> Option<String> {
        Self::label(self.manager.top_redo_value())
    }

    pub(super) fn can_undo(&self) -> bool {
        self.manager.can_undo()
    }

    pub(super) fn can_redo(&self) -> bool {
        self.manager.can_redo()
    }
}

#[cfg(test)]
#[path = "undo_tests.rs"]
mod undo_tests;
