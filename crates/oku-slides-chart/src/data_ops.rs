//! `set_data` : modifications de `dataset.source` (graphiques et tableaux), chacune avec son
//! inverse (pour l'historique du deck).
//!
//! Les index de ligne comptent la ligne d'en-têtes : la ligne 0 contient les
//! noms de colonnes, les données commencent à 1. Renommer une colonne est
//! donc un `set_cell` sur la ligne 0.

use oku_slides_i18n::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::canonical::sync_series_with_columns;
use crate::error::{ChartError, DataOpSnafu, Result};
use crate::labels::ChartLabels;
use crate::model::normalize;
use crate::option::{Cell, ChartOption, Dataset};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataOp {
    SetCell {
        row: usize,
        col: usize,
        value: Cell,
    },
    InsertRow {
        at: usize,
        cells: Vec<Cell>,
    },
    RemoveRow {
        at: usize,
    },
    InsertColumn {
        at: usize,
        cells: Vec<Cell>,
    },
    RemoveColumn {
        at: usize,
    },
    /// Remplace tout le tableau (collage depuis un tableur, réinitialisation).
    Replace {
        source: Vec<Vec<Cell>>,
    },
}

fn err(reason: Message) -> ChartError {
    DataOpSnafu { reason }.build().into()
}

impl DataOp {
    /// Applique l'opération et renvoie son inverse.
    pub fn apply(&self, dataset: &mut Dataset) -> Result<DataOp> {
        let src = &mut dataset.source;
        let width = src.first().map_or(0, Vec::len);
        match self {
            DataOp::SetCell { row, col, value } => {
                let r = src.get_mut(*row).ok_or_else(|| err(Message::new("chart-data-no-row").arg("row", *row)))?;
                if *col >= width {
                    return Err(err(Message::new("chart-data-no-column").arg("col", *col).arg("width", width)));
                }
                // Une ligne courte (JSON écrit à la main) est complétée.
                if r.len() <= *col {
                    r.resize(width, Cell::Null);
                }
                let previous = std::mem::replace(&mut r[*col], value.clone());
                Ok(DataOp::SetCell { row: *row, col: *col, value: previous })
            }
            DataOp::InsertRow { at, cells } => {
                if *at == 0 || *at > src.len() {
                    return Err(err(Message::new("chart-data-insert-row").arg("at", *at)));
                }
                let mut cells = cells.clone();
                cells.resize(width, Cell::Null);
                src.insert(*at, cells);
                Ok(DataOp::RemoveRow { at: *at })
            }
            DataOp::RemoveRow { at } => {
                if *at == 0 || *at >= src.len() {
                    return Err(err(Message::new("chart-data-remove-row").arg("at", *at)));
                }
                let cells = src.remove(*at);
                Ok(DataOp::InsertRow { at: *at, cells })
            }
            DataOp::InsertColumn { at, cells } => {
                if *at > width {
                    return Err(err(Message::new("chart-data-insert-column").arg("at", *at)));
                }
                if cells.len() > src.len() {
                    return Err(err(Message::new("chart-data-too-many-cells")));
                }
                for (i, row) in src.iter_mut().enumerate() {
                    row.resize(width, Cell::Null);
                    row.insert(*at, cells.get(i).cloned().unwrap_or(Cell::Null));
                }
                Ok(DataOp::RemoveColumn { at: *at })
            }
            DataOp::RemoveColumn { at } => {
                if *at >= width || width == 1 {
                    return Err(err(Message::new("chart-data-remove-column").arg("at", *at)));
                }
                let cells = src
                    .iter_mut()
                    .map(|row| {
                        row.resize(width, Cell::Null);
                        row.remove(*at)
                    })
                    .collect();
                Ok(DataOp::InsertColumn { at: *at, cells })
            }
            DataOp::Replace { source } => {
                if source.is_empty() {
                    return Err(err(Message::new("chart-data-keep-header")));
                }
                let previous = std::mem::replace(src, source.clone());
                Ok(DataOp::Replace { source: previous })
            }
        }
    }
}

/// `set_data` sur un graphique : applique `change`, recale les séries, normalise.
pub fn chart_data(option: &ChartOption, change: &DataOp) -> Result<ChartOption> {
    let mut next = option.clone();
    let dataset = next.dataset.as_mut().ok_or_else(|| err(Message::new("chart-data-no-dataset")))?;
    change.apply(dataset)?;
    sync_series_with_columns(&mut next);
    normalize(&next, ChartLabels::english())?;
    Ok(next)
}

#[cfg(test)]
#[path = "data_ops_tests.rs"]
mod data_ops_tests;
