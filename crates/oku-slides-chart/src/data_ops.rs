//! `set_data` : modifications de `dataset.source` (graphiques et tableaux).
//!
//! Les index de ligne comptent la ligne d'en-têtes : la ligne 0 contient les
//! noms de colonnes, les données commencent à 1. Renommer une colonne est
//! donc un `set_cell` sur la ligne 0.
//!
//! Chaque ligne et chaque colonne porte un identifiant stable (`ids`,
//! `columns`), tiré par le cœur ; les ops les tiennent alignés sur `source`.

use std::collections::HashSet;

use oku_slides_i18n::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::canonical::{insert_column_series, remove_column_series, sync_series_with_columns};
use crate::error::{ChartError, DataOpSnafu, InconsistentSnafu, Result};
use crate::labels::ChartLabels;
use crate::model::normalize;
use crate::option::{Cell, ChartOption, Dataset};

/// Caractères au plus dans un identifiant de ligne ou de colonne.
const MAX_ID_CHARS: usize = 64;
/// Caractères hexadécimaux d'un identifiant tiré.
const NEW_ID_CHARS: usize = 8;

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
        /// Identifiant de la ligne. Ne pas fournir : le cœur en tire un.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    RemoveRow {
        at: usize,
    },
    InsertColumn {
        at: usize,
        cells: Vec<Cell>,
        /// Identifiant de la colonne. Ne pas fournir : le cœur en tire un.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    RemoveColumn {
        at: usize,
    },
    /// Remplace tout le tableau (collage depuis un tableur, réinitialisation).
    ///
    /// Les identifiants sont gardés par position ; l'excédent en reçoit de neufs.
    Replace {
        source: Vec<Vec<Cell>>,
    },
}

fn err(reason: Message) -> ChartError {
    DataOpSnafu { reason }.build().into()
}

