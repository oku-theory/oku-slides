//! The `host-data` surface: the data of a selected chart or table, in the data pane.
//!
//! A view names a deck data (`data` in its `host(chart)` or `host(table)` param); the pane shows
//! the columns the view reads and its first `PANE_ROWS` rows, with their ids, which the edits
//! of `data_edit` name.

use std::ops::Range;
use std::sync::Arc;

use oku_slides_a2ui::{Action, ComponentKind, DataChoice, DataRef, Dynamic, SeriesSwatch};
use oku_slides_core::{Deck, HostType, OpError, PlanOutcome, ViewPlan, bind_choices, plan_views};
use oku_slides_data::limits::MAX_TEXT_BYTES;
use oku_slides_data::{Data, DataCell, DataError, DataId, Dataset};
use serde_json::Value;
use snafu::{IntoError, OptionExt};

use crate::agent::Surface;
use crate::agent::forms::data_surface;
use crate::data_read::RowsRead;
use crate::error::{NoChartSnafu, PrepareSnafu};
use crate::{Result, Session, SessionError};

/// The surface of the selected element's data.
pub(crate) const HOST_DATA: &str = "host-data";
/// The event of an edit; its context reads `/edit`.
pub(crate) const DATA_EVENT: &str = "data";
/// The stack group the UI sets.
pub(crate) const STACK_GROUP: &str = "total";
/// Rows the pane shows, from the first (Task 7 pages them).
pub(crate) const PANE_ROWS: usize = 99;

/// A chart or table param of an element, as the deck holds it.
#[derive(Debug, Clone)]
pub(crate) struct DataTarget {
    /// The slide that holds the element.
    pub slide: String,
    /// The element.
    pub element: String,
    /// The element's host param.
    pub param: String,
    /// Chart or table.
    pub host: HostType,
    /// Its value in the deck.
    pub value: Value,
}

impl DataTarget {
    /// The id of the data the view names.
    ///
    /// # Errors
    ///
    /// `op-data-missing` (naming the element) for a view that names no data.
    pub(crate) fn data(&self) -> Result<&str> {
        match self.value.get("data").and_then(Value::as_str) {
            Some(data) => Ok(data),
            None => Err(data_missing(&self.element)),
        }
    }
}

/// The refusal of a view naming no data, or a data the deck does not have (`op-data-missing`).
pub(crate) fn data_missing(data: &str) -> SessionError {
    PrepareSnafu.into_error(OpError::data_missing(data)).into()
}

/// What a view reads: its data, the columns and the rows.
struct ViewSpan {
    data: DataId,
    stored: Arc<Data>,
    columns: Option<Vec<String>>,
    rows: Range<usize>,
}

/// A data pane taken from the session, its rows read with no lock held ([`PaneRead::surface`]).
#[derive(Debug)]
pub(crate) struct PaneRead {
    target: DataTarget,
    read: RowsRead,
    /// The pane's localized title.
    title: String,
    /// The deck's chart palette, for a chart's series swatches.
    palette: Vec<String>,
    /// The data the view shows.
    data: DataRef,
    /// The data it can switch to.
    choices: Vec<DataChoice>,
}

impl PaneRead {
    /// The `host-data` surface; `None` if the rows cannot be read. Blocking for a based data:
    /// call it with no session lock held.
    pub(crate) fn surface(self) -> Option<Surface> {
        let dataset = match self.read.run() {
            Ok(dataset) => dataset,
            Err(error) => {
                let (slide, element) = (&self.target.slide, &self.target.element);
                tracing::debug!(%slide, %element, %error, "no data pane for this view");
                return None;
            }
        };
        let data = match self.target.host {
            HostType::Table => table_grid(&self.target.value, &dataset, self.data, self.choices),
            _ => chart_grid(&self.target.value, &dataset, &self.palette, self.data, self.choices),
        };
        Some(data_surface(HOST_DATA, self.title, data))
    }
}

impl Session {
    /// The `host-data` surface of `element`, read under the caller's lock ([`Session::data_pane`]).
    pub(crate) fn data_panel(&self, slide: &str, element: &str) -> Option<Surface> {
        self.data_pane(slide, element)?.surface()
    }

