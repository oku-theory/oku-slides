//! Data refs set by option edits: checked after the op, against the views it found.
//!
//! An op carrying chart or table options (`set_param`, `set_chart_option`,
//! `set_table_option`, `replace_element`, `add_element`, `add_slide`,
//! `edit_list`, `set_kind`) may point a view at a data. After it applies, each
//! view of the slide it changed must name an existing, whole data and columns
//! it has; a view that lost its ref gets it back (no unbinding); a view moved
//! to another data is refused (use `bind_data`). Inline data never get here: the
//! op's value check refuses them (`host.rs`). On refusal the slide is restored.

use std::collections::BTreeMap;

use oku_slides_chart::{parse_option, parse_table, require_data_in_chart, require_data_in_table};
use oku_slides_data::DataError;
use serde_json::Value;
use snafu::ResultExt;

use super::Op;
use crate::error::{ChartSnafu, DataFault, OpError, TableSnafu};
use crate::model::{Deck, Slide};
use crate::params::{ParamValue, Params};
use crate::widget::{HostType, WidgetRegistry};

/// A chart or table param as found before or after the op.
struct View {
    host: HostType,
    value: Value,
}

impl View {
    fn data(&self) -> Option<&str> {
        self.value.get("data").and_then(Value::as_str)
    }
}

/// Views by (owner id, param): the slide id owns the slide's params.
type Views = BTreeMap<(String, String), View>;

/// The slide whose views `op` may change, if it carries options.
pub(super) fn scope(op: &Op) -> Option<&str> {
    match op {
        Op::SetParam { target, .. }
        | Op::SetChartOption { target, .. }
        | Op::SetTableOption { target, .. }
        | Op::EditList { target, .. }
        | Op::SetKind { target, .. } => Some(&target.slide),
        Op::ReplaceElement { slide, .. } => Some(slide),
        Op::AddElement { slot, .. } => Some(&slot.owner.slide),
        Op::AddSlide { slide, .. } => Some(&slide.id),
        Op::SetDeckParam { .. }
        | Op::SetStyle { .. }
        | Op::ReplaceStyle { .. }
        | Op::SetFrame { .. }
        | Op::SetCell { .. }
        | Op::SetGrid { .. }
        | Op::RemoveElement { .. }
        | Op::MoveElement { .. }
        | Op::RemoveSlide { .. }
        | Op::MoveSlide { .. }
        | Op::SetNotes { .. }
        | Op::SetTheme { .. }
        | Op::AddPack { .. }
        | Op::RemovePack { .. }
        | Op::SetToken { .. }
        | Op::UnsetToken { .. }
        | Op::SetTokens { .. }
        | Op::SetVar { .. }
        | Op::AddData { .. }
        | Op::EditData { .. }
        | Op::BindData { .. }
        | Op::RemoveData { .. }
        | Op::RenameData { .. }
        | Op::SetColumnType { .. }
        | Op::PutViewData { .. }
        | Op::KnowledgeWrite { .. }
        | Op::KnowledgeEdit { .. }
        | Op::KnowledgeMove { .. }
        | Op::KnowledgeDelete { .. } => None,
    }
}

/// The views of slide `id` before an op, and the slide to restore on refusal.
pub(super) struct Guard {
    slide: String,
    saved: Option<(usize, Slide)>,
    before: Views,
}

impl Guard {
    pub(super) fn take(deck: &Deck, registry: &WidgetRegistry, slide: &str) -> Guard {
        let saved = deck.slide_index(slide).map(|i| (i, deck.slides[i].clone()));
        let before = saved.as_ref().map(|(_, s)| views(registry, s)).unwrap_or_default();
        Guard { slide: slide.to_owned(), saved, before }
    }

