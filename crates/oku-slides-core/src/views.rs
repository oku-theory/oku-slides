//! What each chart and table view reads of its data: columns, rows and the render budget.
//!
//! A view names a data and, optionally, the columns it reads. A table shows at most
//! `MAX_TABLE_ROWS - 1` rows and `MAX_TABLE_COLUMNS` columns; a chart reads every row of the
//! columns it uses. Views are counted in document order against the deck's render budget:
//! one that would pass it gets no data. The render resolves the views from these plans.

use std::collections::HashSet;
use std::ops::Range;

use oku_slides_chart::{ChartOption, MAX_TABLE_CELL_CHARS, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS};
use oku_slides_data::{DataCell, Table};
use oku_slides_i18n::Message;
use serde_json::Value;

use crate::data_users::ViewRef;
use crate::model::Deck;
use crate::params::{ParamValue, Params};
use crate::validate::Issue;
use crate::widget::{HostType, WidgetRegistry};

/// Cells all views of a deck may read for one render.
pub const MAX_RESOLVED_CELLS: usize = 200_000;
/// Text bytes (column names and text cells) all views of a deck may read for one render.
pub const MAX_RESOLVED_BYTES: usize = 16 * 1024 * 1024;

/// What one view reads, and whether it can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewPlan {
    /// The view and the data it names.
    pub view: ViewRef,
    /// The id of the data.
    pub data: String,
    /// Whether the view is a chart or a table.
    pub host: HostType,
    /// Column ids read, in order.
    pub columns: Vec<String>,
    /// Rows read, by position.
    pub rows: Range<usize>,
    /// Cells read.
    pub cells: usize,
    /// Text bytes read: column names and text cells, each cut to `MAX_TABLE_CELL_CHARS`.
    pub text_bytes: usize,
    /// Whether the view resolves.
    pub outcome: PlanOutcome,
}

/// Whether a view resolves, and why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOutcome {
    /// The view reads its columns and rows; `cut` when a table shows only part of its data.
    Resolve {
        /// Whether rows or columns were left out.
        cut: bool,
    },
    /// The view names a data the deck does not have.
    MissingData,
    /// The view reads a column its data does not have.
    MissingColumn(String),
    /// The view's option cannot be read: a column that is not an id, or a chart option that
    /// does not parse.
    Unreadable,
    /// The view would pass the deck's render budget.
    OverBudget,
}

/// The plan of every view of `deck` in document order, and the issues they raise.
#[must_use]
pub fn plan_views(deck: &Deck, registry: &WidgetRegistry) -> (Vec<ViewPlan>, Vec<Issue>) {
    let mut planner = Planner { deck, registry, cells: 0, bytes: 0, plans: Vec::new(), issues: Vec::new() };
    for slide in &deck.slides {
        planner.params(&slide.id, &slide.id, &slide.kind, &slide.params);
        for located in slide.walk() {
            let e = located.element;
            planner.params(&slide.id, &e.id, &e.kind, &e.params);
        }
    }
    (planner.plans, planner.issues)
}

struct Planner<'a> {
    deck: &'a Deck,
    registry: &'a WidgetRegistry,
    cells: usize,
    bytes: usize,
    plans: Vec<ViewPlan>,
    issues: Vec<Issue>,
}