    /// The pane of `element`, to read with no lock held; `None` outside an editable deck, for
    /// a missing element or one without a chart or table, or for a view whose data does not
    /// resolve.
    pub(crate) fn data_pane(&self, slide: &str, element: &str) -> Option<PaneRead> {
        if self.ensure_editable().is_err() {
            return None;
        }
        let target = self.data_target(slide, element)?;
        let read = match self.view_read(&target, Some(0..PANE_ROWS), MAX_TEXT_BYTES) {
            Ok(read) => read,
            Err(error) => {
                // The deck's issues already report a view that does not resolve.
                tracing::debug!(%slide, %element, %error, "no data pane for this view");
                return None;
            }
        };
        let title = match target.host {
            HostType::Table => "session-data-table",
            _ => "session-data-chart",
        };
        let (title, palette) = (self.localizer.tr(title), self.appearance.chart_palette.clone());
        let (data, choices) = binding(self.doc.sources.deck(), &target)?;
        Some(PaneRead { target, read, title, palette, data, choices })
    }

    /// The chart (else table) param of `element` on `slide`, if it has one.
    pub(crate) fn data_target(&self, slide: &str, element: &str) -> Option<DataTarget> {
        let deck = self.doc.sources.deck();
        let located = deck.slide(slide)?.locate(element)?;
        let schema = self.registry.get(&located.element.kind)?;
        [HostType::Chart, HostType::Table].into_iter().find_map(|host| {
            let param = schema.host_param(host)?.name.clone();
            match located.element.params.get(&param) {
                Some(oku_slides_core::ParamValue::Value(v)) => Some(DataTarget {
                    slide: slide.to_owned(),
                    element: element.to_owned(),
                    param,
                    host,
                    value: v.clone(),
                }),
                _ => None,
            }
        })
    }

    /// The dataset `target` reads: its columns and all its rows, with the ids of its rows and
    /// columns, text cut to `max_chars` characters. A based data is never read under the lock
    /// ([`Session::rows_read_off_lock`]): call it through `data_compact::folding`.
    ///
    /// # Errors
    ///
    /// `op-data-missing` for a view without data or naming a data the deck does not have; the
    /// data's refusal for a column it lacks or a base; `NoChart` for an option that cannot be read;
    /// `BaseStale` while its base read is due.
    pub(crate) fn view_dataset(&mut self, target: &DataTarget, max_chars: usize) -> Result<Dataset> {
        let span = self.view_span(target, None)?;
        self.rows_read_off_lock(&span.data, span.columns, span.rows, max_chars)
    }

    /// The read of `target`'s columns and `rows` (all the rows it reads if `None`), to run with no
    /// lock held. The pane passes `MAX_TEXT_BYTES`: a text cell holds no more bytes, so it is
    /// never cut.
    ///
    /// # Errors
    ///
    /// As [`Session::view_dataset`], but the worker's and `BaseStale`.
    pub(crate) fn view_read(
        &self,
        target: &DataTarget,
        rows: Option<Range<usize>>,
        max_chars: usize,
    ) -> Result<RowsRead> {
        let span = self.view_span(target, rows)?;
        self.rows_read(&span.stored, span.columns.as_deref(), span.rows, max_chars)
    }

    /// The data `target` reads, its columns and `rows` (all the rows it reads if `None`).
    fn view_span(&self, target: &DataTarget, rows: Option<Range<usize>>) -> Result<ViewSpan> {
        let data = target.data()?;
        let deck = self.doc.sources.deck();
        let Some((id, stored)) = deck.data.get_key_value(data) else {
            return Err(data_missing(data));
        };
        let (id, stored) = (id.clone(), Arc::clone(stored));
        let plan = self.view_plan(target)?;
        let columns = match plan.outcome {
            // Over the render budget, the view still names the columns it would read.
            PlanOutcome::Resolve { .. } | PlanOutcome::OverBudget => Some(plan.columns),
            // `to_dataset` refuses the missing column, with its reason.
            PlanOutcome::MissingColumn(_) => plan.view.columns,
            PlanOutcome::MissingData => return Err(data_missing(data)),
            PlanOutcome::Unreadable => return Err(NoChartSnafu { id: &target.element }.build().into()),
        };
        let rows = rows.map_or(plan.rows.clone(), |r| r.start..r.end.min(plan.rows.end));
        Ok(ViewSpan { data: id, stored, columns, rows })
    }

