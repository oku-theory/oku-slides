//! The agent's open verdict: one tab move per turn, asked by the session's gate.

use oku_slides_session::agent::TabOpen;

use crate::SurfaceHub;
use crate::hub::{Reserve, TabTarget};
use crate::tool_error::{TabLimitSnafu, TabNoTurnSnafu, ToolError};

/// Why the agent's gate refused a tab open.
#[derive(Debug, Clone, Copy)]
enum Refusal {
    /// No open turn: the call arrived outside one, or its turn is over.
    NoTurn,
    /// The turn already claimed another tab move.
    Limit,
}

/// What the agent's gate answered while a pack or theme tab opened.
#[derive(Debug, Default)]
pub(crate) struct TabVerdict {
    /// Why the last gate refused, if it did; a later grant clears it.
    refusal: Option<Refusal>,
    /// A claim on the turn's one move was granted (and must be given back if no tab moves).
    claimed: bool,
}

impl TabVerdict {
    /// Grants a gate, `Focus` or `NewTab` alike: the first grant of a call claims the turn's one move.
    ///
    /// Later gates of the same call ride on that claim while the turn stays open. It never
    /// blocks; the hub lock it takes is a leaf.
    pub(crate) fn ask(&mut self, hub: &SurfaceHub, turn: Option<u32>, target: TabTarget) -> bool {
        let Some(turn) = turn else {
            self.refusal = Some(Refusal::NoTurn);
            return false;
        };
        if self.claimed {
            return self.answer(hub.turn_is_open(turn).then_some(()).ok_or(Refusal::NoTurn));
        }
        let answer = match hub.reserve_tab(turn, target) {
            Reserve::Granted => {
                self.claimed = true;
                Ok(())
            }
            Reserve::NotThisTurn => Err(Refusal::NoTurn),
            Reserve::Taken => Err(Refusal::Limit),
        };
        self.answer(answer)
    }

    /// Records an answer: a grant clears any refusal noted before it.
    fn answer(&mut self, answer: Result<(), Refusal>) -> bool {
        self.refusal = answer.err();
        answer.is_ok()
    }

    /// The error that tells the agent why the gate refused.
    pub(crate) fn refusal(&self) -> ToolError {
        match self.refusal {
            Some(Refusal::Limit) => TabLimitSnafu.build(),
            Some(Refusal::NoTurn) => TabNoTurnSnafu.build(),
            None => {
                tracing::error!("tab open refused without a gate verdict");
                TabNoTurnSnafu.build()
            }
        }
    }

    /// Gives back the claim unless the open moved a tab; notes writes elsewhere after a move.
    ///
    /// Only the `Opened` and `Focused` outcomes are moves; `Here`, `Refused` and an error are
    /// not. A blocking task that panics never reaches this call: its claim stays spent for the
    /// rest of the turn, so the turn fails closed.
    pub(crate) fn settle(
        &self,
        hub: &SurfaceHub,
        turn: Option<u32>,
        target: TabTarget,
        outcome: &oku_slides_session::Result<TabOpen>,
    ) {
        let Some(turn) = turn else { return };
        match outcome {
            Ok(TabOpen::Opened { .. } | TabOpen::Focused { .. }) => hub.note_elsewhere(turn),
            _ if self.claimed => hub.release_tab(turn, target),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "tab_gate_tests.rs"]
mod tab_gate_tests;
