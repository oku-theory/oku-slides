//! Annuler / rétablir : des étapes à nous, chacune faite de plusieurs items
//! de l'`UndoManager` de Loro, qui ne défait que les changements de ce pair.
//!
//! Loro 1.16.2 défait mal un item qui porte plusieurs `mov` sur une liste de
//! conteneurs (élément perdu, voire panique ; t-263). La réconciliation
//! committe donc avant chaque `mov` : une application est une étape, faite
//! d'un item par commit, défaits ou rétablis tous ensemble. Pour la même
//! raison, les groupes (un tour de l'agent) sont des étapes à nous, jamais
//! un `group_start` de Loro, qui fusionnerait leurs items en un seul.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use loro::{LoroDoc, LoroResult, UndoItemMeta, UndoManager, UndoOrRedo};

/// Origine des commits non annulables.
pub(super) const SILENT: &str = "oku-silent";

/// Étapes gardées tant que [`Undo::set_limit`] n'a pas été appelé.
const DEFAULT_LIMIT: usize = 100;

/// Items Loro gardés par étape, en moyenne : au-delà, Loro écarte les plus
/// anciens. Borne la mémoire des items d'étapes déjà écartées par la limite.
const ITEMS_PER_STEP: usize = 64;

/// Une étape d'annulation : `items` items Loro consécutifs, sous un libellé.
#[derive(Debug)]
struct Step {
    label: String,
    items: usize,
}

pub(super) struct Undo {
    manager: UndoManager,
    /// Items empilés par des commits locaux, pas encore rangés dans une étape.
    pushed: Arc<AtomicUsize>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// Libellé du groupe ouvert, et si l'étape du sommet lui appartient déjà.
    group: Option<(String, bool)>,
    limit: usize,
}

impl Undo {
    pub(super) fn new(doc: &LoroDoc) -> Self {
        let mut manager = UndoManager::new(doc);
        // Une application = une étape (spec) : jamais de fusion de commits
        // rapprochés, quel que soit le défaut de Loro.
        manager.set_merge_interval(0);
        manager.add_exclude_origin_prefix(SILENT);
        let pushed: Arc<AtomicUsize> = Arc::default();
        let p = Arc::clone(&pushed);
        manager.set_on_push(Some(Box::new(move |kind, _span, _event| {
            if kind == UndoOrRedo::Undo {
                p.fetch_add(1, Ordering::Relaxed);
            }
            UndoItemMeta::new()
        })));
        let mut undo = Self { manager, pushed, undo: Vec::new(), redo: Vec::new(), group: None, limit: 0 };
        undo.set_limit(DEFAULT_LIMIT);
        undo
    }

    /// Range les items des derniers commits dans une étape `label` (ou dans celle du groupe ouvert).
    pub(super) fn close_step(&mut self, label: &str) {
        let items = self.pushed.swap(0, Ordering::Relaxed);
        // Un commit vide n'a rien empilé : pas d'étape, et le rétablir reste possible.
        if items == 0 {
            return;
        }
        // Loro vide sa pile de rétablissement à chaque item local : la nôtre suit.
        self.redo.clear();
        match &mut self.group {
            Some((_, true)) => {
                if let Some(top) = self.undo.last_mut() {
                    top.items += items;
                }
            }
            Some((group, open)) => {
                self.undo.push(Step { label: group.clone(), items });
                *open = true;
            }
            None => self.undo.push(Step { label: label.to_owned(), items }),
        }
        self.trim();
    }

    /// Écarte les étapes les plus anciennes : au-delà de la limite, ou déjà
    /// entamées par Loro (une étape en partie écartée ne se défait plus).
    fn trim(&mut self) {
        let over = self.undo.len().saturating_sub(self.limit);
        self.undo.drain(..over);
        let mut kept = self.manager.undo_count();
        let first = self.undo.iter().rev().position(|s| match kept.checked_sub(s.items) {
            Some(rest) => {
                kept = rest;
                false
            }
            None => true,
        });
        if let Some(from_top) = first {
            self.undo.drain(..self.undo.len() - from_top);
        }
    }

    pub(super) fn begin_group(&mut self, label: &str) {
        self.group = Some((label.to_owned(), false));
    }

    pub(super) fn end_group(&mut self) {
        self.group = None;
    }

    pub(super) fn in_group(&self) -> bool {
        self.group.is_some()
    }

    /// Défait la dernière étape ; `false` s'il n'y en avait pas.
    pub(super) fn undo(&mut self) -> LoroResult<bool> {
        self.end_group();
        let Some(step) = self.undo.pop() else { return Ok(false) };
        let (done, r) = self.replay(step.items, UndoManager::undo);
        self.redo.push(Step { label: step.label, items: done });
        r.map(|()| true)
    }

    /// Rétablit la dernière étape défaite ; `false` s'il n'y en avait pas.
    pub(super) fn redo(&mut self) -> LoroResult<bool> {
        self.end_group();
        let Some(step) = self.redo.pop() else { return Ok(false) };
        let (done, r) = self.replay(step.items, UndoManager::redo);
        self.undo.push(Step { label: step.label, items: done });
        r.map(|()| true)
    }

    /// Joue `items` fois `f` ; renvoie le nombre d'items joués, qui passent d'une pile à l'autre.
    fn replay(&mut self, items: usize, f: fn(&mut UndoManager) -> LoroResult<bool>) -> (usize, LoroResult<()>) {
        // Rétablir empile des items d'annulation : ce ne sont pas des commits locaux.
        let pushed = self.pushed.load(Ordering::Relaxed);
        let mut done = 0;
        let mut r = Ok(());
        while done < items {
            match f(&mut self.manager) {
                Ok(true) => done += 1,
                Ok(false) => break,
                Err(e) => {
                    r = Err(e);
                    break;
                }
            }
        }
        self.pushed.store(pushed, Ordering::Relaxed);
        (done, r)
    }

    /// Nombre maximal d'étapes gardées ; `0` vaut `1`.
    pub(super) fn set_limit(&mut self, steps: usize) {
        self.limit = steps.max(1);
        self.manager.set_max_undo_steps(self.limit.saturating_mul(ITEMS_PER_STEP));
        self.trim();
    }

    pub(super) fn top_undo(&self) -> Option<String> {
        self.undo.last().map(|s| s.label.clone())
    }

    pub(super) fn top_redo(&self) -> Option<String> {
        self.redo.last().map(|s| s.label.clone())
    }

    pub(super) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(super) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
#[path = "undo_tests.rs"]
mod undo_tests;
