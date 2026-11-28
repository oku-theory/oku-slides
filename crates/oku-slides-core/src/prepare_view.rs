//! Inline view data lifted into sealed `PutViewData` ops.
//!
//! A chart or table the batch writes with its own data (a `dataset`, inline series or axis
//! data) is stripped of it and names a data instead; a sealed `PutViewData`, put right before
//! the op, stores the table. Charts are first converted to a `dataset` in the user's language.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use oku_slides_chart::{ChartLabels, ChartOption, canonicalize, check_ref_null};
use oku_slides_data::{DataId, Dataset, GivenIds, IdGen, NewTable, default_name};
use oku_slides_i18n::Message;
use serde_json::{Map, Value};

use snafu::ResultExt;

use crate::error::{ChartSnafu, OpError, TableSnafu};
use crate::model::Deck;
use crate::ops::{Op, Seal, Target, target_params};
use crate::params::{ParamValue, Params};
use crate::validate::Issue;
use crate::widget::{HostType, WidgetRegistry};

/// What lifting reads and what it draws, for one op of a batch.
///
/// Everything is borrowed from the batch, so lifting each op costs no copy of its state.
pub(crate) struct Lifter<'a> {
    pub(crate) registry: &'a WidgetRegistry,
    pub(crate) labels: &'a ChartLabels,
    /// Data ids the batch gives.
    pub(crate) given: &'a BTreeSet<String>,
    /// Data ids drawn earlier in the batch.
    pub(crate) drawn: &'a BTreeSet<String>,
    /// Row and column ids the batch gives, per data.
    pub(crate) given_tables: &'a BTreeMap<DataId, GivenIds>,
    /// The names of the scratch deck's data and those drawn in the batch, lowercase.
    pub(crate) names: &'a mut BTreeSet<String>,
    /// Per name text, the first default-name suffix not known to be taken.
    pub(crate) suffixes: &'a mut HashMap<String, usize>,
    pub(crate) warnings: &'a mut Vec<Issue>,
}

