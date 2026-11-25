//! Edits of a chart's or a table's data sent by the `host-data` surface: each becomes ops, in
//! one undo step named by the session. Grid edits name the data's rows and columns by id
//! (`edit_data`); the patches (stacking, color, alignment) are computed here, from the deck.

use oku_slides_chart::{ChartOption, ColumnAlign, SeriesType, insert_column_series, remove_column_series};
use oku_slides_core::{
    Deck, HostType, Op, OpError, ParamValue, PlanOutcome, Target, ViewRef, WidgetRegistry, plan_views,
};
use oku_slides_data::{DataCell, DataId, TableOp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snafu::{OptionExt, ResultExt};

use crate::data_panel::{DataTarget, STACK_GROUP, data_missing, hex_color, plan_columns, stackable};
use crate::data_paste::Paste;
use crate::error::{HostKeySnafu, NoChartSnafu, NoTableSnafu, PrepareSnafu, ResponseSnafu, ViewUnfollowedSnafu};
use crate::request::View;
use crate::{Result, Session};

/// An edit from the `host-data` surface, named by `edit`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataEdit {
    /// Changes the data the view names (cell, row, column), by row and column id.
    Cells {
        /// The edit of the data's table, applied as is.
        change: TableOp,
    },
    /// Pastes a block at a cell, by row and column id: the cells it covers are set, and the
    /// rows and columns past the data's last row and the view's last column are added.
    Paste {
        /// Row of the top-left cell.
        row: String,
        /// Column of the top-left cell.
        column: String,
        /// The pasted cells, row by row.
        cells: Vec<Vec<DataCell>>,
        /// Names of the columns the block adds, in order.
        #[serde(default)]
        names: Vec<String>,
    },
    /// Adds an empty row after the data's last row, past the rows the pane shows.
    AddRow,
    /// Changes the chart type; its data is converted.
    Kind {
        /// New series type of the chart.
        kind: SeriesType,
    },
    /// Stacks all the series, or unstacks them.
    Stack {
        /// `true` to stack the series.
        stacked: bool,
    },
    /// Colors the series `index`, or hands it back to the palette (`null`).
    Color {
        /// Index of the series, from 0.
        index: usize,
        /// Hexadecimal color, or `null` for the palette color.
        color: Option<String>,
    },
    /// Turns a display option of a table on or off.
    Flag {
        /// The targeted display option.
        flag: TableFlag,
        /// `true` to turn it on.
        on: bool,
    },
    /// Aligns the column `column` of a table.
    Align {
        /// Index of the column, from 0.
        column: usize,
        /// Text alignment of the column.
        align: ColumnAlign,
    },
    /// Shows the same data as a chart or as a table.
    ShowAs {
        /// Wanted view: chart or table.
        view: View,
    },
    /// Shows another data of the deck, which holds every column the view reads.
    Bind {
        /// The id of the data.
        data: String,
    },
}

/// Display option of a table, named as in its options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TableFlag {
    /// Row 0 as a header.
    HeaderRow,
    /// Column 0 as a header.
    HeaderColumn,
    /// Every other row on a background.
    Striped,
}

impl TableFlag {
    fn key(self) -> &'static str {
        match self {
            Self::HeaderRow => "header-row",
            Self::HeaderColumn => "header-column",
            Self::Striped => "striped",
        }
    }
}

