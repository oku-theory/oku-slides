//! Historique annuler / rétablir, par transactions : une dictée appliquée ou
//! un glisser-déposer comptent pour une seule annulation.

use serde::{Deserialize, Serialize};

use crate::model::Deck;
use crate::ops::{Op, OpError, apply_all};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Libellé affiché (« Dictée : passer l'accent en bleu », « Déplacer »…).
    pub label: String,
    pub ops: Vec<Op>,
    /// Inverses, dans l'ordre d'application pour défaire.
    pub inverse: Vec<Op>,
}

#[derive(Debug, Clone)]
pub struct History {
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new(500)
    }
}

impl History {
    pub fn new(limit: usize) -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), limit: limit.max(1) }
    }

    /// Applique `ops` en une transaction. Atomique : en cas d'erreur, rien n'a changé.
    pub fn commit(&mut self, deck: &mut Deck, label: impl Into<String>, ops: Vec<Op>) -> Result<(), (usize, OpError)> {
        let inverse = apply_all(deck, &ops)?;
        self.undo.push(Transaction { label: label.into(), ops, inverse });
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        Ok(())
    }

    /// Défait la dernière transaction ; renvoie son libellé.
    pub fn undo(&mut self, deck: &mut Deck) -> Result<Option<String>, (usize, OpError)> {
        let Some(tx) = self.undo.pop() else { return Ok(None) };
        match apply_all(deck, &tx.inverse) {
            Ok(_) => {
                let label = tx.label.clone();
                self.redo.push(tx);
                Ok(Some(label))
            }
            Err(e) => {
                self.undo.push(tx);
                Err(e)
            }
        }
    }

    /// Rejoue la dernière transaction défaite ; renvoie son libellé.
    pub fn redo(&mut self, deck: &mut Deck) -> Result<Option<String>, (usize, OpError)> {
        let Some(tx) = self.redo.pop() else { return Ok(None) };
        match apply_all(deck, &tx.ops) {
            Ok(inverse) => {
                let label = tx.label.clone();
                self.undo.push(Transaction { inverse, ..tx });
                Ok(Some(label))
            }
            Err(e) => {
                self.redo.push(tx);
                Err(e)
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|t| t.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|t| t.label.as_str())
    }

    /// Journal des transactions, une par ligne (JSON Lines), pour `history.jsonl`.
    pub fn to_jsonl(&self) -> String {
        self.undo.iter().map(|t| serde_json::to_string(t).expect("sérialisable")).collect::<Vec<_>>().join("\n")
    }
}
