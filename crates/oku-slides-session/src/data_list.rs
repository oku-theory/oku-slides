//! The data panel: the deck's data listed (`deck-data`), its actions, and one data's rows paged
//! in the data pane (`data-sheet`).

use oku_slides_a2ui::{Action, Component, ComponentKind, DataRef, DataRow, DataUser, Dynamic};
use oku_slides_core::{Deck, Op};
use oku_slides_data::limits::MAX_TEXT_BYTES;
use oku_slides_data::{ColumnType, DataCell, DataId, DataKind, NewColumn, NewRow, NewTable, TableOp, default_name};
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
use crate::{Result, Session, SessionErrorKind};

/// The surface of the deck's data list.
pub(crate) const DECK_DATA: &str = "deck-data";
/// The surface of one data's rows, in the data pane.
pub(crate) const DATA_SHEET: &str = "data-sheet";
/// The rows of a sheet page: a pane page, not a view, so the view cut does not apply.
const SHEET_ROWS: usize = 100;

/// An action of the data panel on the deck's data, one undo step each.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DataAction {
    /// Adds a data: one text column `a`, one empty row, a free default name.
    Add,
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
    /// The data the action names; `None` for `Add`.
    fn data(&self) -> Option<&str> {
        match self {
            Self::Add => None,
            Self::Rename { data, .. }
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

/// The deck's data, in id order, with their size and the views that show them.
pub(crate) fn data_list(deck: &Deck, l: &Localizer) -> Vec<DataRow> {
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
        .map(|(id, d)| DataRow {
            id: id.as_str().to_owned(),
            name: d.name.clone(),
            label: d.label.clone(),
            kind: kind_name(d.kind).to_owned(),
            columns: count(d.table.columns.len()),
            rows: count(d.table.rows.len()),
            users: users(id),
            error: None,
        })
        .collect()
}

/// The kind as the panel names it.
fn kind_name(kind: DataKind) -> &'static str {
    match kind {
        DataKind::Manual => "manual",
    }
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
    read: RowsRead,
    title: String,
    data: DataRef,
    types: Vec<String>,
    offset: usize,
    total: usize,
}

impl SheetRead {
    /// The `data-sheet` surface; `None` if the rows cannot be read. Blocking for a based data:
    /// call it with no session lock held.
    pub(crate) fn surface(self) -> Option<Surface> {
        let dataset = match self.read.run() {
            Ok(dataset) => dataset,
            Err(error) => {
                tracing::debug!(data = %self.data.id, %error, "no data sheet for this data");
                return None;
            }
        };
        let sheet = ComponentKind::DataSheet {
            data: self.data,
            source: source(&dataset),
            row_ids: row_ids(&dataset),
            column_ids: dataset.columns.clone(),
            types: self.types,
            offset: count(self.offset),
            total: count(self.total),
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
        let mut rows = data_list(self.doc.sources.deck(), &self.localizer);
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
    /// editable deck or for a data the deck does not have.
    pub(crate) fn data_sheet(&self, data: &str, offset: usize) -> Option<SheetRead> {
        self.ensure_editable().ok()?;
        let deck = self.doc.sources.deck();
        let Some((id, found)) = deck.data.get_key_value(data) else {
            tracing::debug!(%data, "no data sheet: the data is missing");
            return None;
        };
        let total = found.table.rows.len();
        let offset = offset.min(total);
        let rows = offset..offset.saturating_add(SHEET_ROWS).min(total);
        let read = match self.rows_read(found, None, rows, MAX_TEXT_BYTES) {
            Ok(read) => read,
            Err(error) => {
                tracing::debug!(%data, %error, "no data sheet for this data");
                return None;
            }
        };
        let types = found.table.columns.iter().map(|c| c.ty.as_str().to_owned()).collect();
        let users = deck.data_users(id).len();
        let title = found.label.clone().unwrap_or_else(|| found.name.clone());
        let data = DataRef { id: id.as_str().to_owned(), name: found.name.clone(), label: found.label.clone(), users };
        Some(SheetRead { read, title, data, types, offset, total })
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
                let op = Op::AddData { id: None, name, label: None, table: new_data_table() };
                self.apply_folding(&tr(self, "session-undo-data-add"), vec![op], None)
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
