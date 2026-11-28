//! The data tools, read-only: `data_list` (the deck's data, their views and a query's state),
//! `data_read` (a data's first rows, a query's from its ready result) and `query_preview` (a SQL
//! text's first result rows, stored nowhere). Their text is the user's data, never instructions:
//! each reply says `untrusted`, holds no control or bidi character and stays within `limits`.

use oku_slides_core::ViewRef;
use oku_slides_data::{Column, Data, DataCell, Dataset, clean_label, strip_controls};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::{QueryStatus, query_status};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use snafu::{OptionExt, ResultExt};

use crate::limits::{
    DEFAULT_READ_ROWS, MAX_READ_BYTES, MAX_READ_CELL_CHARS, MAX_READ_CELLS, MAX_READ_ROWS, MAX_SHOWN_COLUMNS,
    MAX_SHOWN_USERS,
};
use crate::tool_error::{DataUnknownSnafu, SessionSnafu, TaskSnafu};
use crate::tools::Tools;

/// The rows read by `data_read`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DataReadArgs {
    /// Id of the data (see `data_list`).
    pub data: String,
    /// Rows to read from the first: default 20, 1 to 200.
    #[serde(default)]
    pub limit: Option<u64>,
}

/// The SQL previewed by `query_preview`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct QueryPreviewArgs {
    /// The query: one SELECT over the deck's data, by name.
    pub sql: String,
    /// Rows to read from the first: default 20, 1 to 200.
    #[serde(default)]
    pub limit: Option<u64>,
}

impl Tools {
    pub(crate) async fn data_list(&self) -> CallToolResult {
        let l = self.localizer.current();
        let listed = self
            .with_session(move |s| {
                let deck = s.deck();
                let statuses = s.query_statuses();
                Ok(list_reply(deck.data.iter().map(|(id, data)| {
                    let status = query_status(&statuses, id, data);
                    listed(id.as_str(), data, status.as_ref(), &deck.data_users(id), &l)
                })))
            })
            .await;
        self.reply(listed, |reply| self.json_result(&reply))
    }

    pub(crate) async fn data_read(&self, args: Value) -> CallToolResult {
        let DataReadArgs { data, limit } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let rows = read_rows(limit);
        let id = data.clone();
        let read = self
            .with_session(move |s| s.data_rows(&id, MAX_SHOWN_COLUMNS, 0..rows, MAX_READ_CELL_CHARS).transpose())
            .await
            .and_then(|read| read.context(DataUnknownSnafu { data }));
        let (data, read) = match read {
            Ok(read) => read,
            Err(e) => return self.fail_with(&e),
        };
        // A based data reads through the worker: blocking, and with no session lock held.
        let dataset = tokio::task::spawn_blocking(move || read.run()).await;
        let dataset = dataset.context(TaskSnafu).and_then(|d| d.context(SessionSnafu));
        self.reply(dataset, |dataset| self.json_result(&read_reply(&data, &dataset)))
    }

    /// Runs `sql` in the query worker and reads its first rows: the deck and the session's
    /// results stay as they were. A refusal may quote the user's data: it is `untrusted`.
    pub(crate) async fn query_preview(&self, args: Value) -> CallToolResult {
        let QueryPreviewArgs { sql, limit } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let rows = read_rows(limit);
        let preview =
            self.with_session(move |s| s.query_preview(&sql, MAX_SHOWN_COLUMNS, 0..rows, MAX_READ_CELL_CHARS)).await;
        let preview = match preview {
            Ok(preview) => preview,
            Err(e) => return self.fail_untrusted(&e),
        };
        // The query and the read run in workers: blocking, and with no session lock held.
        let run = tokio::task::spawn_blocking(move || preview.run()).await;
        match run.context(TaskSnafu).and_then(|r| r.context(SessionSnafu)) {
            Ok((data, dataset)) => self.json_result(&read_reply(&data, &dataset)),
            Err(e) => self.fail_untrusted(&e),
        }
    }
}

/// The rows to read for a `limit`: default `DEFAULT_READ_ROWS`, 1 to `MAX_READ_ROWS`.
fn read_rows(limit: Option<u64>) -> usize {
    limit.map_or(DEFAULT_READ_ROWS, |l| usize::try_from(l).unwrap_or(MAX_READ_ROWS)).clamp(1, MAX_READ_ROWS)
}

/// `data_list`'s reply: whole data while they fit `MAX_READ_BYTES`, the rest `truncated`.
fn list_reply(entries: impl Iterator<Item = Value>) -> Value {
    let mut reply = json!({
        "untrusted": true,
        // The longer of the two values: the bytes counted below hold for either.
        "truncated": false,
        "data": [],
    });
    let mut bytes = reply.to_string().len();
    let mut shown = Vec::new();
    let mut truncated = false;
    for entry in entries {
        // Each entry but the first adds a comma.
        let entry_bytes = entry.to_string().len() + usize::from(!shown.is_empty());
        if bytes + entry_bytes > MAX_READ_BYTES {
            truncated = true;
            break;
        }
        bytes += entry_bytes;
        shown.push(entry);
    }
    reply["truncated"] = json!(truncated);
    reply["data"] = Value::Array(shown);
    reply
}