impl Lifter<'_> {
    /// Strips the inline data `op` writes; returns the `PutViewData` ops to put before it.
    pub(crate) fn lift(&mut self, op: &mut Op, scratch: &Deck, ids: &mut IdGen<'_>) -> Result<Vec<Op>, OpError> {
        merge_patch_as_param(op, scratch)?;
        let mut puts = Vec::new();
        match op {
            Op::SetParam { target, param, value } => {
                if let Some(host) = host_of(self.registry, scratch, target, param) {
                    self.site(scratch, ids, &mut puts, target, param, host, value)?;
                }
            }
            Op::ReplaceElement { slide, element } => {
                self.tree(scratch, ids, &mut puts, slide, Some(&element.id), &element.kind, &mut element.params)?;
            }
            Op::AddElement { slot, element, .. } => {
                let slide = slot.owner.slide.clone();
                self.tree(scratch, ids, &mut puts, &slide, Some(&element.id), &element.kind, &mut element.params)?;
            }
            Op::AddSlide { slide, .. } => {
                let id = slide.id.clone();
                self.tree(scratch, ids, &mut puts, &id, None, &slide.kind, &mut slide.params)?;
            }
            _ => {}
        }
        Ok(puts)
    }

    /// The host params of an element or slide, then its slot children, in tree order.
    #[expect(clippy::too_many_arguments, reason = "the walk state of one tree")]
    fn tree(
        &mut self,
        scratch: &Deck,
        ids: &mut IdGen<'_>,
        puts: &mut Vec<Op>,
        slide: &str,
        element: Option<&str>,
        kind: &str,
        params: &mut Params,
    ) -> Result<(), OpError> {
        let target = Target { slide: slide.to_owned(), element: element.map(str::to_owned) };
        let hosts: Vec<(String, HostType)> = self.registry.get(kind).map_or_else(Vec::new, |schema| {
            schema.params.iter().filter_map(|p| p.ty.host().map(|h| (p.name.clone(), h))).collect()
        });
        for (param, host) in hosts {
            let Some(ParamValue::Value(v)) = params.get(&param) else { continue };
            let mut value = v.clone();
            if self.site(scratch, ids, puts, &target, &param, host, &mut value)? {
                params.set(param, ParamValue::Value(value));
            }
        }
        for (_, children) in params.slots_mut() {
            for child in children {
                self.tree(scratch, ids, puts, slide, Some(&child.id), &child.kind, &mut child.params)?;
            }
        }
        Ok(())
    }

    /// Lifts the inline data of one view value; whether it changed.
    #[expect(clippy::too_many_arguments, reason = "the walk state of one view")]
    fn site(
        &mut self,
        scratch: &Deck,
        ids: &mut IdGen<'_>,
        puts: &mut Vec<Op>,
        target: &Target,
        param: &str,
        host: HostType,
        value: &mut Value,
    ) -> Result<bool, OpError> {
        let Value::Object(option) = value else { return Ok(false) };
        // A ref next to inline data is refused by the apply, with the reason.
        if option.contains_key("data") {
            return Ok(false);
        }
        let (dataset, title, chart) = match host {
            HostType::Chart => {
                let Ok(parsed) = serde_json::from_value::<ChartOption>(Value::Object(option.clone())) else {
                    return Ok(false);
                };
                if !chart_has_inline_data(&parsed) {
                    return Ok(false);
                }
                // The apply reports an option it cannot convert.
                let Ok(mut canonical) = canonicalize(&parsed, self.labels) else { return Ok(false) };
                let Some(dataset) = canonical.dataset.take() else { return Ok(false) };
                let title = canonical.title.as_ref().and_then(|t| t.text.clone());
                (dataset, title, Some(canonical))
            }
            HostType::Table => {
                let Some(raw) = option.get("dataset").filter(|d| !d.is_null()) else { return Ok(false) };
                let Ok(dataset) = serde_json::from_value::<Dataset>(raw.clone()) else { return Ok(false) };
                (dataset, None, None)
            }
            HostType::Path => return Ok(false),
        };
        let owner = target.element.as_deref().unwrap_or(&target.slide);
        let lifted = oku_slides_data::NewTable::from_dataset(&dataset);
        let mut table = lifted.table;
        let own = own_inline(scratch, target, param, owner);
        let keep = own.as_ref().and_then(|id| scratch.data.get(id)).and_then(|d| d.table());
        let in_batch = own.as_ref().and_then(|id| self.given_tables.get(id));
        table.fill_ids(keep, in_batch.unwrap_or(&GivenIds::default()), ids);
        let id = if let Some(id) = own {
            id
        } else {
            let (given, drawn) = (self.given, self.drawn);
            let fresh = ids(&|c: &str| given.contains(c) || drawn.contains(c) || scratch.data.contains_key(c));
            DataId::try_from(fresh)?
        };
        let text = title.as_deref().unwrap_or(param);
        let first = self.suffixes.get(text).copied().unwrap_or(2);
        let names = &*self.names;
        let (name, suffix) = default_name(text, first, |n| names.contains(&n.to_lowercase()));
        self.suffixes.insert(text.to_owned(), suffix + 1);
        self.names.insert(name.to_lowercase());
        self.warn(target, "data-non-finite", lifted.non_finite);
        self.warn(target, "data-row-cut", lifted.cut_rows);
        *value = stripped(option, chart, &id, &table)?;
        let put = Op::PutViewData {
            target: target.clone(),
            param: param.to_owned(),
            id,
            name,
            label: title,
            table,
            seal: Seal::sealed(),
        };
        puts.push(put);
        Ok(true)
    }

    fn warn(&mut self, target: &Target, key: &'static str, count: usize) {
        if count > 0 {
            self.warnings.push(Issue {
                slide: Some(target.slide.clone()),
                element: target.element.clone(),
                message: Message::new(key).arg("count", count),
            });
        }
    }
}

/// The view's own inline data, which a new table replaces in place.
fn own_inline(scratch: &Deck, target: &Target, param: &str, owner: &str) -> Option<DataId> {
    let (_, params) = target_params(scratch, target)?;
    let bound = params.value(param)?.get("data")?.as_str()?;
    let (id, data) = scratch.data.get_key_value(bound)?;
    let own = data.inline && scratch.data_users(id).iter().all(|v| v.element == owner && v.param == param);
    own.then(|| id.clone())
}