    /// The column ids `target` reads, in order, from the deck alone: no base is read, so it
    /// holds under the session lock.
    ///
    /// # Errors
    ///
    /// As [`Session::view_dataset`], without the worker's.
    pub(crate) fn view_columns(&self, target: &DataTarget) -> Result<Vec<String>> {
        plan_columns(self.view_plan(target)?)
    }

    /// The plan of view `target`.
    fn view_plan(&self, target: &DataTarget) -> Result<ViewPlan> {
        let (plans, _) = plan_views(self.doc.sources.deck(), &self.registry);
        let plan = plans
            .into_iter()
            .find(|p| p.view.slide == target.slide && p.view.element == target.element && p.view.param == target.param);
        Ok(plan.context(NoChartSnafu { id: &target.element })?)
    }
}

/// The column ids a resolved `plan` reads, as the read of its rows would name them.
///
/// # Errors
///
/// The data's refusal of a missing column; `op-data-missing`; `NoChart` for an unreadable view.
pub(crate) fn plan_columns(plan: ViewPlan) -> Result<Vec<String>> {
    match plan.outcome {
        PlanOutcome::Resolve { .. } | PlanOutcome::OverBudget => Ok(plan.columns),
        PlanOutcome::MissingColumn(column) => {
            Err(PrepareSnafu.into_error(OpError::from(DataError::no_column(&column))).into())
        }
        PlanOutcome::MissingData => Err(data_missing(&plan.data)),
        PlanOutcome::Unreadable => Err(NoChartSnafu { id: plan.view.element }.build().into()),
    }
}

/// The action of the data components: `data`, with the edit written at `/edit`.
fn data_action() -> Action {
    Action::event(DATA_EVENT, [("edit", Dynamic::path("/edit"))])
}

/// `dataset.source` as JSON cells: row 0 holds the column names.
pub(crate) fn source(dataset: &Dataset) -> Vec<Vec<Value>> {
    let cell = |c: &DataCell| match c {
        // A table holds finite numbers only.
        DataCell::Number(n) => Value::from(*n),
        DataCell::Text(t) => Value::String(t.clone()),
        DataCell::Bool(b) => Value::Bool(*b),
        DataCell::Null => Value::Null,
    };
    dataset.source.iter().map(|row| row.iter().map(cell).collect()).collect()
}

/// The ids of the data rows of `dataset` (`ids[0]` stands for the header row).
pub(crate) fn row_ids(dataset: &Dataset) -> Vec<String> {
    dataset.ids.iter().skip(1).cloned().collect()
}

fn series(option: &Value) -> &[Value] {
    option.get("series").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn kind_of(s: &Value) -> Option<&str> {
    s.get("type").and_then(Value::as_str)
}

/// Bars or lines: stacking and per-column color make sense.
fn cartesian(s: &Value) -> bool {
    matches!(kind_of(s), Some("bar" | "line"))
}

/// Several series, all bars or all lines.
pub(crate) fn stackable(option: &Value) -> bool {
    let all = series(option);
    all.len() > 1 && all.iter().all(cartesian)
}

/// All the series share one stack group.
fn stacked(option: &Value) -> bool {
    let all = series(option);
    let first = all.first().and_then(|s| s.get("stack")).and_then(Value::as_str);
    first.is_some() && all.iter().all(|s| s.get("stack").and_then(Value::as_str) == first)
}

/// A `#rgb` or `#rrggbb` color as lowercase `#rrggbb`; `None` for any other value.
pub(crate) fn hex_color(value: &Value) -> Option<String> {
    let s = value.as_str()?.trim();
    let digits = s.strip_prefix('#')?;
    if !matches!(digits.len(), 3 | 6) || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let lower = digits.to_ascii_lowercase();
    let full = if lower.len() == 3 { lower.chars().flat_map(|c| [c, c]).collect() } else { lower };
    Some(format!("#{full}"))
}

/// The series one can color, named by their header: bars and lines read column i + 1, a
/// scatter the pair (2i, 2i + 1); pie, radar and waterfall follow the palette.
fn swatches(option: &Value, header: &[Value]) -> Vec<SeriesSwatch> {
    let all = series(option);
    let column = |i: usize| -> Option<usize> {
        if all.iter().all(cartesian) {
            Some(i + 1)
        } else if all.iter().all(|s| kind_of(s) == Some("scatter")) {
            Some(2 * i + 1)
        } else {
            None
        }
    };
    let name = |col: usize| header.get(col).map(cell_text).unwrap_or_default();
    all.iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let color = s.pointer("/itemStyle/color").and_then(hex_color);
            column(i).map(|col| SeriesSwatch { name: name(col), color })
        })
        .collect()
}