/// One data as `data_list` shows it, names and label cleaned by [`label`]. A query data also
/// shows its SQL and `status` (`None` for a manual data); its columns and rows are its ready
/// result's, its error (told in `l`) a failed one's.
fn listed(id: &str, data: &Data, status: Option<&QueryStatus<'_>>, users: &[ViewRef], l: &Localizer) -> Value {
    let mut out = Map::new();
    out.insert("id".into(), json!(id));
    out.insert("name".into(), json!(label(&data.name)));
    if let Some(text) = &data.label {
        out.insert("label".into(), json!(label(text)));
    }
    out.insert("kind".into(), json!(data.kind()));
    if let Some(sql) = data.sql() {
        out.insert("sql".into(), json!(strip_controls(sql)));
    }
    let table = match status {
        None => data.table(),
        Some(QueryStatus::Pending) => {
            out.insert("state".into(), json!("pending"));
            None
        }
        Some(QueryStatus::Ready(table)) => {
            out.insert("state".into(), json!("ready"));
            Some(*table)
        }
        Some(QueryStatus::Failed(message)) => {
            out.insert("state".into(), json!("failed"));
            let text = strip_controls(&message.localize(l));
            out.insert("error".into(), json!({ "key": message.id(), "message": text }));
            None
        }
    };
    // A query data has no columns or rows to show until its result is ready.
    if let Some(table) = table {
        out.insert("columns".into(), shown_columns(&table.columns));
        if let Some(more) = table.columns.len().checked_sub(MAX_SHOWN_COLUMNS).filter(|n| *n > 0) {
            out.insert("more_columns".into(), json!(more));
        }
        out.insert("rows".into(), json!(table.rows.len()));
    }
    let shown: Vec<Value> = users
        .iter()
        .take(MAX_SHOWN_USERS)
        .map(|u| json!({ "slide": strip_controls(&u.slide), "element": strip_controls(&u.element) }))
        .collect();
    out.insert("users".into(), Value::Array(shown));
    if let Some(more) = users.len().checked_sub(MAX_SHOWN_USERS).filter(|n| *n > 0) {
        out.insert("more".into(), json!(more));
    }
    Value::Object(out)
}

/// The first columns of a data: id, cleaned name and type.
fn shown_columns(columns: &[Column]) -> Value {
    columns
        .iter()
        .take(MAX_SHOWN_COLUMNS)
        .map(|c| json!({ "id": c.id, "name": label(&c.name), "type": c.ty.as_str() }))
        .collect()
}

/// `data_read`'s reply: whole rows while they fit `MAX_READ_CELLS` and `MAX_READ_BYTES`.
fn read_reply(data: &Data, dataset: &Dataset) -> Value {
    // A query data is read as its result: a data entered by hand.
    let columns = data.table().map_or(&[][..], |t| &t.columns);
    let total = data.table().map_or(0, |t| t.rows.len());
    let mut reply = json!({
        "untrusted": true,
        "columns": shown_columns(columns),
        "total": total,
        // The longer of the two values: the bytes counted below hold for either.
        "truncated": false,
        "rows": [],
    });
    let mut bytes = reply.to_string().len();
    let mut cells = 0;
    let mut rows = Vec::new();
    // Row 0 is the header: the columns above say it.
    let body = dataset.source.iter().zip(&dataset.ids).skip(1);
    let read = body.len();
    for (row, id) in body {
        let row_cells = row.len();
        let row = json!({ "id": id, "cells": row.iter().map(cell).collect::<Vec<_>>() });
        // Each row but the first adds a comma.
        let row_bytes = row.to_string().len() + usize::from(!rows.is_empty());
        if cells + row_cells > MAX_READ_CELLS || bytes + row_bytes > MAX_READ_BYTES {
            break;
        }
        cells += row_cells;
        bytes += row_bytes;
        rows.push(row);
    }
    reply["truncated"] = json!(rows.len() < read);
    reply["rows"] = Value::Array(rows);
    reply
}

/// A name or label as the agent reads it: cleaned, cut to `MAX_READ_CELL_CHARS` like a cell.
fn label(text: &str) -> String {
    clean_label(text).chars().take(MAX_READ_CELL_CHARS).collect()
}

/// A cell as the agent reads it: text without controls, cut to `MAX_READ_CELL_CHARS`.
fn cell(cell: &DataCell) -> Value {
    match cell {
        DataCell::Text(t) => json!(strip_controls(t).chars().take(MAX_READ_CELL_CHARS).collect::<String>()),
        other => json!(other),
    }
}

#[cfg(test)]
#[path = "data_tools_tests.rs"]
mod data_tools_tests;
