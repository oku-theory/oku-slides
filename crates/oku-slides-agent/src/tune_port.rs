//! The host's [`TunePort`]: an agent's import starts the new tab's fine-tuning turn.
//!
//! It reaches the stack weakly (as [`crate::drafts_port::StackDrafts`] does): the stack's
//! table holds the tabs, whose tools hold this port. The app and `serve` share it.

use std::sync::Weak;

use oku_slides_mcp::{TunePort, TuneRefusal};

use crate::Stack;

/// Starts an imported tab's fine-tuning turn through [`Stack::tune`].
#[derive(Debug)]
pub(crate) struct StackTune {
    /// The stack, gone once the app closes.
    pub(crate) stack: Weak<Stack>,
}

impl TunePort for StackTune {
    /// `Unavailable` once the stack is gone; `NotStarted` when the stack started no turn.
    ///
    /// An unusable agent is also told in the tab itself.
    fn tune(&self, tab: &str) -> Result<(), TuneRefusal> {
        let stack = self.stack.upgrade().ok_or_else(TuneRefusal::unavailable)?;
        if stack.tune(tab) { Ok(()) } else { Err(TuneRefusal::not_started()) }
    }
}

#[cfg(test)]
#[path = "tune_port_tests.rs"]
mod tune_port_tests;
