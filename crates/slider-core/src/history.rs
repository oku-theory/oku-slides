//! Historique annuler / rétablir, par transactions : une dictée appliquée ou
//! un glisser-déposer comptent pour une seule annulation.
//!
//! Chaque op appliquée renvoie son inverse : une transaction ne garde donc
//! que les ops qui font passer le deck de l'autre côté. Annuler et rétablir
//! sont le même geste, dans les deux sens.

use std::collections::VecDeque;

use crate::error::TransactionError;
use crate::model::Deck;
use crate::ops::{Op, apply_all};

#[derive(Debug, Clone, PartialEq)]
struct Transaction {
    /// Libellé affiché (« Dictée : passer l'accent en bleu », « Déplacer »…).
    label: String,
    /// Ops à appliquer pour passer la transaction dans l'autre pile.
    ops: Vec<Op>,
}

#[derive(Debug, Clone)]
pub struct History {
    undo: VecDeque<Transaction>,
    redo: VecDeque<Transaction>,
    limit: usize,
}

/// Applique la dernière transaction de `from` et range son inverse dans `to`.
/// En cas d'erreur (historique incohérent avec le deck), la transaction est
/// abandonnée et le deck reste intact.
fn step<'a>(
    from: &mut VecDeque<Transaction>,
    to: &'a mut VecDeque<Transaction>,
    deck: &mut Deck,
) -> Result<Option<&'a str>, TransactionError> {
    let Some(Transaction { label, ops }) = from.pop_back() else { return Ok(None) };
    let ops = apply_all(deck, ops)?;
    to.push_back(Transaction { label, ops });
    Ok(to.back().map(|t| t.label.as_str()))
}

impl History {
    pub fn new(limit: usize) -> Self {
        Self { undo: VecDeque::new(), redo: VecDeque::new(), limit: limit.max(1) }
    }

    /// Applique `ops` en une transaction. Atomique : en cas d'erreur, rien n'a changé.
    pub fn commit(&mut self, deck: &mut Deck, label: impl Into<String>, ops: Vec<Op>) -> Result<(), TransactionError> {
        let inverse = apply_all(deck, ops)?;
        self.undo.push_back(Transaction { label: label.into(), ops: inverse });
        if self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        self.redo.clear();
        Ok(())
    }

    /// Défait la dernière transaction ; renvoie son libellé.
    pub fn undo(&mut self, deck: &mut Deck) -> Result<Option<&str>, TransactionError> {
        step(&mut self.undo, &mut self.redo, deck)
    }

    /// Rejoue la dernière transaction défaite ; renvoie son libellé.
    pub fn redo(&mut self, deck: &mut Deck) -> Result<Option<&str>, TransactionError> {
        step(&mut self.redo, &mut self.undo, deck)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|t| t.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.back().map(|t| t.label.as_str())
    }
}