    /// Checks the views the op changed; on refusal, puts the slide back.
    pub(super) fn check(self, deck: &mut Deck, registry: &WidgetRegistry) -> Result<(), OpError> {
        let Some(i) = deck.slide_index(&self.slide) else { return Ok(()) };
        let result = check_slide(deck, registry, i, &self.before);
        if result.is_err() {
            match self.saved {
                Some((at, slide)) => deck.slides[at] = slide,
                None => {
                    deck.slides.remove(i);
                }
            }
        }
        result
    }
}

fn check_slide(deck: &mut Deck, registry: &WidgetRegistry, i: usize, before: &Views) -> Result<(), OpError> {
    let after = views(registry, &deck.slides[i]);
    let mut restores = Vec::new();
    for (key, view) in &after {
        let prev = before.get(key);
        if prev.is_some_and(|p| p.value == view.value) {
            continue;
        }
        let old = prev.and_then(View::data);
        match (old, view.data()) {
            (Some(old), Some(new)) if old != new => return Err(DataFault::Rebind { subject: key.0.clone() }.into()),
            (Some(_), None) => {
                let Some(prev) = prev else { continue };
                restores.push((key.clone(), with_ref(&view.value, &prev.value)));
            }
            (_, Some(new)) => check_ref(deck, new, view)?,
            (None, None) => require_data(&key.0, view)?,
        }
    }
    for ((owner, param), restored) in restores {
        let slide = &mut deck.slides[i];
        let params = if owner == slide.id {
            &mut slide.params
        } else {
            let Some(e) = super::element_mut(slide, &owner) else { continue };
            &mut e.params
        };
        params.set(&param, ParamValue::Value(restored));
    }
    Ok(())
}

/// A ref to an existing, whole data, projected on columns it has.
fn check_ref(deck: &Deck, data: &str, view: &View) -> Result<(), OpError> {
    let Some((id, d)) = deck.data.get_key_value(data) else {
        return Err(DataFault::Missing { data: data.to_owned() }.into());
    };
    if deck.truncated.contains(id) {
        return Err(DataFault::Truncated { data: data.to_owned() }.into());
    }
    let columns = view.value.get("columns").and_then(Value::as_array).into_iter().flatten();
    if let Some(missing) = columns.filter_map(Value::as_str).find(|c| !d.table.columns.iter().any(|col| col.id == *c)) {
        return Err(DataError::no_column(missing).into());
    }
    Ok(())
}

/// Refuses a view without a ref, as the host would.
fn require_data(subject: &str, view: &View) -> Result<(), OpError> {
    match view.host {
        HostType::Chart => {
            let option = parse_option(&view.value).context(ChartSnafu { subject })?;
            Ok(require_data_in_chart(&option).context(ChartSnafu { subject })?)
        }
        HostType::Table => {
            let option = parse_table(&view.value).context(TableSnafu { subject })?;
            Ok(require_data_in_table(&option).context(TableSnafu { subject })?)
        }
        HostType::Path => Ok(()),
    }
}

/// `value` with the ref and projection of `prev` put back.
fn with_ref(value: &Value, prev: &Value) -> Value {
    let mut out = value.clone();
    if let Value::Object(option) = &mut out {
        for field in ["data", "columns"] {
            if let Some(v) = prev.get(field) {
                option.insert(field.to_owned(), v.clone());
            }
        }
    }
    out
}

/// The chart and table params of `slide` and its elements.
fn views(registry: &WidgetRegistry, slide: &Slide) -> Views {
    let mut out = Views::new();
    push(&mut out, registry, &slide.kind, &slide.id, &slide.params);
    for located in slide.walk() {
        let e = located.element;
        push(&mut out, registry, &e.kind, &e.id, &e.params);
    }
    out
}

fn push(out: &mut Views, registry: &WidgetRegistry, kind: &str, owner: &str, params: &Params) {
    let Some(schema) = registry.get(kind) else { return };
    for p in &schema.params {
        let Some(host @ (HostType::Chart | HostType::Table)) = p.ty.host() else { continue };
        if let Some(ParamValue::Value(v)) = params.get(&p.name) {
            out.insert((owner.to_owned(), p.name.clone()), View { host, value: v.clone() });
        }
    }
}
