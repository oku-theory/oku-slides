//! Views resolved for the render: each chart and table param that names a deck data gets the
//! dataset its plan reads ([`oku_slides_core::plan_views`]), and the deck's data is emptied.
//!
//! Typst and the chart engine then see plain datasets: cell text stays a JSON string, never
//! Typst markup. A view that does not resolve (missing data or column, over the render budget,
//! a based data without a [`BaseRead`] or whose base does not read) has no dataset and draws a
//! failure frame. A dataset stored next to
//! a `data` ref is never trusted: the plan's one replaces it. A param without `data` is left as
//! is: a deck holds no inline dataset (ADR-0015), and the sanitizer bounds what it carries.
//!
//! A query data's views read its result ([`Results`]), a table over the result's base; a
//! query whose result is pending or failed has none, and its views draw the failure frame.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::sync::Arc;

use oku_slides_chart::MAX_TABLE_CELL_CHARS;
use oku_slides_core::{Deck, PlanOutcome, ViewPlan, WidgetRegistry, plan_views};
use oku_slides_core::{HostType, ParamValue, Params};
use oku_slides_data::{Data, DataError, DataId, Dataset, Source, Table};
use serde_json::Value as Json;

/// Reads a view over a data's base file: only the render worker has one ([`Renderer::set_bases`]).
///
/// [`Renderer::set_bases`]: crate::Renderer::set_bases
pub trait BaseRead: Send {
    /// The dataset of `columns` and of the data rows in `rows` of `table`, over its base, text
    /// cut to `max_cell_chars` characters.
    ///
    /// # Errors
    ///
    /// The data's refusal: a projection it does not have, or a base missing or invalid.
    fn read_view(
        &self,
        table: &Table,
        columns: &[String],
        rows: Range<usize>,
        max_cell_chars: usize,
    ) -> Result<Dataset, DataError>;
}

/// The result of each query data: a table over the result's base with an empty overlay, or
/// `None` while it is pending or after it failed.
pub type Results = BTreeMap<DataId, Option<Table>>;

/// `deck` with each query data that has a result in `results` turned into a data entered by
/// hand over that result: its name, label and views kept. Others are left as they are.
#[must_use]
pub fn with_results<'a>(deck: &'a Deck, results: &Results) -> Cow<'a, Deck> {
    let ready: Vec<(&DataId, &Table)> = results
        .iter()
        .filter_map(|(id, table)| Some((id, table.as_ref()?)))
        .filter(|(id, _)| deck.data.get(*id).is_some_and(|d| d.sql().is_some()))
        .collect();
    if ready.is_empty() {
        return Cow::Borrowed(deck);
    }
    let mut out = deck.clone();
    for (id, table) in ready {
        if let Some(data) = out.data.get_mut(id) {
            let derived = Data { source: Source::Manual(table.clone()), ..(**data).clone() };
            *data = Arc::new(derived);
        }
    }
    Cow::Owned(out)
}

/// `deck` with every view's dataset resolved and no data; `deck` is left unchanged. A based
/// data's views read through `bases`; without it they draw a failure frame. A query data's
/// views read its result in `results` ([`with_results`]).
#[must_use]
pub fn resolve_views(deck: &Deck, registry: &WidgetRegistry, bases: Option<&dyn BaseRead>, results: &Results) -> Deck {
    let derived = with_results(deck, results);
    let (plans, _) = plan_views(&derived, registry);
    let mut datasets: HashMap<(&str, &str, &str), Option<Json>> = HashMap::new();
    for plan in &plans {
        let key = (plan.view.slide.as_str(), plan.view.element.as_str(), plan.view.param.as_str());
        datasets.insert(key, dataset(&derived, plan, bases));
    }
    let mut out = deck.clone();
    out.data.clear();
    out.truncated.clear();
    for slide in &mut out.slides {
        let id = slide.id.clone();
        visit(&id, &id, &slide.kind, &mut slide.params, registry, &datasets);
    }
    out
}

/// The dataset a resolving plan reads, as JSON; none for any other outcome.
fn dataset(deck: &Deck, plan: &ViewPlan, bases: Option<&dyn BaseRead>) -> Option<Json> {
    if !matches!(plan.outcome, PlanOutcome::Resolve { .. }) {
        return None;
    }
    // A plan resolves only a manual data: a query data without a result plans as derived.
    let table = deck.data.get(plan.data.as_str())?.table()?;
    let read = match (&table.base, bases) {
        (None, _) => table.to_dataset(Some(&plan.columns), plan.rows.clone(), MAX_TABLE_CELL_CHARS),
        (Some(_), Some(bases)) => bases.read_view(table, &plan.columns, plan.rows.clone(), MAX_TABLE_CELL_CHARS),
        (Some(_), None) => {
            // Only the render worker reads bases: an in-process render draws the failure frame.
            tracing::warn!(data = plan.data, "based view rendered without a base reader");
            return None;
        }
    };
    match read {
        Ok(mut dataset) => {
            // Row and column ids serve edits, not drawing; the header's placeholder id would be refused.
            dataset.ids.clear();
            dataset.columns.clear();
            serde_json::to_value(dataset)
                .inspect_err(
                    |err| tracing::error!(error = %err, data = plan.data, "planned dataset does not serialize"),
                )
                .ok()
        }
        Err(err) => {
            // The plan checked the columns: a refusal is a bug or a base that does not read
            // (missing, invalid), shown as a frame.
            tracing::error!(error = %err, data = plan.data, "planned view does not resolve");
            None
        }
    }
}

type Datasets<'a> = HashMap<(&'a str, &'a str, &'a str), Option<Json>>;

/// Resolves the views of `owner`'s params, then of the elements of its slots.
fn visit(
    slide: &str,
    owner: &str,
    kind: &str,
    params: &mut Params,
    registry: &WidgetRegistry,
    datasets: &Datasets<'_>,
) {
    if let Some(schema) = registry.get(kind) {
        for p in &schema.params {
            if !matches!(p.ty.host(), Some(HostType::Chart | HostType::Table)) {
                continue;
            }
            let Some(ParamValue::Value(Json::Object(option))) = params.0.get_mut(&p.name) else { continue };
            if !option.contains_key("data") {
                continue;
            }
            // A dataset stored next to the ref is never drawn: only the plan's is.
            option.remove("dataset");
            option.remove("data");
            option.remove("columns");
            if let Some(Some(dataset)) = datasets.get(&(slide, owner, p.name.as_str())) {
                option.insert("dataset".to_owned(), dataset.clone());
            }
        }
    }
    for (_, elements) in params.slots_mut() {
        for e in elements {
            let id = e.id.clone();
            visit(slide, &id, &e.kind, &mut e.params, registry, datasets);
        }
    }
}

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;

#[cfg(test)]
#[path = "views_render_tests.rs"]
mod views_render_tests;