impl Session {
    /// Applies `edit` to chart or table `element` of `slide`, in one undo step.
    pub(crate) fn data_edit(&mut self, slide: String, element: &str, edit: DataEdit) -> Result<()> {
        let target = self.data_target(&slide, element).context(NoChartSnafu { id: element })?;
        let owner = Target { slide: slide.clone(), element: Some(element.to_owned()) };
        let chart = target.host == HostType::Chart;
        let need = |ok: bool, table: bool| -> Result<()> {
            match (ok, table) {
                (true, _) => Ok(()),
                (false, true) => Err(NoTableSnafu { id: element }.build().into()),
                (false, false) => Err(NoChartSnafu { id: element }.build().into()),
            }
        };
        match edit {
            DataEdit::Cells { change } => {
                let id = DataId::try_from(target.data()?.to_owned()).map_err(OpError::from).context(PrepareSnafu)?;
                let (ops, merge) = self.grid_ops(&id, change, Followers::View(&owner, &target))?;
                self.apply_folding(&self.localizer.tr("session-undo-data-edit"), ops, merge.as_deref())
            }
            DataEdit::Paste { row, column, cells, names } => {
                let ops = self.paste_ops(&owner, &target, Paste { row, column, cells, names })?;
                self.apply_folding(&self.localizer.tr("session-undo-data-edit"), ops, None)
            }
            DataEdit::AddRow => {
                let id = DataId::try_from(target.data()?.to_owned()).map_err(OpError::from).context(PrepareSnafu)?;
                let op = self.add_row_op(&id)?;
                self.apply_folding(&self.localizer.tr("session-undo-data-edit"), vec![op], None)
            }
            DataEdit::Kind { kind } => {
                need(chart, false)?;
                self.set_chart_kind(&self.localizer.tr("session-undo-chart-kind"), slide, element, kind)
            }
            DataEdit::Stack { stacked } => {
                need(chart && stackable(&target.value), false)?;
                let label = self.localizer.tr_with("session-undo-chart-stack", &[("stacked", yes_no(stacked).into())]);
                let op = chart_op(owner, &target, stack_patch(&target.value, stacked));
                self.apply(&label, vec![op], None)
            }
            DataEdit::Color { index, color } => {
                need(chart, false)?;
                let color = match color {
                    Some(c) => Some(hex_color(&Value::String(c.clone())).context(HostKeySnafu { key: c })?),
                    None => None,
                };
                let op = chart_op(owner, &target, color_patch(&target.value, index, color.as_deref()));
                self.apply(&self.localizer.tr("session-undo-chart-color"), vec![op], None)
            }
            DataEdit::Flag { flag, on } => {
                need(!chart, true)?;
                let op = Op::SetTableOption { target: owner, param: target.param, patch: json!({ flag.key(): on }) };
                self.apply(&self.localizer.tr("session-undo-table-option"), vec![op], None)
            }
            DataEdit::Align { column, align } => {
                need(!chart, true)?;
                let width = self.view_columns(&target)?.len();
                let op = Op::SetTableOption {
                    target: owner,
                    param: target.param,
                    patch: align_patch(&target.value, width, column, align),
                };
                self.apply(&self.localizer.tr("session-undo-table-align"), vec![op], None)
            }
            DataEdit::ShowAs { view } => {
                need(chart == (view == View::Table), view == View::Chart)?;
                let key = match view {
                    View::Table => "session-undo-show-as-table",
                    View::Chart => "session-undo-show-as-chart",
                };
                self.show_as(&self.localizer.tr(key), slide, element, view)
            }
            DataEdit::Bind { data } => {
                let data = DataId::try_from(data).map_err(OpError::from).context(PrepareSnafu)?;
                let op = Op::BindData { target: owner, param: target.param, data, columns: None };
                self.apply_folding(&self.localizer.tr("session-undo-data-bind"), vec![op], None)
            }
        }
    }
}

/// The views a grid edit's column insert or remove keeps in step.
#[derive(Clone, Copy)]
pub(crate) enum Followers<'a> {
    /// The data pane's own view, with its owner.
    View(&'a Target, &'a DataTarget),
    /// Every view of the data: the data sheet's.
    Every,
}

/// A view a column change keeps in step: its owner, its param, and the column ids it reads.
pub(crate) struct ViewFollow {
    owner: Target,
    target: DataTarget,
    read: Vec<String>,
}

impl Session {
    /// The ops of grid edit `change` on `data`, with `followers` kept on the columns they
    /// read, and the key folding repeated edits of one cell into one undo step. Only a column
    /// insert or remove looks at the views, through the deck alone: no base is read.
    ///
    /// # Errors
    ///
    /// `Prepare` when the core draws no id for an inserted column; for a column insert or
    /// remove, a view that cannot follow it.
    pub(crate) fn grid_ops(
        &self,
        data: &DataId,
        mut change: TableOp,
        followers: Followers<'_>,
    ) -> Result<(Vec<Op>, Option<String>)> {
        let merge = match &change {
            TableOp::SetCell { row, column, .. } => Some(format!("data:{}/{row}/{column}", data.as_str())),
            _ => None,
        };
        if let TableOp::InsertColumn { id: None, .. } = change {
            let drawn = self.draw_ids(data, vec![change.clone()])?;
            change = drawn.into_iter().next().ok_or_else(|| data_missing(data.as_str()))?;
        }
        let (mut before, mut after) = (Vec::new(), Vec::new());
        match &change {
            TableOp::InsertColumn { id: Some(id), after: anchor, .. } => {
                for view in self.followers(data, followers)? {
                    after.extend(follow_inserts(&view, anchor.as_deref(), std::slice::from_ref(id))?);
                }
            }
            TableOp::RemoveColumn { column } => {
                for view in self.followers(data, followers)? {
                    before.extend(follow_removal(&view, column)?);
                }
            }
            _ => {}
        }
        let mut ops = before;
        ops.push(Op::EditData { data: data.clone(), change });
        ops.extend(after);
        Ok((ops, merge))
    }