/// Un identifiant admis : 1 à 64 lettres ASCII, chiffres, `-` ou `_`.
pub(crate) fn valid_id(id: &str) -> bool {
    (1..=MAX_ID_CHARS).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Un identifiant neuf de 8 caractères hexadécimaux, absent de `taken`.
fn new_id(taken: impl Fn(&str) -> bool) -> String {
    loop {
        let mut id = uuid::Uuid::new_v4().simple().to_string();
        id.truncate(NEW_ID_CHARS);
        if !taken(&id) {
            return id;
        }
    }
}

/// Recale `list` sur `len` : garde les identifiants valides et uniques, tire les autres.
fn fit(list: &mut Vec<String>, len: usize) {
    let mut seen = HashSet::with_capacity(len);
    let kept: Vec<Option<String>> = std::mem::take(list)
        .into_iter()
        .take(len)
        .map(|id| (valid_id(&id) && seen.insert(id.clone())).then_some(id))
        .collect();
    let missing = len.saturating_sub(kept.len());
    *list = kept
        .into_iter()
        .chain(std::iter::repeat_n(None, missing))
        .map(|id| {
            id.unwrap_or_else(|| {
                let id = new_id(|c| seen.contains(c));
                seen.insert(id.clone());
                id
            })
        })
        .collect();
}

/// Aligne `ids` sur les lignes et `columns` sur la ligne 0.
///
/// Garde les identifiants valides et uniques, tire les autres. Idempotente ; ne panique pas, quelle que soit l'incohérence de départ.
pub fn ensure_ids(d: &mut Dataset) {
    fit(&mut d.ids, d.source.len());
    fit(&mut d.columns, d.source.first().map_or(0, Vec::len));
}

/// Refuse des identifiants mal alignés, mal formés ou répétés.
///
/// Des listes vides sont admises : le cœur les remplira.
pub(crate) fn validate_ids(d: &Dataset) -> Result<()> {
    let width = d.source.first().map_or(0, Vec::len);
    let aligned =
        (d.ids.is_empty() || d.ids.len() == d.source.len()) && (d.columns.is_empty() || d.columns.len() == width);
    ensure!(aligned, InconsistentSnafu { reason: Message::new("chart-data-ids-length") });
    ensure!(
        d.ids.iter().chain(&d.columns).all(|id| valid_id(id)),
        InconsistentSnafu { reason: Message::new("chart-data-ids-invalid") }
    );
    let unique = |list: &[String]| {
        let mut seen = HashSet::with_capacity(list.len());
        list.iter().all(|id| seen.insert(id.as_str()))
    };
    ensure!(
        unique(&d.ids) && unique(&d.columns),
        InconsistentSnafu { reason: Message::new("chart-data-ids-duplicate") }
    );
    Ok(())
}

/// L'identifiant d'une ligne ou colonne insérée : `id` s'il est libre, sinon un neuf.
fn insert_id(list: &[String], id: Option<&str>) -> Result<String> {
    match id {
        None => Ok(new_id(|c| list.iter().any(|x| x == c))),
        Some(id) if !valid_id(id) => Err(err(Message::new("chart-data-ids-invalid"))),
        Some(id) if list.iter().any(|x| x == id) => Err(err(Message::new("chart-data-id-taken"))),
        Some(id) => Ok(id.to_owned()),
    }
}

impl DataOp {
    /// Applique l'opération, identifiants de lignes et de colonnes compris.
    ///
    /// Commence par [`ensure_ids`] : les accès à `ids` et `columns` sont donc dans les bornes.
    pub fn apply(&self, dataset: &mut Dataset) -> Result<()> {
        ensure_ids(dataset);
        let Dataset { source: src, ids, columns } = dataset;
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
                if let Some(c) = r.get_mut(*col) {
                    *c = value.clone();
                }
            }
            DataOp::InsertRow { at, cells, id } => {
                if *at == 0 || *at > src.len() || *at > ids.len() {
                    return Err(err(Message::new("chart-data-insert-row").arg("at", *at)));
                }
                let id = insert_id(ids, id.as_deref())?;
                let mut cells = cells.clone();
                cells.resize(width, Cell::Null);
                src.insert(*at, cells);
                ids.insert(*at, id);
            }
            DataOp::RemoveRow { at } => {
                if *at == 0 || *at >= src.len() || *at >= ids.len() {
                    return Err(err(Message::new("chart-data-remove-row").arg("at", *at)));
                }
                src.remove(*at);
                ids.remove(*at);
            }
            DataOp::InsertColumn { at, cells, id } => {
                if *at > width || *at > columns.len() {
                    return Err(err(Message::new("chart-data-insert-column").arg("at", *at)));
                }
                if cells.len() > src.len() {
                    return Err(err(Message::new("chart-data-too-many-cells")));
                }
                let id = insert_id(columns, id.as_deref())?;
                for (i, row) in src.iter_mut().enumerate() {
                    row.resize(width, Cell::Null);
                    row.insert(*at, cells.get(i).cloned().unwrap_or(Cell::Null));
                }
                // Sans ligne 0, il n'y a pas de colonne à identifier.
                if !src.is_empty() {
                    columns.insert(*at, id);
                }
            }
            DataOp::RemoveColumn { at } => {
                if *at >= width || width == 1 || *at >= columns.len() {
                    return Err(err(Message::new("chart-data-remove-column").arg("at", *at)));
                }
                for row in src.iter_mut() {
                    row.resize(width, Cell::Null);
                    row.remove(*at);
                }
                columns.remove(*at);
            }
            DataOp::Replace { source } => {
                if source.is_empty() {
                    return Err(err(Message::new("chart-data-keep-header")));
                }
                src.clone_from(source);
                // `ensure_ids` garde les identifiants par position et tire l'excédent.
                ensure_ids(dataset);
            }
        }
        Ok(())
    }
}

/// `set_data` sur un graphique : applique `change`, recale les séries, normalise.
pub fn chart_data(option: &ChartOption, change: &DataOp) -> Result<ChartOption> {
    let mut next = option.clone();
    let dataset = next.dataset.as_mut().ok_or_else(|| err(Message::new("chart-data-no-dataset")))?;
    change.apply(dataset)?;
    // Une colonne de valeurs emporte ou apporte sa propre série, pas la dernière.
    match change {
        DataOp::RemoveColumn { at } => remove_column_series(&mut next, *at),
        DataOp::InsertColumn { at, .. } => insert_column_series(&mut next, *at),
        _ => {}
    }
    sync_series_with_columns(&mut next);
    normalize(&next, ChartLabels::english())?;
    Ok(next)
}

#[cfg(test)]
#[path = "data_ops_tests.rs"]
mod data_ops_tests;