/// The text of a header cell.
fn cell_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The data `target` shows and those it can switch to (`bind_choices`); `None`, logged, for a
/// data the deck does not have.
fn binding(deck: &Deck, target: &DataTarget) -> Option<(DataRef, Vec<DataChoice>)> {
    let named = target.data().ok()?;
    let Some((id, data)) = deck.data.get_key_value(named) else {
        tracing::debug!(element = %target.element, data = %named, "no data pane: the data is missing");
        return None;
    };
    let users = deck.data_users(id).len();
    let choices = bind_choices(deck, target.host, &target.value)
        .into_iter()
        .filter_map(|id| deck.data.get(&id).map(|d| (id, d)))
        .map(|(id, d)| DataChoice { id: id.as_str().to_owned(), name: d.name.clone(), label: d.label.clone() })
        .collect();
    let data = DataRef { id: id.as_str().to_owned(), name: data.name.clone(), label: data.label.clone(), users };
    Some((data, choices))
}

/// `ChartData` of a chart option showing `dataset`, with the theme's palette and the data it
/// shows, and can switch to.
pub(crate) fn chart_grid(
    option: &Value,
    dataset: &Dataset,
    palette: &[String],
    data: DataRef,
    choices: Vec<DataChoice>,
) -> ComponentKind {
    let source = source(dataset);
    let kind = series(option).first().and_then(kind_of).map(str::to_owned);
    let width = source.first().map_or(0, Vec::len);
    let checkbox_columns = if kind.as_deref() == Some("waterfall") && width > 2 { vec![2] } else { Vec::new() };
    let header = source.first().cloned().unwrap_or_default();
    ComponentKind::ChartData {
        kind,
        checkbox_columns,
        stacked: stackable(option).then(|| stacked(option)),
        series: swatches(option, &header),
        palette: palette.iter().filter_map(|c| hex_color(&Value::String(c.clone()))).collect(),
        source,
        row_ids: row_ids(dataset),
        column_ids: dataset.columns.clone(),
        data,
        choices,
        edit: Dynamic::path("/edit"),
        action: data_action(),
    }
}

/// `TableData` of a table option showing `dataset` and its data; missing flags take their
/// defaults.
pub(crate) fn table_grid(option: &Value, dataset: &Dataset, data: DataRef, choices: Vec<DataChoice>) -> ComponentKind {
    let flag = |key: &str, default: bool| option.get(key).and_then(Value::as_bool).unwrap_or(default);
    let align = option
        .get("align")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|v| v.as_str().unwrap_or("auto").to_owned()).collect());
    ComponentKind::TableData {
        source: source(dataset),
        row_ids: row_ids(dataset),
        column_ids: dataset.columns.clone(),
        header_row: flag("header-row", true),
        header_column: flag("header-column", false),
        striped: flag("striped", false),
        align: align.unwrap_or_default(),
        data,
        choices,
        edit: Dynamic::path("/edit"),
        action: data_action(),
    }
}

#[cfg(test)]
#[path = "data_panel_tests.rs"]
mod data_panel_tests;
