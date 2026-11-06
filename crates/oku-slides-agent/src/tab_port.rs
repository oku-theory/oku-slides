//! The host's [`TabPort`]: an agent acts on another tab through the stack.
//!
//! It prefills another tab's empty draft, pages it, and starts an imported tab's fine-tuning
//! turn. It reaches the stack weakly: the stack's table holds the tabs, whose tools hold this
//! port. The app and `serve` share it.

use std::sync::Weak;

use oku_slides_mcp::{Prefill, TabPort, TuneRefusal};

use crate::Stack;

/// Acts on other tabs through [`Stack::prefill`], [`Stack::page`] and [`Stack::tune`].
#[derive(Debug)]
pub(crate) struct StackTabs {
    /// The stack, gone once the app closes.
    pub(crate) stack: Weak<Stack>,
}

impl TabPort for StackTabs {
    /// `Unavailable` once the stack is gone or for an unknown or closed tab.
    ///
    /// Otherwise `Set` when the draft took the text, `Kept` when it held one already.
    fn prefill(&self, tab: &str, text: String) -> Prefill {
        self.stack.upgrade().map_or(Prefill::Unavailable, |stack| stack.prefill(tab, text))
    }

    /// False once the stack is gone or for an unknown or closed tab.
    fn page(&self, tab: &str, slide: String) -> bool {
        self.stack.upgrade().is_some_and(|stack| stack.page(tab, slide))
    }

    /// `Unavailable` once the stack is gone; `NotStarted` when the stack started no turn.
    ///
    /// An unusable agent is also told in the tab itself.
    fn tune(&self, tab: &str) -> Result<(), TuneRefusal> {
        let stack = self.stack.upgrade().ok_or_else(TuneRefusal::unavailable)?;
        if stack.tune(tab) { Ok(()) } else { Err(TuneRefusal::not_started()) }
    }
}

#[cfg(test)]
#[path = "tab_port_tests.rs"]
mod tab_port_tests;
