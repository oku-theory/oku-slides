//! Modifications des données d'un graphique ou d'un tableau, envoyées par la surface
//! `host-data` : chacune devient une op, en une étape d'annulation nommée par la session.
//! Les patchs (empilement, couleur, alignement) sont calculés ici, depuis le deck.

use oku_slides_chart::{ColumnAlign, DataOp, SeriesType};
use oku_slides_core::{HostType, Op, Target};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snafu::OptionExt;

use crate::data_panel::{DataTarget, STACK_GROUP, hex_color, stackable};
use crate::error::{HostKeySnafu, NoChartSnafu, NoTableSnafu};
use crate::request::View;
use crate::{Result, Session};

/// An edit from the `host-data` surface, named by `edit`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataEdit {
    /// Changes the data (cell, row, column).
    Cells {
        /// The operation on the dataset, applied as is.
        change: DataOp,
    },
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
    /// Applique `edit` au graphique ou au tableau `element` de `slide`, en une étape d'annulation.
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
                let merge = match &change {
                    DataOp::SetCell { row, col, .. } => Some(format!("data:{slide}/{element}/{row}/{col}")),
                    _ => None,
                };
                let key = if chart { "session-undo-chart-data" } else { "session-undo-table-data" };
                let op = Op::SetData { target: owner, param: target.param, change };
                self.apply(&self.localizer.tr(key), vec![op], merge.as_deref())
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
                let op = Op::SetTableOption {
                    target: owner,
                    param: target.param,
                    patch: align_patch(&target.value, column, align),
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
        }
    }
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

/// Patch qui empile (ou non) toutes les séries ; un merge patch remplace le tableau entier,
/// chaque série garde ses autres réglages.
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

/// Patch qui colore la série `index`, ou la rend à la palette.
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

/// Patch qui aligne la colonne `column` ; les `auto` de fin sont retirés (`null` : plus d'entrée).
pub(crate) fn align_patch(option: &Value, column: usize, align: ColumnAlign) -> Value {
    let width = option.pointer("/dataset/source/0").and_then(Value::as_array).map_or(0, Vec::len);
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
