//! Elements the agent's tools changed, per turn, for the receipt's traces.
//!
//! Only the agent's writes count: `Session::handle_by_agent` and `apply_by_agent` set
//! `recording` for their call; `apply_ops` computes the batch's touches before its ops run
//! and records them once it succeeded. A list per turn, the last `KEPT_TURNS` turns kept.

use std::collections::{HashMap, VecDeque};

use oku_slides_core::{
    Deck, Element, HostType, Op, ParamType, ParamValue, Params, Target, WidgetRegistry, WidgetSchema,
};

use super::KEPT_TURNS;
use super::bounds::clip;

/// Longest name of a trace taken from an element's text, ellipsis included.
const NAME_CHARS: usize = 40;

/// One element an agent op touched, or several merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Touch {
    /// Slide that holds the element after the op.
    pub slide: String,
    /// Element id.
    pub element: String,
    /// Its widget when touched: the name of an element gone since.
    pub kind: String,
    /// Its first text when touched (`first_text`): the name of an element gone since.
    pub text: Option<String>,
    /// The slide a move to another slide took it from: its earlier touch there follows it.
    pub from: Option<String>,
    /// Ops that touched it.
    pub changes: u32,
    /// The last op removed it.
    pub removed: bool,
    /// An op changed its chart or table data.
    pub data: bool,
}

/// A turn's touches, in first-touch order.
#[derive(Debug, Default)]
struct Touches {
    list: Vec<Touch>,
    /// `(slide, element)` → its rank in `list`.
    index: HashMap<(String, String), usize>,
}

/// Adds `t` to `seen`, the same element touched earlier.
fn merge(seen: &mut Touch, t: Touch) {
    seen.changes = seen.changes.saturating_add(t.changes);
    seen.data |= t.data;
    // An op on an element that succeeded saw it there: only the last one says if it is gone.
    seen.removed = t.removed;
    seen.kind = t.kind;
    if t.text.is_some() {
        seen.text = t.text;
    }
}

impl Touches {
    fn record(&mut self, t: Touch) {
        if let Some(from) = t.from.as_deref().filter(|f| *f != t.slide) {
            self.follow(from, &t.slide, &t.element);
        }
        let key = (t.slide.clone(), t.element.clone());
        if let Some(seen) = self.index.get(&key).and_then(|&i| self.list.get_mut(i)) {
            merge(seen, t);
        } else {
            self.index.insert(key, self.list.len());
            self.list.push(t);
        }
    }

    /// `element` moved from slide `from` to `to`: its touch there is now keyed on `to`, at its
    /// first-touch rank, merged with a touch already on `to` (the earlier rank kept).
    fn follow(&mut self, from: &str, to: &str, element: &str) {
        let Some(old) = self.index.remove(&(from.to_owned(), element.to_owned())) else { return };
        let new = (to.to_owned(), element.to_owned());
        match self.index.get(&new).copied() {
            None => {
                if let Some(t) = self.list.get_mut(old) {
                    to.clone_into(&mut t.slide);
                }
                self.index.insert(new, old);
            }
            Some(there) => {
                let (keep, gone) = if old < there { (old, there) } else { (there, old) };
                let merged = self.list.remove(gone);
                if let Some(kept) = self.list.get_mut(keep) {
                    to.clone_into(&mut kept.slide);
                    merge(kept, Touch { removed: false, ..merged });
                }
                self.index =
                    self.list.iter().enumerate().map(|(i, t)| ((t.slide.clone(), t.element.clone()), i)).collect();
            }
        }
    }
}

/// The elements the agent changed in its last turns.
#[derive(Debug, Default)]
pub(crate) struct TurnTouches {
    turns: VecDeque<(u32, Touches)>,
    /// Inside a `*_by_agent` call: the ops applied are the agent's.
    recording: bool,
    /// The open turn, which records.
    open: Option<u32>,
}

impl TurnTouches {
    /// Turn `turn` opens, with an empty list; the oldest beyond `KEPT_TURNS` go.
    pub(crate) fn begin(&mut self, turn: u32) {
        self.turns.retain(|(t, _)| *t != turn);
        self.turns.push_back((turn, Touches::default()));
        while self.turns.len() > KEPT_TURNS {
            self.turns.pop_front();
        }
        self.open = Some(turn);
    }

    /// The open turn closes; returns it.
    pub(crate) fn end(&mut self) -> Option<u32> {
        self.open.take()
    }

    /// Another document: the lists no longer name its elements; an open turn starts over empty.
    pub(crate) fn clear(&mut self) {
        self.turns.clear();
        if let Some(turn) = self.open {
            self.turns.push_back((turn, Touches::default()));
        }
    }

    /// Ops applied now are the agent's (`true`) or not; returns the previous state.
    pub(crate) fn set_recording(&mut self, on: bool) -> bool {
        std::mem::replace(&mut self.recording, on)
    }

    /// Inside an agent call: the ops applied now are recorded.
    pub(crate) fn recording(&self) -> bool {
        self.recording
    }

    /// Records an applied batch's touches in the open turn; none open: nothing.
    pub(crate) fn record_all(&mut self, touches: Vec<Touch>) {
        let Some(open) = self.open else { return };
        let Some((_, list)) = self.turns.iter_mut().rev().find(|(t, _)| *t == open) else { return };
        for t in touches {
            list.record(t);
        }
    }

    /// The touches of `turn`, in first-touch order; none for an unknown turn.
    pub(crate) fn touches(&self, turn: u32) -> &[Touch] {
        self.turns.iter().find(|(t, _)| *t == turn).map_or(&[], |(_, list)| list.list.as_slice())
    }

