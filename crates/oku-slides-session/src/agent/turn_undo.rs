//! Annuler un tour de l'agent depuis son reçu (t-417).
//!
//! Un tour est un groupe d'annulation du deck (`begin_turn`). La session note, par tour, les
//! rangs d'étapes (`DeckReplica::undo_step`) créées pendant qu'il était ouvert : le tour dont
//! l'étape est au sommet de la pile d'annulation est celui que le reçu propose d'annuler. Rien
//! d'autre n'est gardé : une fois une autre étape au-dessus, le reçu ne propose plus rien.

use std::collections::VecDeque;

use snafu::ensure;

use super::KEPT_TURNS;
use crate::error::TurnNotOnTopSnafu;
use crate::{Mode, Result, Session};

/// Les étapes d'un tour : rangs dans `before + 1 ..= after` (`after` absent tant qu'il est ouvert).
#[derive(Debug, Clone, Copy)]
struct TurnSpan {
    turn: u32,
    before: u64,
    after: Option<u64>,
}

/// Les étapes d'annulation du deck prises par les derniers tours de l'agent.
#[derive(Debug, Default)]
pub(crate) struct TurnSteps(VecDeque<TurnSpan>);

impl TurnSteps {
    /// Le tour `turn` s'ouvre alors que `steps` étapes ont été empilées.
    fn begin(&mut self, turn: u32, steps: u64) {
        self.end(steps);
        self.0.retain(|s| s.turn != turn);
        self.0.push_back(TurnSpan { turn, before: steps, after: None });
        while self.0.len() > KEPT_TURNS {
            self.0.pop_front();
        }
    }

    /// Le tour ouvert se ferme alors que `steps` étapes ont été empilées.
    fn end(&mut self, steps: u64) {
        if let Some(open) = self.0.back_mut().filter(|s| s.after.is_none()) {
            open.after = Some(steps);
        }
    }

    /// Le tour qui a créé l'étape de rang `step` (le tour ouvert compris).
    fn owner(&self, step: u64) -> Option<u32> {
        self.0.iter().rev().find(|s| s.before < step && s.after.is_none_or(|a| step <= a)).map(|s| s.turn)
    }
}

impl Session {
    /// Le deck de l'utilisateur, mis de côté ou non : celui dont le tour groupe les étapes.
    fn turn_deck_steps(&self) -> u64 {
        match &self.mode {
            // A theme tab's own document is the one its turns change.
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => self.doc.sources.undo_steps(),
            Mode::Pack { preview: stash, .. } | Mode::History { stash, .. } => stash.sources.undo_steps(),
        }
    }

    /// Note l'ouverture du tour `turn` ; seul un tour ouvert sur le deck peut s'annuler depuis son reçu.
    pub(super) fn turn_steps_begin(&mut self, turn: u32) {
        if matches!(self.mode, Mode::Deck | Mode::Theme { .. }) {
            let steps = self.turn_deck_steps();
            self.turn_steps.begin(turn, steps);
        }
    }

    /// Note la fermeture du tour ouvert.
    pub(super) fn turn_steps_end(&mut self) {
        let steps = self.turn_deck_steps();
        self.turn_steps.end(steps);
    }

    /// Le tour de l'agent dont l'étape est au sommet de la pile d'annulation du deck.
    ///
    /// `None` hors du mode deck : le mode pack et l'historique ont leur propre annuler.
    pub fn undo_turn(&self) -> Option<u32> {
        if !matches!(self.mode, Mode::Deck | Mode::Theme { .. }) {
            return None;
        }
        self.turn_steps.owner(self.doc.sources.replica().undo_step()?)
    }

    /// Undoes turn `turn` normally, if it is still the deck's last change.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, if the turn is no longer on top of the stack
    /// (`TurnNotOnTop`), or if the undo fails.
    pub fn undo_agent_turn(&mut self, turn: u32) -> Result<()> {
        self.ensure_document_editable()?;
        ensure!(self.undo_turn() == Some(turn), TurnNotOnTopSnafu { turn });
        self.undo()
    }
}

#[cfg(test)]
#[path = "turn_undo_tests.rs"]
mod turn_undo_tests;