    /// The views `followers` names, each with the columns it reads.
    ///
    /// # Errors
    ///
    /// The pane's view as [`Session::view_columns`]; as [`data_views`] for every view.
    fn followers(&self, data: &DataId, followers: Followers<'_>) -> Result<Vec<ViewFollow>> {
        match followers {
            Followers::View(owner, target) => {
                let read = self.view_columns(target)?;
                Ok(vec![ViewFollow { owner: owner.clone(), target: target.clone(), read }])
            }
            Followers::Every => data_views(self.doc.sources.deck(), &self.registry, data),
        }
    }

    /// The op adding an empty row after the last row of `data`.
    ///
    /// # Errors
    ///
    /// `Prepare` when the deck holds no data `data`.
    pub(crate) fn add_row_op(&self, data: &DataId) -> Result<Op> {
        let last = {
            let deck = self.doc.sources.deck();
            let found = deck.data.get(data.as_str()).ok_or_else(|| data_missing(data.as_str()))?;
            found.table.rows.iter().last().map(str::to_owned)
        };
        let change = TableOp::InsertRow { id: None, after: last, cells: std::collections::BTreeMap::new() };
        Ok(Op::EditData { data: data.clone(), change })
    }
}

/// Every view of `data` in `deck`, once per param, slide params included, with the columns
/// it reads, from the deck alone.
///
/// # Errors
///
/// `ViewUnfollowed` for a view `registry` cannot read (a widget it does not know, a param
/// that is not a chart or table, an unreadable option); a projected column the data lacks.
pub(crate) fn data_views(deck: &Deck, registry: &WidgetRegistry, data: &DataId) -> Result<Vec<ViewFollow>> {
    let (mut plans, _) = plan_views(deck, registry);
    deck.data_users(data)
        .into_iter()
        .map(|user| {
            let unfollowed = || ViewUnfollowedSnafu { view: user.element.clone() }.build();
            let found = plans.iter().position(|p| {
                p.view.slide == user.slide && p.view.element == user.element && p.view.param == user.param
            });
            let plan = plans.swap_remove(found.ok_or_else(unfollowed)?);
            if plan.outcome == PlanOutcome::Unreadable {
                return Err(unfollowed().into());
            }
            let (owner, value) = view_param(deck, &user).ok_or_else(unfollowed)?;
            let target = DataTarget {
                slide: user.slide.clone(),
                element: user.element.clone(),
                param: user.param.clone(),
                host: plan.host,
                value,
            };
            Ok(ViewFollow { owner, target, read: plan_columns(plan)? })
        })
        .collect()
}

/// The owner of `view` (the slide itself for a slide param) and the param's value.
fn view_param(deck: &Deck, view: &ViewRef) -> Option<(Target, Value)> {
    let slide = deck.slide(&view.slide)?;
    let (element, params) = if view.element == slide.id {
        (None, &slide.params)
    } else {
        (Some(view.element.clone()), &slide.locate(&view.element)?.element.params)
    };
    match params.get(&view.param)? {
        ParamValue::Value(value) => Some((Target { slide: view.slide.clone(), element }, value.clone())),
        _ => None,
    }
}

/// The op keeping `view` on columns `ids` inserted after `anchor` (first when `None`): a
/// projection takes them, a chart its series. Nothing when the view does not read `anchor`.
///
/// # Errors
///
/// `NoChart` for a chart option that does not parse.
pub(crate) fn follow_inserts(view: &ViewFollow, anchor: Option<&str>, ids: &[String]) -> Result<Option<Op>> {
    let mut read = view.read.clone();
    let at = match anchor {
        None => 0,
        Some(anchor) => match read.iter().position(|c| c == anchor) {
            Some(i) => i + 1,
            None => return Ok(None),
        },
    };
    read.splice(at..at, ids.iter().cloned());
    follow_op(&view.owner, &view.target, &read, &Follow::Inserted((at..at + ids.len()).collect()))
}