impl Planner<'_> {
    fn params(&mut self, slide: &str, owner: &str, kind: &str, params: &Params) {
        let Some(schema) = self.registry.get(kind) else { return };
        for p in &schema.params {
            let Some(host @ (HostType::Chart | HostType::Table)) = p.ty.host() else { continue };
            let Some(ParamValue::Value(value)) = params.get(&p.name) else { continue };
            let Some(data) = value.get("data").and_then(Value::as_str) else { continue };
            let projection = value
                .get("columns")
                .and_then(Value::as_array)
                .map(|c| c.iter().map(|c| c.as_str().map(str::to_owned)).collect::<Option<Vec<_>>>());
            let view = ViewRef {
                slide: slide.to_owned(),
                element: owner.to_owned(),
                param: p.name.clone(),
                columns: projection.clone().flatten(),
            };
            let plan = match projection {
                Some(None) => ViewPlan { outcome: PlanOutcome::Unreadable, ..empty_plan(view, data, host) },
                Some(Some(columns)) => self.plan(view, data, host, value, Some(columns)),
                None => self.plan(view, data, host, value, None),
            };
            self.issue(&plan);
            self.plans.push(plan);
        }
    }

    fn plan(
        &mut self,
        view: ViewRef,
        data: &str,
        host: HostType,
        value: &Value,
        projection: Option<Vec<String>>,
    ) -> ViewPlan {
        let mut plan = empty_plan(view, data, host);
        let Some(table) = self.deck.data.get(data).map(|d| &d.table) else {
            return plan;
        };
        let mut columns = match projection {
            Some(columns) => columns,
            None if host == HostType::Chart => {
                let Ok(option) = serde_json::from_value::<ChartOption>(value.clone()) else {
                    plan.outcome = PlanOutcome::Unreadable;
                    return plan;
                };
                let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
                let used = option.used_columns(&names);
                table.columns.iter().take(used).map(|c| c.id.clone()).collect()
            }
            None => table.columns.iter().map(|c| c.id.clone()).collect(),
        };
        if let Some(missing) = columns.iter().find(|c| !table.columns.iter().any(|col| col.id == **c)) {
            plan.outcome = PlanOutcome::MissingColumn(missing.clone());
            return plan;
        }
        let total_rows = table.rows.len();
        let (rows, cut) = if host == HostType::Table {
            let shown = total_rows.min(MAX_TABLE_ROWS - 1);
            let cut = shown < total_rows || columns.len() > MAX_TABLE_COLUMNS;
            columns.truncate(MAX_TABLE_COLUMNS);
            (0..shown, cut)
        } else {
            (0..total_rows, false)
        };
        plan.cells = rows.len().saturating_mul(columns.len());
        let cells = self.cells.saturating_add(plan.cells);
        // Cells first: text is counted only for a view within the cell budget, and only
        // until it passes the bytes left.
        if cells <= MAX_RESOLVED_CELLS {
            let left = MAX_RESOLVED_BYTES - self.bytes;
            plan.text_bytes = text_bytes(table, &columns, rows.clone(), left);
        }
        plan.columns = columns;
        plan.rows = rows;
        let bytes = self.bytes.saturating_add(plan.text_bytes);
        if cells > MAX_RESOLVED_CELLS || bytes > MAX_RESOLVED_BYTES {
            plan.outcome = PlanOutcome::OverBudget;
        } else {
            self.cells = cells;
            self.bytes = bytes;
            plan.outcome = PlanOutcome::Resolve { cut };
        }
        plan
    }

    fn issue(&mut self, plan: &ViewPlan) {
        let element = plan.view.element.as_str();
        let messages = match &plan.outcome {
            PlanOutcome::Resolve { cut: false } => return,
            // A cut plan was made from this deck's data, which is still there.
            PlanOutcome::Resolve { cut: true } => {
                let Some(table) = self.deck.data.get(plan.data.as_str()).map(|d| &d.table) else { return };
                let truncated = |what: &str, shown: usize, total: usize| {
                    (shown < total).then(|| {
                        Message::new("issue-view-truncated")
                            .arg("element", element)
                            .arg("what", what.to_owned())
                            .arg("shown", shown)
                            .arg("total", total)
                    })
                };
                let columns = plan.view.columns.as_ref().map_or(table.columns.len(), Vec::len);
                let rows = truncated("rows", plan.rows.len(), table.rows.len());
                rows.into_iter().chain(truncated("columns", plan.columns.len(), columns)).collect()
            }
            PlanOutcome::MissingData => {
                vec![Message::new("issue-data-missing").arg("element", element).arg("data", plan.data.as_str())]
            }
            PlanOutcome::MissingColumn(column) => {
                vec![Message::new("issue-view-column-missing").arg("element", element).arg("column", column.as_str())]
            }
            PlanOutcome::Unreadable => vec![Message::new("issue-view-unreadable").arg("element", element)],
            PlanOutcome::OverBudget => vec![Message::new("issue-render-budget").arg("element", element)],
        };
        let slide = plan.view.slide.clone();
        let element = (plan.view.element != slide).then(|| plan.view.element.clone());
        for message in messages {
            self.issues.push(Issue { slide: Some(slide.clone()), element: element.clone(), message });
        }
    }
}

/// A plan of `view` reading nothing, as for a data the deck does not have.
fn empty_plan(view: ViewRef, data: &str, host: HostType) -> ViewPlan {
    ViewPlan {
        view,
        data: data.to_owned(),
        host,
        columns: Vec::new(),
        rows: 0..0,
        cells: 0,
        text_bytes: 0,
        outcome: PlanOutcome::MissingData,
    }
}

/// Column names and text cells read, each cell cut to `MAX_TABLE_CELL_CHARS`; the count stops
/// once it passes `left`.
///
/// Only text cells count: the overlay's entries are walked, never every (row, column) of the
/// view; a based column counts its base's text bytes in proportion to the rows read.
fn text_bytes(table: &Table, columns: &[String], rows: Range<usize>, left: usize) -> usize {
    let mut bytes: usize = table.columns.iter().filter(|c| columns.contains(&c.id)).map(|c| c.name.len()).sum();
    if let Some(base) = &table.base {
        let read = u64::try_from(rows.len()).unwrap_or(u64::MAX);
        for column in base.columns.iter().filter(|c| columns.contains(&c.id)) {
            let share = column.text_bytes.saturating_mul(read) / u64::from(base.rows.max(1));
            bytes = bytes.saturating_add(usize::try_from(share).unwrap_or(usize::MAX));
        }
    }
    let read: HashSet<&str> = columns.iter().map(String::as_str).collect();
    for (key, cell) in &table.cells {
        if bytes > left {
            break;
        }
        let DataCell::Text(s) = cell else { continue };
        if read.contains(key.column.as_str()) && table.rows.index_of(&key.row).is_some_and(|i| rows.contains(&i)) {
            bytes = bytes.saturating_add(cut_len(s));
        }
    }
    bytes
}

/// Bytes of the first `MAX_TABLE_CELL_CHARS` chars of `s`.
fn cut_len(s: &str) -> usize {
    s.char_indices().nth(MAX_TABLE_CELL_CHARS).map_or(s.len(), |(i, _)| i)
}

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;
