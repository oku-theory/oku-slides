//! Answers to an open question from a microphone utterance the host routed.
//!
//! The target is the hub's own, with the page's `quickTarget` priority: modal, then the sheet
//! staged on the shown slide, then dock question, then the variant drawer. The answer is
//! decided under the state lock; the answer is sent and variant ops applied after it is
//! released. Utterance text is never logged.

use std::time::{Duration, Instant};

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::bounds::label;
use oku_slides_session::agent::{FocusSource, PermissionKind, Place, ShowVariants};
use serde_json::{Map, Value, json};

use super::asks::{AskKind, PendingAsk};
use super::{Answer, HubState, Shown, SurfaceHub};
use crate::{HeardUtterance, Lexicon, YesNo};

/// How long a question waits, once shown, before speech may answer it (the modal's guard).
pub const ARMING: Duration = Duration::from_millis(500);

/// What a reply did to the open question; one enum for typed and voice replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// It answered the question, or picked a variant.
    Answered,
    /// The question stays open, and says why when it can.
    KeptOpen,
    /// No question takes it: the reply is an ordinary prompt.
    NotAQuestion,
}

impl SurfaceHub {
    /// The read-aloud of question `id` ended `at`: its arming starts then, never earlier.
    ///
    /// A question already answered or an earlier `at` changes nothing.
    pub fn spoken_until(&self, id: &str, at: Instant) {
        if let Some(ask) = self.state().asks.iter_mut().find(|a| a.id == id) {
            ask.armed_from = ask.armed_from.max(at);
        }
    }
}

/// True when speech began at least [`ARMING`] after the question became answerable.
pub(crate) fn armed(armed_from: Instant, started: Instant) -> bool {
    armed_from.checked_add(ARMING).is_some_and(|at| started >= at)
}

/// An utterance to answer with, and what reads it.
struct Spoken<'a> {
    heard: &'a HeardUtterance,
    lexicon: &'a Lexicon,
    /// The host says the tab's window has the focus: enough to say yes to a guard.
    host: bool,
}

/// What the hub does once the state lock is released.
enum Act {
    /// Send this answer to the question taken from the list.
    Send(PendingAsk, Answer),
    /// Apply variant `index` of the drawer `id`, taken from the shown surfaces.
    Variant { id: String, spec: ShowVariants, index: usize },
    /// Nothing more: the outcome is already known.
    Done(AnswerOutcome),
}

/// The index of the question a spoken answer goes to: modal, staged sheet, then dock question.
///
/// The first modal (the page shows the first), then the first sheet anchored on the `staged`
/// slide (the one the page stages), then the last of the thread questions and the other
/// sheets, which the page lists in the thread.
fn target(asks: &[PendingAsk], staged: Option<&str>) -> Option<usize> {
    let on_stage = |a: &PendingAsk| a.place == Place::Sheet && staged.is_some() && a.slide.as_deref() == staged;
    asks.iter()
        .position(|a| a.place == Place::Modal)
        .or_else(|| asks.iter().position(on_stage))
        .or_else(|| asks.iter().rposition(|a| matches!(a.place, Place::Sheet | Place::Thread)))
}

/// An action answer with one value in its context.
fn action(name: &str, key: &str, value: Value) -> Answer {
    let mut context = Map::new();
    context.insert(key.into(), value);
    Answer::Action { name: name.into(), context }
}

impl SurfaceHub {
    /// Answers the open question of this tab from a microphone utterance.
    ///
    /// `focus` is who says the tab's window has the focus, `None` when it has not: a guarded
    /// confirmation or a permission takes a yes only with [`FocusSource::Host`]. `staged` is the
    /// slide the page last reported: its sheet is the one the page stages. The question
    /// is looked up and answered under one lock, so a question that closes in between cannot
    /// take an utterance meant for the next one. `NotAQuestion`: no question and no drawer took
    /// it, the caller sends it as a prompt.
    pub fn answer_voice(
        &self,
        heard: &HeardUtterance,
        lexicon: &Lexicon,
        focus: Option<FocusSource>,
        staged: Option<&str>,
    ) -> AnswerOutcome {
        let l = self.localizer.current();
        let spoken = Spoken { heard, lexicon, host: focus == Some(FocusSource::Host) };
        let act = {
            let mut state = self.state();
            match target(&state.asks, staged) {
                Some(i) => self.decide_ask(&mut state, i, &spoken, &l),
                None => decide_variant(&mut state, &heard.text, lexicon),
            }
        };
        match act {
            Act::Send(ask, answer) => {
                tracing::debug!(seq = heard.seq, "voice: question answered");
                match ask.tx.send(answer) {
                    Ok(()) => AnswerOutcome::Answered,
                    // A guard or a permission gone in between: the answer was for it, not a prompt.
                    Err(_) if !matches!(ask.kind, AskKind::Agent) => AnswerOutcome::Answered,
                    Err(_) => AnswerOutcome::NotAQuestion,
                }
            }
            Act::Variant { id, spec, index } => self.pick_variant(id, spec, index, heard.seq),
            Act::Done(outcome) => outcome,
        }
    }

