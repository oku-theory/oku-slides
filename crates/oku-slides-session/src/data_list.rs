//! The data panel: the deck's data listed (`deck-data`), its actions, and one data's rows paged
//! in the data pane (`data-sheet`). A query data shows its result's state ([`QueryStatus`]); its
//! sheet, its SQL, error and inputs over its ready result, read-only.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_a2ui::{
    Action, Component, ComponentKind, DataChoice, DataRef, DataRow, DataState, DataUser, Dynamic, QueryView,
};
use oku_slides_core::{Deck, Op};
use oku_slides_data::limits::MAX_TEXT_BYTES;
use oku_slides_data::sql::parse_query;
use oku_slides_data::{ColumnType, Data, DataCell, DataId, NewColumn, NewRow, NewTable, TableOp, default_name};
use oku_slides_i18n::{Localize, Localizer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::forms::{data_surface, surface};
use crate::agent::{Place, Surface};
use crate::data_edit::Followers;
use crate::data_panel::{DATA_EVENT, data_missing, row_ids, source};
use crate::data_paste::Paste;
use crate::data_read::RowsRead;
use crate::derived::{query_status, result_data, status_of_query};
use crate::{QueryStatus, Result, Session, SessionErrorKind};

/// The surface of the deck's data list.
pub(crate) const DECK_DATA: &str = "deck-data";
/// The surface of one data's rows, in the data pane.
pub(crate) const DATA_SHEET: &str = "data-sheet";
/// The rows of a sheet page: a pane page, not a view, so the view cut does not apply.
const SHEET_ROWS: usize = 100;
/// The SQL of a query added from the panel, before the user writes theirs.
const NEW_QUERY_SQL: &str = "SELECT 1 AS a";

/// An action of the data panel on the deck's data, one undo step each.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DataAction {
    /// Adds a data: one text column `a`, one empty row, a free default name.
    Add,
    /// Adds a query data reading no data (`SELECT 1 AS a`), named `name` or a free default name.
    AddQuery {
        /// Its name; absent: `query`, `query_2`…
        #[serde(default)]
        name: Option<String>,
    },
    /// Replaces a query data's SQL; refused if it does not parse, reads no data or a cycle.
    SetQuery {
        /// The data's id.
        data: String,
        /// The new SQL.
        sql: String,
    },
    /// Renames a data.
    Rename {
        /// The data's id.
        data: String,
        /// Its new name.
        name: String,
    },
    /// Removes a data no view shows.
    Remove {
        /// The data's id.
        data: String,
    },
    /// Changes the type of a column; refused if a cell does not convert.
    SetType {
        /// The data's id.
        data: String,
        /// The column's id.
        column: String,
        /// The new type.
        #[serde(rename = "type")]
        ty: ColumnType,
    },
    /// Changes a data's cells, rows or columns, by id.
    Cells {
        /// The data's id.
        data: String,
        /// The edit of its table.
        change: TableOp,
    },
    /// Adds an empty row after a data's last row.
    AddRow {
        /// The data's id.
        data: String,
    },
    /// Pastes a block at a cell, by row and column id: the cells it covers are set, and the
    /// rows and columns past the data's last ones are added.
    Paste {
        /// The data's id.
        data: String,
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
}

impl DataAction {
    /// The data the action names; `None` for `Add` and `AddQuery`.
    fn data(&self) -> Option<&str> {
        match self {
            Self::Add | Self::AddQuery { .. } => None,
            Self::SetQuery { data, .. }
            | Self::Rename { data, .. }
            | Self::Remove { data }
            | Self::SetType { data, .. }
            | Self::Cells { data, .. }
            | Self::AddRow { data }
            | Self::Paste { data, .. } => Some(data),
        }
    }
}

/// A refused action, shown on its data's row until the next edit.
#[derive(Debug)]
pub(crate) struct DataRefusal {
    /// The edit count when it was refused.
    seq: u64,
    data: String,
    text: String,
}

/// The deck's data, in id order, with their size and the views that show them; a query data
/// with its state in `statuses`, sized as its ready result.
pub(crate) fn data_list(deck: &Deck, statuses: &BTreeMap<DataId, QueryStatus<'_>>, l: &Localizer) -> Vec<DataRow> {
    let number = |slide: &str| deck.slides.iter().position(|s| s.id == slide).map_or(0, |i| i + 1);
    let users = |id| {
        deck.data_users(id)
            .into_iter()
            .map(|v| {
                let args = [("slide", number(&v.slide).into()), ("element", v.element.as_str().into())];
                let label = l.tr_with("session-data-user", &args);
                DataUser { slide: v.slide, element: v.element, label }
            })
            .collect()
    };
    deck.data
        .iter()
        .map(|(id, d)| {
            let status = query_status(statuses, id, d);
            // A query data's columns and rows are known once its result is ready.
            let table = match &status {
                None => d.table(),
                Some(QueryStatus::Ready(table)) => Some(*table),
                Some(QueryStatus::Pending | QueryStatus::Failed(_)) => None,
            };
            DataRow {
                id: id.as_str().to_owned(),
                name: d.name.clone(),
                label: d.label.clone(),
                kind: d.kind().as_str().to_owned(),
                columns: count(table.map_or(0, |t| t.columns.len())),
                rows: count(table.map_or(0, |t| t.rows.len())),
                users: users(id),
                state: status.as_ref().map(state),
                error: None,
            }
        })
        .collect()
}

/// A query's result state, as the panel shows it.
fn state(status: &QueryStatus<'_>) -> DataState {
    match status {
        QueryStatus::Pending => DataState::Pending,
        QueryStatus::Ready(_) => DataState::Ready,
        QueryStatus::Failed(_) => DataState::Failed,
    }
}

/// The deck's data `sql` reads, by name; a name no single data has is left to the query's error.
fn query_inputs(deck: &Deck, sql: &str) -> Vec<DataChoice> {
    let Ok(parsed) = parse_query(sql) else { return Vec::new() };
    parsed
        .inputs()
        .into_iter()
        .filter_map(|name| {
            let mut named = deck.data.iter().filter(|(_, d)| d.name.eq_ignore_ascii_case(&name));
            match (named.next(), named.next()) {
                (Some((id, d)), None) => {
                    Some(DataChoice { id: id.as_str().to_owned(), name: d.name.clone(), label: d.label.clone() })
                }
                _ => None,
            }
        })
        .collect()
}

/// `n` as a count of the panel; a table holds far fewer than `u32::MAX` rows or columns.
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The action of the data panel's components: `data`, with the change written at `/edit`.
fn panel_action() -> Action {
    Action::event(DATA_EVENT, [("edit", Dynamic::path("/edit"))])
}

/// `data` as an id, refused as a missing data if it is not one.
fn data_id(data: &str) -> Result<DataId> {
    DataId::try_from(data.to_owned()).map_err(|_| data_missing(data))
}

/// A page of a data's rows taken from the session, read with no lock held.
#[derive(Debug)]
pub(crate) struct SheetRead {
    /// The rows to read; `None` for a query without a ready result.
    read: Option<RowsRead>,
    title: String,
    data: DataRef,
    types: Vec<String>,
    offset: usize,
    total: usize,
    query: Option<QueryView>,
    /// The refusal of the panel's last action on this data.
    error: Option<String>,
}

impl SheetRead {
    /// The `data-sheet` surface; `None` if the rows cannot be read. Blocking for a based data:
    /// call it with no session lock held.
    pub(crate) fn surface(self) -> Option<Surface> {
        let (source, row_ids, column_ids) = match self.read.map(RowsRead::run) {
            // A query without a ready result has no rows to show: its state says why.
            None => (vec![vec![]], Vec::new(), Vec::new()),
            Some(Ok(dataset)) => (source(&dataset), row_ids(&dataset), dataset.columns),
            Some(Err(error)) => {
                tracing::debug!(data = %self.data.id, %error, "no data sheet for this data");
                return None;
            }
        };
        let sheet = ComponentKind::DataSheet {
            data: self.data,
            source,
            row_ids,
            column_ids,
            types: self.types,
            offset: count(self.offset),
            total: count(self.total),
            query: self.query,
            error: self.error,
            edit: Dynamic::path("/edit"),
            action: panel_action(),
        };
        Some(data_surface(DATA_SHEET, self.title, sheet))
    }
}

impl Session {
    /// The `deck-data` surface; `None` outside an editable deck.
    pub(crate) fn data_list_surface(&self) -> Option<Surface> {
        self.ensure_editable().ok()?;
        let mut rows = data_list(self.doc.sources.deck(), &self.query_statuses(), &self.localizer);
        if let Some(refusal) = self.data_refusal.as_ref().filter(|r| r.seq == self.edit_seq)
            && let Some(row) = rows.iter_mut().find(|r| r.id == refusal.data)
        {
            row.error = Some(refusal.text.clone());
        }
        let list = ComponentKind::DataList { rows, edit: Dynamic::path("/edit"), action: panel_action() };
        let components = vec![Component::new("root", list)];
        Some(surface(DECK_DATA, Place::Thread, None, components, json!({ "edit": null })))
    }

    /// The page of `data` from row `offset`, to read with no lock held; `None` outside an
    /// editable deck or for a data the deck does not have. A query data's page is its ready
    /// result's, none while pending or failed.
    pub(crate) fn data_sheet(&self, data: &str, offset: usize) -> Option<SheetRead> {
        self.ensure_editable().ok()?;
        let deck = self.doc.sources.deck();
        let Some((id, found)) = deck.data.get_key_value(data) else {
            tracing::debug!(%data, "no data sheet: the data is missing");
            return None;
        };
        let (shown, query) = match found.sql() {
            None => (Some(Arc::clone(found)), None),
            Some(sql) => {
                let (shown, view) = self.query_page(id, found, sql);
                (shown, Some(view))
            }
        };
        let table = shown.as_deref().and_then(|d| d.table());
        let total = table.map_or(0, |t| t.rows.len());
        let offset = offset.min(total);
        let rows = offset..offset.saturating_add(SHEET_ROWS).min(total);
        let read = match (&shown, &query) {
            (None, _) => None,
            // A ready result's base is the session's, not the deck's: the read worker reads it.
            (Some(result), Some(_)) => Some(self.result_read(id, Arc::clone(result), None, rows, MAX_TEXT_BYTES)),
            (Some(manual), None) => match self.rows_read(id, manual, None, rows, MAX_TEXT_BYTES) {
                Ok(read) => Some(read),
                Err(error) => {
                    tracing::debug!(%data, %error, "no data sheet for this data");
                    return None;
                }
            },
        };
        let types = table.map_or(&[][..], |t| &t.columns).iter().map(|c| c.ty.as_str().to_owned()).collect();
        let users = deck.data_users(id).len();
        let title = found.label.clone().unwrap_or_else(|| found.name.clone());
        let data = DataRef { id: id.as_str().to_owned(), name: found.name.clone(), label: found.label.clone(), users };
        let error = self.data_refusal.as_ref().filter(|r| r.seq == self.edit_seq && r.data == id.as_str());
        let error = error.map(|r| r.text.clone());
        Some(SheetRead { read, title, data, types, offset, total, query, error })
    }

    /// The ready result of query data `id` (`found`, running `sql`), none while pending or
    /// failed, and its query's SQL, state, error and inputs.
    pub(crate) fn query_page(&self, id: &DataId, found: &Data, sql: &str) -> (Option<Arc<Data>>, QueryView) {
        let status = status_of_query(&self.query_statuses(), id, &found.name);
        let error = match &status {
            QueryStatus::Failed(message) => Some(message.localize(&self.localizer)),
            QueryStatus::Pending | QueryStatus::Ready(_) => None,
        };
        let shown = match &status {
            QueryStatus::Ready(table) => Some(result_data(found, table)),
            QueryStatus::Pending | QueryStatus::Failed(_) => None,
        };
        let deck = self.doc.sources.deck();
        (shown, QueryView { sql: sql.to_owned(), state: state(&status), error, inputs: query_inputs(deck, sql) })
    }

    /// Applies `action`, one undo step; a refusal naming a data shows on its row until the next edit.
    pub(crate) fn data_action(&mut self, action: DataAction) -> Result<()> {
        let named = action.data().map(str::to_owned);
        let done = self.run_data_action(action);
        if let (Err(e), Some(data)) = (&done, named) {
            // A due base read is retried, not refused.
            if e.kind() != SessionErrorKind::BaseStale {
                let text = e.localize(&self.localizer);
                self.data_refusal = Some(DataRefusal { seq: self.edit_seq, data, text });
            }
        }
        done
    }

    fn run_data_action(&mut self, action: DataAction) -> Result<()> {
        let tr = |s: &Self, key: &str| s.localizer.tr(key);
        match action {
            DataAction::Add => {
                let deck = self.doc.sources.deck();
                let names: Vec<String> = deck.data.values().map(|d| d.name.to_lowercase()).collect();
                let (name, _) = default_name("data", 2, |n| names.contains(&n.to_lowercase()));
                let op = Op::AddData { id: None, name, label: None, table: Some(new_data_table()), sql: None };
                self.apply_folding(&tr(self, "session-undo-data-add"), vec![op], None)
            }
            DataAction::AddQuery { name } => {
                let name = name.unwrap_or_else(|| {
                    let deck = self.doc.sources.deck();
                    let names: Vec<String> = deck.data.values().map(|d| d.name.to_lowercase()).collect();
                    default_name("query", 2, |n| names.contains(&n.to_lowercase())).0
                });
                let sql = Some(NEW_QUERY_SQL.to_owned());
                let op = Op::AddData { id: None, name, label: None, table: None, sql };
                self.apply_folding(&tr(self, "session-undo-data-add"), vec![op], None)
            }
            DataAction::SetQuery { data, sql } => {
                let op = Op::SetQuery { data: data_id(&data)?, sql };
                self.apply_folding(&tr(self, "session-undo-data-query"), vec![op], None)
            }
            DataAction::Rename { data, name } => {
                let op = Op::RenameData { data: data_id(&data)?, name };
                self.apply_folding(&tr(self, "session-undo-data-rename"), vec![op], None)
            }
            DataAction::Remove { data } => {
                let op = Op::RemoveData { data: data_id(&data)? };
                self.apply_folding(&tr(self, "session-undo-data-remove"), vec![op], None)
            }
            DataAction::SetType { data, column, ty } => {
                let op = Op::SetColumnType { data: data_id(&data)?, column, ty };
                self.apply_folding(&tr(self, "session-undo-data-type"), vec![op], None)
            }
            DataAction::Cells { data, change } => {
                let (ops, merge) = self.grid_ops(&data_id(&data)?, change, Followers::Every)?;
                self.apply_folding(&tr(self, "session-undo-data-edit"), ops, merge.as_deref())
            }
            DataAction::AddRow { data } => {
                let op = self.add_row_op(&data_id(&data)?)?;
                self.apply_folding(&tr(self, "session-undo-data-edit"), vec![op], None)
            }
            DataAction::Paste { data, row, column, cells, names } => {
                let ops = self.sheet_paste_ops(&data_id(&data)?, Paste { row, column, cells, names })?;
                self.apply_folding(&tr(self, "session-undo-data-edit"), ops, None)
            }
        }
    }
}

/// The table of a new data: one text column `a`, one empty row.
#[expect(
    clippy::default_trait_access,
    reason = "the overlay's map type is imbl's, a crate the session does not depend on"
)]
fn new_data_table() -> NewTable {
    let column = NewColumn { id: None, name: "a".to_owned(), ty: Some(ColumnType::Text) };
    NewTable {
        generation: None,
        columns: vec![column],
        rows: vec![NewRow { id: None, cells: Vec::new() }],
        base: None,
        overlay: Default::default(),
    }
}

#[cfg(test)]
#[path = "data_list_tests.rs"]
mod data_list_tests;