/// The op keeping `view` on its columns once `column` is removed, applied first since a
/// projected column cannot be removed. Nothing when the view does not read it.
fn follow_removal(view: &ViewFollow, column: &str) -> Result<Option<Op>> {
    let mut read = view.read.clone();
    let Some(at) = read.iter().position(|c| c == column) else { return Ok(None) };
    read.remove(at);
    follow_op(&view.owner, &view.target, &read, &Follow::Removed(at))
}

/// How the columns a view reads change.
pub(crate) enum Follow {
    /// Columns inserted at these positions of the new list, ascending.
    Inserted(Vec<usize>),
    /// The column at this position removed.
    Removed(usize),
}

/// The op keeping view `target` on `read`, the columns it reads once `follow` is done: its
/// projection, if any, becomes `read`; a chart's bars and lines keep one series per column.
/// `None` when nothing changes.
pub(crate) fn follow_op(owner: &Target, target: &DataTarget, read: &[String], follow: &Follow) -> Result<Option<Op>> {
    let mut patch = serde_json::Map::new();
    if target.value.get("columns").is_some_and(Value::is_array) {
        patch.insert("columns".into(), json!(read));
    }
    if target.host == HostType::Chart {
        let Ok(mut option) = serde_json::from_value::<ChartOption>(target.value.clone()) else {
            return Err(NoChartSnafu { id: &target.element }.build().into());
        };
        let before = option.series.clone();
        match follow {
            Follow::Inserted(at) => {
                for &at in at {
                    insert_column_series(&mut option, at);
                }
            }
            Follow::Removed(at) => remove_column_series(&mut option, *at),
        }
        if option.series != before {
            patch.insert("series".into(), serde_json::to_value(&option.series).context(ResponseSnafu)?);
        }
    }
    if patch.is_empty() {
        return Ok(None);
    }
    let patch = Value::Object(patch);
    Ok(Some(match target.host {
        HostType::Chart => chart_op(owner.clone(), target, patch),
        _ => Op::SetTableOption { target: owner.clone(), param: target.param.clone(), patch },
    }))
}

fn yes_no(on: bool) -> &'static str {
    if on { "yes" } else { "no" }
}

fn chart_op(owner: Target, target: &DataTarget, patch: Value) -> Op {
    Op::SetChartOption { target: owner, param: target.param.clone(), patch }
}

fn series(option: &Value) -> Vec<serde_json::Map<String, Value>> {
    let all = option.get("series").and_then(Value::as_array);
    all.map(|a| a.iter().map(|s| s.as_object().cloned().unwrap_or_default()).collect()).unwrap_or_default()
}

/// Patch stacking (or unstacking) every series; a merge patch replaces the whole array, so
/// each series keeps its other settings.
pub(crate) fn stack_patch(option: &Value, stacked: bool) -> Value {
    let all: Vec<Value> = series(option)
        .into_iter()
        .map(|mut s| {
            s.remove("stack");
            if stacked {
                s.insert("stack".into(), Value::String(STACK_GROUP.into()));
            }
            Value::Object(s)
        })
        .collect();
    json!({ "series": all })
}

/// Patch coloring series `index`, or handing it back to the palette.
pub(crate) fn color_patch(option: &Value, index: usize, color: Option<&str>) -> Value {
    let all: Vec<Value> = series(option)
        .into_iter()
        .enumerate()
        .map(|(i, mut s)| {
            if i == index {
                s.remove("itemStyle");
                if let Some(c) = color {
                    s.insert("itemStyle".into(), json!({ "color": c }));
                }
            }
            Value::Object(s)
        })
        .collect();
    json!({ "series": all })
}

/// Patch aligning column `column` of a table `width` columns wide; trailing `auto`s are
/// dropped (`null`: no entry left).
pub(crate) fn align_patch(option: &Value, width: usize, column: usize, align: ColumnAlign) -> Value {
    let current =
        |j: usize| option.pointer(&format!("/align/{j}")).and_then(Value::as_str).unwrap_or("auto").to_owned();
    let name = match align {
        ColumnAlign::Auto => "auto",
        ColumnAlign::Start => "start",
        ColumnAlign::Center => "center",
        ColumnAlign::End => "end",
    };
    let mut all: Vec<String> =
        (0..width.max(column + 1)).map(|j| if j == column { name.to_owned() } else { current(j) }).collect();
    while all.last().is_some_and(|a| a == "auto") {
        all.pop();
    }
    json!({ "align": if all.is_empty() { Value::Null } else { json!(all) } })
}

#[cfg(test)]
#[path = "data_edit_tests.rs"]
mod data_edit_tests;