    /// Decides the answer to question `i` under the state lock: taken and answered, or kept open.
    fn decide_ask(&self, state: &mut HubState, i: usize, spoken: &Spoken<'_>, l: &Localizer) -> Act {
        let Spoken { heard, lexicon, host } = *spoken;
        let Some(ask) = state.asks.get(i) else { return Act::Done(AnswerOutcome::NotAQuestion) };
        let (is_armed, kind, place) = (armed(ask.armed_from, heard.started), ask.kind.clone(), ask.place);
        let predates = heard.started < ask.armed_from;
        let keep = |state: &mut HubState, key: &str| {
            let line = match key {
                "agent-surface-heard" => l.tr_with(key, &[("text", label(&heard.text).into())]),
                other => l.tr(other),
            };
            self.show_line(state, i, &line, l);
            Act::Done(AnswerOutcome::KeptOpen)
        };
        match kind {
            // The agent's own question takes the text, said after it showed; a modal one only once
            // armed. Speech begun before the question is the next instruction.
            AskKind::Agent if predates => {
                tracing::debug!(seq = heard.seq, "voice: utterance predates the question");
                Act::Done(AnswerOutcome::NotAQuestion)
            }
            AskKind::Agent if place == Place::Modal && !is_armed => {
                tracing::debug!(seq = heard.seq, "voice: modal question not armed yet");
                Act::Done(AnswerOutcome::KeptOpen)
            }
            AskKind::Agent => Act::Send(state.take_ask(i), Answer::text(&heard.text)),
            _ if !is_armed => keep(state, "agent-surface-said-too-early"),
            AskKind::Confirm { verb, .. } => match lexicon.yes_no(&heard.text, verb) {
                YesNo::Yes if host => Act::Send(state.take_ask(i), action("confirm", "confirmed", json!(true))),
                YesNo::Yes => keep(state, "agent-surface-click-to-confirm"),
                YesNo::No => Act::Send(state.take_ask(i), action("confirm", "confirmed", json!(false))),
                YesNo::Unclear => keep(state, "agent-surface-heard"),
            },
            AskKind::Permission { options, .. } => {
                let of = |kind: PermissionKind| options.iter().position(|o| o.kind == kind);
                match lexicon.yes_no(&heard.text, None) {
                    YesNo::Yes if !host => keep(state, "agent-surface-click-to-confirm"),
                    YesNo::Yes => match of(PermissionKind::AllowOnce) {
                        Some(j) => Act::Send(state.take_ask(i), action("permission", "index", json!(j))),
                        None => keep(state, "agent-surface-heard"),
                    },
                    // No reject-once option: the refusal still answers, as Escape would.
                    YesNo::No => match of(PermissionKind::RejectOnce) {
                        Some(j) => Act::Send(state.take_ask(i), action("permission", "index", json!(j))),
                        None => Act::Send(state.take_ask(i), Answer::Cancelled),
                    },
                    YesNo::Unclear => keep(state, "agent-surface-heard"),
                }
            }
        }
    }

    /// Applies a variant picked by voice, after the state lock: one undo step, drawer closed.
    ///
    /// A failure puts the drawer back and keeps it open, as a click would, unless another drawer
    /// replaced it meanwhile: then the stale one is closed.
    fn pick_variant(&self, id: String, spec: ShowVariants, index: usize, seq: u64) -> AnswerOutcome {
        match self.apply_variant_at(&spec, index) {
            Ok(()) => {
                tracing::debug!(seq, "voice: variant picked");
                self.emit(super::McpEvent::CloseSurface { id });
                self.prune();
                AnswerOutcome::Answered
            }
            Err(e) => {
                tracing::info!(seq, error = %snafu::Report::from_error(&e), "voice: variant not applied");
                let mut state = self.state();
                if state.shows.contains_key(&id) {
                    // Shown again under its id meanwhile: that one stays.
                } else if state.shows.values().any(|s| matches!(s, Shown::Variants(_))) {
                    self.record_and_send(&mut state, super::McpEvent::CloseSurface { id });
                } else {
                    state.shows.insert(id, Shown::Variants(spec));
                }
                AnswerOutcome::KeptOpen
            }
        }
    }
}

/// The variant an utterance picks in the shown drawer, by ordinal or one label, taken out.
///
/// The utterance must be the ordinal or the label itself. No drawer, or nothing picked (a
/// prompt that only mentions a label): `NotAQuestion`, the utterance is a prompt.
fn decide_variant(state: &mut HubState, said: &str, lexicon: &Lexicon) -> Act {
    let drawer = state.shows.iter().find_map(|(id, s)| match s {
        Shown::Variants(spec) => Some((id.clone(), spec)),
        _ => None,
    });
    let Some((id, spec)) = drawer else { return Act::Done(AnswerOutcome::NotAQuestion) };
    let labels: Vec<&str> = spec.variants.iter().map(|v| v.label.as_str()).collect();
    let Some(index) = lexicon.ordinal(said, labels.len()).or_else(|| lexicon.label(said, &labels)) else {
        return Act::Done(AnswerOutcome::NotAQuestion);
    };
    match state.shows.remove(&id) {
        Some(Shown::Variants(spec)) => Act::Variant { id, spec, index },
        Some(other) => {
            state.shows.insert(id, other);
            Act::Done(AnswerOutcome::NotAQuestion)
        }
        None => Act::Done(AnswerOutcome::NotAQuestion),
    }
}

#[cfg(test)]
#[path = "voice_answer_tests.rs"]
mod voice_answer_tests;
