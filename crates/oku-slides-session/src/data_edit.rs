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

/// Une modification de la surface `host-data`, nommée par `edit`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataEdit {
    /// Change les données (cellule, ligne, colonne).
    Cells { change: DataOp },
    /// Change le type du graphique ; ses données sont converties.
    Kind { kind: SeriesType },
    /// Empile toutes les séries, ou les désempile.
    Stack { stacked: bool },
    /// Colore la série `index`, ou la rend à la palette (`null`).
    Color { index: usize, color: Option<String> },
    /// Allume ou éteint une option d'affichage d'un tableau.
    Flag { flag: TableFlag, on: bool },
    /// Aligne la colonne `column` d'un tableau.
    Align { column: usize, align: ColumnAlign },
    /// Montre les mêmes données en graphique ou en tableau.
    ShowAs { view: View },
}

/// Option d'affichage d'un tableau, nommée comme dans ses options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TableFlag {
    /// Ligne 0 en en-tête.
    HeaderRow,
    /// Colonne 0 en en-tête.
    HeaderColumn,
    /// Une ligne sur deux sur fond.
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