    /// The open turn, if any.
    pub(crate) fn open(&self) -> Option<u32> {
        self.open
    }
}

/// The plain text of a text parameter: runs joined, paragraphs and blanks folded to one space.
fn plain_text(value: &ParamValue) -> Option<String> {
    let text = match value {
        ParamValue::Text(t) => t.clone(),
        ParamValue::Rich(paras) => {
            paras.iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect::<String>()).collect::<Vec<_>>().join(" ")
        }
        ParamValue::Slot(_) | ParamValue::List(_) | ParamValue::Value(_) => return None,
    };
    let folded = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!folded.is_empty()).then(|| clip(&folded, NAME_CHARS))
}

/// The element's first non-empty text parameter, in schema order, plain, at most 40 characters.
pub(super) fn first_text(schema: &WidgetSchema, params: &Params) -> Option<String> {
    schema
        .params
        .iter()
        .filter(|p| matches!(p.ty, ParamType::Plain | ParamType::Rich))
        .find_map(|p| params.get(&p.name).and_then(plain_text))
}

/// What names an element: its widget and its first text.
type Named = (String, Option<String>);

/// The batch's elements: those of `deck` before it, overridden by its earlier ops.
struct Known<'a> {
    deck: &'a Deck,
    registry: &'a WidgetRegistry,
    added: HashMap<(String, String), Named>,
}

impl Known<'_> {
    fn named(&self, element: &Element) -> Named {
        let text = self.registry.get(&element.kind).and_then(|w| first_text(w, &element.params));
        (element.kind.clone(), text)
    }

    fn of(&self, slide: &str, element: &str) -> Option<Named> {
        match self.added.get(&(slide.to_owned(), element.to_owned())) {
            Some(named) => Some(named.clone()),
            None => Some(self.named(self.deck.slide(slide)?.locate(element)?.element)),
        }
    }

    /// `element` and the elements of its slots now stand on `slide`.
    fn add(&mut self, slide: &str, element: &Element) {
        let named = self.named(element);
        self.added.insert((slide.to_owned(), element.id.clone()), named);
        for (_, children) in element.params.slots() {
            for child in children {
                self.add(slide, child);
            }
        }
    }
}

/// Is `kind` a chart or a table (a `host(chart)` or `host(table)` parameter in its schema)?
fn holds_data(registry: &WidgetRegistry, kind: &str) -> bool {
    registry
        .get(kind)
        .is_some_and(|w| w.host_param(HostType::Chart).is_some() || w.host_param(HostType::Table).is_some())
}

/// The touches of `ops`, applied in order to `deck`: one per op on an element, none for a slide
/// or deck op. An element the batch cannot name a widget for makes core refuse the batch.
pub(crate) fn touches_of(deck: &Deck, registry: &WidgetRegistry, ops: &[Op]) -> Vec<Touch> {
    let mut kinds = Known { deck, registry, added: HashMap::new() };
    let mut out = Vec::new();
    for op in ops {
        let (slide, element, data, removed, from) = match op {
            Op::SetParam { target, .. } | Op::EditList { target, .. } | Op::SetKind { target, .. } => {
                let Some(element) = &target.element else { continue };
                (&target.slide, element, false, false, None)
            }
            Op::SetData { target, .. } | Op::SetChartOption { target, .. } | Op::SetTableOption { target, .. } => {
                let Target { slide, element: Some(element) } = target else { continue };
                (slide, element, true, false, None)
            }
            Op::SetStyle { slide, element, .. }
            | Op::ReplaceStyle { slide, element, .. }
            | Op::SetFrame { slide, element, .. }
            | Op::SetCell { slide, element, .. } => (slide, element, false, false, None),
            Op::ReplaceElement { slide, element } => {
                kinds.add(slide, element);
                (slide, &element.id, holds_data(registry, &element.kind), false, None)
            }
            Op::AddElement { slot, element, .. } => {
                kinds.add(&slot.owner.slide, element);
                (&slot.owner.slide, &element.id, false, false, None)
            }
            Op::RemoveElement { slide, element } => (slide, element, false, true, None),
            Op::MoveElement { slide, element, to, .. } => {
                if let Some(named) = kinds.of(slide, element) {
                    kinds.added.insert((to.owner.slide.clone(), element.clone()), named);
                }
                (&to.owner.slide, element, false, false, Some(slide.clone()))
            }
            Op::SetDeckParam { .. }
            | Op::SetGrid { .. }
            | Op::AddSlide { .. }
            | Op::RemoveSlide { .. }
            | Op::MoveSlide { .. }
            | Op::SetNotes { .. }
            | Op::SetTheme { .. }
            | Op::AddPack { .. }
            | Op::RemovePack { .. }
            | Op::SetToken { .. }
            | Op::UnsetToken { .. }
            | Op::SetTokens { .. }
            | Op::SetVar { .. } => continue,
        };
        if let Op::SetKind { kind, .. } = op
            && let Some((_, text)) = kinds.of(slide, element)
        {
            kinds.added.insert((slide.clone(), element.clone()), (kind.clone(), text));
        }
        let Some((kind, text)) = kinds.of(slide, element) else { continue };
        let (slide, element) = (slide.clone(), element.clone());
        out.push(Touch { slide, element, kind, text, from, changes: 1, removed, data });
    }
    out
}

#[cfg(test)]
#[path = "turn_touches_tests.rs"]
mod turn_touches_tests;
