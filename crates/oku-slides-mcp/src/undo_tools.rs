//! `undo`, `redo` and `undo_turn`: the tab's own undo stack, never confirmed (t-999).
//!
//! `undo` and `redo` play up to `steps` steps under one session lock and stop at the
//! first one refused or missing; the agent learns how many were done. `undo_turn`
//! undoes a finished turn still on top of the stack, never the turn it runs in.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::{Request, Session, SessionError};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::IntoError;

use crate::McpEvent;
use crate::tool_error::{SessionSnafu, UndoOpenTurnSnafu};
use crate::tools::Tools;

/// Most steps one `undo` or `redo` call plays.
pub(crate) const MAX_STEPS: u8 = 20;

/// A step count from 1 to [`MAX_STEPS`]; anything else is refused by serde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "u8")]
pub(crate) struct Steps(u8);

impl TryFrom<u8> for Steps {
    type Error = String;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        if (1..=MAX_STEPS).contains(&n) { Ok(Self(n)) } else { Err(format!("steps must be 1 to {MAX_STEPS}")) }
    }
}

impl Default for Steps {
    fn default() -> Self {
        Self(1)
    }
}

/// `undo` and `redo`: how many steps.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StepsArgs {
    /// Steps to play, 1 to 20 (default 1).
    #[serde(default)]
    #[schemars(with = "u8", range(min = 1, max = MAX_STEPS))]
    pub steps: Steps,
}

/// `undo_turn`: the finished turn to undo.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UndoTurnArgs {
    /// The turn's number, as the context's `undoable_turn` line gives it.
    pub turn: u32,
}

/// Which way a step goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Undo the top step.
    Undo,
    /// Redo the last undone step.
    Redo,
}

/// A document whose undo and redo stacks are played step by step.
pub(crate) trait Stepper {
    /// Why a step is refused.
    type Error;

    /// Is a step left to play `way`?
    fn left(&self, way: Direction) -> bool;

    /// Plays one step `way`.
    fn step(&mut self, way: Direction) -> Result<(), Self::Error>;
}

impl Stepper for Session {
    type Error = SessionError;

    fn left(&self, way: Direction) -> bool {
        match way {
            Direction::Undo => self.can_undo(),
            Direction::Redo => self.can_redo(),
        }
    }

    fn step(&mut self, way: Direction) -> Result<(), SessionError> {
        let req = match way {
            Direction::Undo => Request::Undo,
            Direction::Redo => Request::Redo,
        };
        self.handle(req).map(drop)
    }
}

/// What a run of steps did, and the refusal that stopped it.
#[derive(Debug)]
pub(crate) struct Played<E> {
    /// Steps played.
    pub done: u8,
    /// The refusal of the next step; `None` when the stack ran out or all were played.
    pub refused: Option<E>,
}

/// Plays up to `steps` steps `way`; stops at an empty stack or a refusal.
pub(crate) fn play<S: Stepper>(s: &mut S, way: Direction, steps: Steps) -> Played<S::Error> {
    let mut done = 0;
    while done < steps.0 && s.left(way) {
        if let Err(e) = s.step(way) {
            return Played { done, refused: Some(e) };
        }
        let Some(next) = done.checked_add(1) else { break };
        done = next;
    }
    Played { done, refused: None }
}

/// The tool's answer to `played`: `{done}`, with `stopped` after a step; a first refusal is the error.
pub(crate) fn outcome<E: Localize>(played: Played<E>, l: &Localizer) -> Result<Value, E> {
    match (played.done, played.refused) {
        (0, Some(e)) => Err(e),
        (done, Some(e)) => Ok(json!({ "done": done, "stopped": e.localize(l) })),
        (done, None) => Ok(json!({ "done": done })),
    }
}

impl Tools {
    /// `undo` or `redo`: returns `{done}`, and `stopped` after a refusal.
    ///
    /// The steps run under one session lock, so no edit slips between them.
    pub(crate) async fn undo_redo(&self, way: Direction, args: Value) -> CallToolResult {
        let StepsArgs { steps } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let played = match self.with_session(move |s| Ok(play(s, way, steps))).await {
            Ok(p) => Played { done: p.done, refused: p.refused.map(|e| SessionSnafu.into_error(e)) },
            Err(e) => return self.fail_with(&e),
        };
        if played.done > 0 {
            self.hub.emit(McpEvent::Changed);
            self.prune_surfaces().await;
            if let Some(e) = &played.refused {
                tracing::info!(error = %snafu::Report::from_error(e), "undo or redo stopped");
            }
        }
        match outcome(played, &self.localizer.current()) {
            Ok(v) => self.json_result(&v),
            Err(e) => self.fail_with(&e),
        }
    }

    /// `undo_turn`: refused for the call's own turn, which is not over.
    pub(crate) async fn undo_turn(&self, current: Option<u32>, args: Value) -> CallToolResult {
        let UndoTurnArgs { turn } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if current == Some(turn) {
            return self.fail_with(&UndoOpenTurnSnafu.build());
        }
        self.change(Request::UndoTurn { turn }).await
    }
}

#[cfg(test)]
#[path = "undo_tools_tests.rs"]
mod undo_tools_tests;
