//! Traces of a turn: the elements the agent's tools changed, resolved for its receipt.
//!
//! The agent's tools write through `handle_by_agent` and `apply_by_agent`, which record what
//! their applied ops touched (`turn_touches`); the user's writes, clicks on agent forms
//! included, go through `handle` and `apply` and are never traced.

use std::ops::{Deref, DerefMut};

use oku_slides_core::{LocalizedText, Op};

use super::turn_touches::{Touch, first_text, touches_of};
use crate::{Request, Result, Session};

/// The action name of a receipt's trace button.
pub const TRACE_EVENT: &str = "trace";
/// Traces a receipt shows as chips; the others are counted in one caption.
pub const TRACES_SHOWN: usize = 6;

/// The object a trace names, localized where it is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceName {
    /// The element's first text, plain, at most 40 characters.
    Text(String),
    /// No text: the element's kind, named through its schema label.
    Kind {
        /// Qualified widget of the element (`base/chart`).
        kind: String,
        /// Its schema's label, if it declares one.
        label: Option<LocalizedText>,
    },
}

/// One element the agent changed in a turn, ready for a receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// Slide that holds the element now.
    pub slide: String,
    /// Element id.
    pub element: String,
    /// What the chip calls it.
    pub name: TraceName,
    /// Ops that touched it.
    pub changes: u32,
    /// Gone from the deck when the trace was built.
    pub removed: bool,
    /// Its chart or table data changed: the click opens the data pane.
    pub data: bool,
}

/// The session during an agent call: ops applied meanwhile are recorded; dropped, it restores
/// the previous state, on every return path.
struct AgentWrites<'a> {
    session: &'a mut Session,
    before: bool,
}

impl<'a> AgentWrites<'a> {
    fn new(session: &'a mut Session) -> Self {
        let before = session.turn_touches.set_recording(true);
        Self { session, before }
    }
}

impl Deref for AgentWrites<'_> {
    type Target = Session;
    fn deref(&self) -> &Session {
        self.session
    }
}

impl DerefMut for AgentWrites<'_> {
    fn deref_mut(&mut self) -> &mut Session {
        self.session
    }
}

impl Drop for AgentWrites<'_> {
    fn drop(&mut self) {
        self.session.turn_touches.set_recording(self.before);
    }
}

impl Session {
    /// `handle` for an agent tool: deck ops applied by it are recorded for the open turn's receipt.
    ///
    /// # Errors
    ///
    /// Those of [`Session::handle`]; a refused request records nothing.
    pub fn handle_by_agent(&mut self, req: Request) -> Result<String> {
        AgentWrites::new(self).handle(req)
    }

    /// `apply` for an agent tool (hub-applied agent ops), recorded like `handle_by_agent`.
    ///
    /// # Errors
    ///
    /// Those of [`Session::apply`]; refused ops record nothing.
    pub fn apply_by_agent(&mut self, label: &str, ops: Vec<Op>, merge: Option<&str>) -> Result<()> {
        AgentWrites::new(self).apply(label, ops, merge)
    }

    /// Traces of `turn`, resolved against the deck now; empty for an unknown turn.
    #[must_use]
    pub fn traces(&self, turn: u32) -> Vec<Trace> {
        self.turn_touches.touches(turn).iter().map(|t| self.trace(t)).collect()
    }

    /// Traces of the open turn; none outside a turn.
    pub(crate) fn open_traces(&self) -> Vec<Trace> {
        self.turn_touches.open().map(|turn| self.traces(turn)).unwrap_or_default()
    }

    /// The touches of a batch about to be applied to the deck.
    pub(crate) fn touches_of_batch(&self, ops: &[Op]) -> Vec<Touch> {
        touches_of(self.doc.sources.deck(), &self.registry, ops)
    }

    /// `t` as the deck shows it now: gone (its slide or a parent removed too), or named.
    fn trace(&self, t: &Touch) -> Trace {
        let deck = self.doc.sources.deck();
        let found = deck.slide(&t.slide).and_then(|s| s.locate(&t.element)).filter(|_| !t.removed);
        let (kind, text) = match found {
            Some(l) => {
                let text = self.registry.get(&l.element.kind).and_then(|w| first_text(w, &l.element.params));
                (l.element.kind.as_str(), text)
            }
            // Gone: named as it was when the agent touched it.
            None => (t.kind.as_str(), t.text.clone()),
        };
        let name = text.map_or_else(
            || TraceName::Kind { kind: kind.to_owned(), label: self.registry.get(kind).and_then(|w| w.label.clone()) },
            TraceName::Text,
        );
        Trace {
            slide: t.slide.clone(),
            element: t.element.clone(),
            name,
            changes: t.changes,
            removed: found.is_none(),
            data: t.data,
        }
    }
}

#[cfg(test)]
#[path = "traces_tests.rs"]
mod traces_tests;