/// The view without its inline data, naming `id`; a chart reads the columns it uses.
fn stripped(
    option: &Map<String, Value>,
    chart: Option<ChartOption>,
    id: &DataId,
    table: &NewTable,
) -> Result<Value, OpError> {
    let ids = |n: usize| -> Vec<Value> {
        table.columns.iter().take(n).filter_map(|c| c.id.clone()).map(Value::String).collect()
    };
    let mut out = if let Some(chart) = chart {
        let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
        let used = chart.used_columns(&names);
        let Ok(Value::Object(mut out)) = serde_json::to_value(&chart) else {
            return Err(crate::error::DataFault::Internal.into());
        };
        if used > 0 {
            out.insert("columns".to_owned(), Value::Array(ids(used)));
        }
        out
    } else {
        let mut out = option.clone();
        out.remove("dataset");
        out.remove("columns");
        out
    };
    out.insert("data".to_owned(), Value::String(id.as_str().to_owned()));
    Ok(Value::Object(out))
}

fn chart_has_inline_data(option: &ChartOption) -> bool {
    option.dataset.is_some()
        || option.series.iter().any(|s| s.data.is_some())
        || [&option.x_axis, &option.y_axis].into_iter().flatten().any(|a| a.data.is_some())
}

/// The host type of a param of an existing target.
fn host_of(registry: &WidgetRegistry, deck: &Deck, target: &Target, param: &str) -> Option<HostType> {
    let (kind, _) = target_params(deck, target)?;
    registry.get(kind)?.params.iter().find(|p| p.name == param)?.ty.host()
}

/// A merge patch bringing inline data becomes a whole value, merged onto the stored one.
///
/// The new data replaces the view's ref; `"columns": null` drops the projection.
///
/// # Errors
///
/// `chart-ref-null` for a patch writing `"data": null`, which the merge would drop.
fn merge_patch_as_param(op: &mut Op, scratch: &Deck) -> Result<(), OpError> {
    let (host, target, param, patch) = match op {
        Op::SetChartOption { target, param, patch } => (HostType::Chart, target, param, patch),
        Op::SetTableOption { target, param, patch } => (HostType::Table, target, param, patch),
        _ => return Ok(()),
    };
    if !brings_inline(patch) {
        return Ok(());
    }
    let subject = target.element.clone().unwrap_or_else(|| param.clone());
    if host == HostType::Table {
        check_ref_null(patch).context(TableSnafu { subject })?;
    } else {
        check_ref_null(patch).context(ChartSnafu { subject })?;
    }
    let Some((_, params)) = target_params(scratch, target) else { return Ok(()) };
    let mut merged = params.value(param).cloned().unwrap_or_else(|| Value::Object(Map::new()));
    if let Value::Object(o) = &mut merged {
        o.remove("data");
        o.remove("columns");
        o.remove("dataset");
    }
    json_patch::merge(&mut merged, patch);
    *op = Op::SetParam { target: target.clone(), param: param.clone(), value: merged };
    Ok(())
}

/// Do raw options, or a merge patch, carry inline data: a `dataset`, a series' or an axis' `data`?
pub fn brings_inline(patch: &Value) -> bool {
    let has_data = |v: Option<&Value>| v.is_some_and(|v| v.get("data").is_some_and(|d| !d.is_null()));
    patch.get("dataset").is_some_and(|d| !d.is_null())
        || patch.get("series").and_then(Value::as_array).is_some_and(|s| s.iter().any(|s| has_data(Some(s))))
        || has_data(patch.get("xAxis"))
        || has_data(patch.get("yAxis"))
}

/// Removes the inline data [`brings_inline`] finds: the `dataset`, series' and axes' `data`.
pub fn strip_inline(option: &mut Value) {
    let Value::Object(o) = option else { return };
    o.remove("dataset");
    for series in o.get_mut("series").and_then(Value::as_array_mut).into_iter().flatten() {
        if let Value::Object(s) = series {
            s.remove("data");
        }
    }
    for axis in ["xAxis", "yAxis"] {
        if let Some(Value::Object(a)) = o.get_mut(axis) {
            a.remove("data");
        }
    }
}

#[cfg(test)]
#[path = "prepare_view_tests.rs"]
mod prepare_view_tests;
