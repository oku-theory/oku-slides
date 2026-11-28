//! The data ops: add, edit, bind, remove, rename and retype a data.
//!
//! Each checks first and writes last, so a refused op leaves the deck intact.
//! Ids come filled in by `prepare_ops`: an id left out is refused here.

use std::collections::HashSet;
use std::sync::Arc;

use oku_slides_chart::ChartOption;
use oku_slides_data::sql::{Query, parse_query};
use oku_slides_data::{
    Column, ColumnType, Data, DataError, DataId, NewTable, Source, TableOp, clean_label, valid_id, valid_name,
};
use serde_json::{Map, Value};

use super::query::{put_query, restore};
use super::{Target, declared, host, target_mut, target_ref, widget};
use crate::error::{DataFault, OpError, ParamTypeSnafu, Shape};
use crate::model::Deck;
use crate::params::ParamValue;
use crate::query_order::Reads;
use crate::widget::{HostType, ParamType, WidgetRegistry};

/// What a new data holds: a table entered by hand, or a query.
pub(super) enum NewSource {
    Table(NewTable),
    Query(String),
}

impl NewSource {
    /// The source `add_data` gives: exactly one of `table` and `sql`.
    pub(super) fn of(table: Option<NewTable>, sql: Option<String>) -> Result<Self, OpError> {
        match (table, sql) {
            (Some(table), None) => Ok(NewSource::Table(table)),
            (None, Some(sql)) => Ok(NewSource::Query(sql)),
            _ => Err(DataFault::Source.into()),
        }
    }
}

/// `add_data`: a free id and name, a valid table or query; the deck bounds are checked by `apply_all`.
pub(super) fn add_data(
    deck: &mut Deck,
    id: Option<DataId>,
    name: String,
    label: Option<&str>,
    source: NewSource,
) -> Result<(), OpError> {
    let id = id.ok_or_else(DataError::id_missing)?;
    if deck.data.contains_key(&id) {
        return Err(DataError::id_taken(id.as_str()).into());
    }
    let lower = name.to_lowercase();
    if deck.data.values().any(|d| d.name.to_lowercase() == lower) {
        return Err(DataFault::NameTaken { name }.into());
    }
    if !valid_name(&name) {
        return Err(DataError::invalid_name(&name).into());
    }
    let label = label.map(clean_label);
    match source {
        NewSource::Table(table) => {
            let (table, _) = table.into_table()?;
            deck.data.insert(id, Arc::new(Data { name, label, inline: false, source: Source::Manual(table) }));
            Ok(())
        }
        NewSource::Query(sql) => {
            let query = parse_query(&sql)?;
            let entry = Arc::new(Data { name, label, inline: false, source: Source::Query { sql } });
            put_query(deck, &id, entry, &query)
        }
    }
}

/// `edit_data`: an existing, whole data; a projected column is never removed.
pub(super) fn edit_data(deck: &mut Deck, data: &DataId, change: &TableOp) -> Result<(), OpError> {
    manual(deck, data)?;
    let replace = matches!(change, TableOp::Replace { .. });
    if !replace && deck.truncated.contains(data) {
        return Err(DataFault::Truncated { data: data.as_str().to_owned() }.into());
    }
    if let TableOp::RemoveColumn { column } = change {
        let views: Vec<String> = deck
            .data_users(data)
            .into_iter()
            .filter(|v| v.columns.as_ref().is_some_and(|c| c.contains(column)))
            .map(|v| v.element)
            .collect();
        if !views.is_empty() {
            return Err(DataFault::ColumnInUse { column: column.clone(), views }.into());
        }
    }
    let Some(entry) = deck.data.get_mut(data) else {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    };
    // `Table::apply` leaves the table unchanged when it refuses the edit.
    let d = Arc::make_mut(entry);
    d.table_mut().ok_or_else(|| OpError::data_derived(data.as_str()))?.apply(change)?;
    d.inline = false;
    if replace {
        deck.truncated.remove(data);
    }
    Ok(())
}

/// Views, and reading queries, a refusal names before counting the others.
const NAMED_VIEWS: usize = 10;

/// `remove_data`: a data no view shows and no other query reads; a truncated one too, and its
/// mark goes with it.
pub(super) fn remove_data(deck: &mut Deck, data: &DataId) -> Result<(), OpError> {
    if !deck.data.contains_key(data) {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    }
    let users = deck.data_users(data);
    let readers: Vec<String> = deck
        .data_readers(data)
        .into_iter()
        .filter(|reader| reader != data)
        .filter_map(|reader| deck.data.get(&reader).map(|d| d.name.clone()))
        .collect();
    if !users.is_empty() || !readers.is_empty() {
        let more = users.len().saturating_sub(NAMED_VIEWS) + readers.len().saturating_sub(NAMED_VIEWS);
        let views = users.into_iter().take(NAMED_VIEWS).map(|v| v.element).collect();
        let queries = readers.into_iter().take(NAMED_VIEWS).collect();
        return Err(DataFault::InUse { views, queries, more }.into());
    }
    deck.data.remove(data);
    deck.truncated.remove(data);
    Ok(())
}

/// `rename_data`: a valid name no other data has (case ignored); the data stops being inline.
///
/// Every query reading the data is rewritten to read the new name, in the same op. Refused
/// when a rewritten query no longer parses (too long, say), when a reader already gives the
/// new name to one of its own relations (rewritten, it would read that relation), when the
/// old name is also another data's and queries read it (a merge may leave that), or when the
/// new name closes a cycle a merge left open (a query reading that name).
pub(super) fn rename_data(deck: &mut Deck, data: &DataId, name: String) -> Result<(), OpError> {
    whole(deck, data)?;
    let lower = name.to_lowercase();
    if deck.data.iter().any(|(id, d)| id != data && d.name.to_lowercase() == lower) {
        return Err(DataFault::NameTaken { name }.into());
    }
    if !valid_name(&name) {
        return Err(DataError::invalid_name(&name).into());
    }
    let Some(current) = deck.data.get(data) else {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    };
    let old = current.name.clone();
    let readers = deck.data_readers(data);
    let old_lower = old.to_ascii_lowercase();
    if !readers.is_empty() && deck.data.iter().any(|(id, d)| id != data && d.name.to_ascii_lowercase() == old_lower) {
        return Err(DataFault::NameAmbiguous { name: old }.into());
    }
    let mut renamed = (**current).clone();
    renamed.name.clone_from(&name);
    renamed.inline = false;
    let mut writes = Vec::new();
    for reader in readers {
        // A reader is a query data whose query parses: `data_readers` reads only those.
        let Some(entry) = deck.data.get(&reader) else { continue };
        let Some(sql) = entry.sql() else { continue };
        let query = parse_query(sql)?;
        let rewritten = query.renamed(&old, &name);
        if captures(&query, &rewritten, &old_lower, &name) {
            return Err(DataFault::RenameCaptured { reader: entry.name.clone(), name }.into());
        }
        let sql = rewritten.to_sql();
        parse_query(&sql)?;
        if reader == *data {
            renamed.source = Source::Query { sql };
        } else if let Some(entry) = deck.data.get(&reader) {
            writes.push((reader, Arc::new(Data { source: Source::Query { sql }, ..(**entry).clone() })));
        }
    }
    let was_on_cycle = Reads::new(deck).on_cycle(data);
    writes.push((data.clone(), Arc::new(renamed)));
    let previous: Vec<(DataId, Option<Arc<Data>>)> =
        writes.into_iter().map(|(id, entry)| (id.clone(), deck.data.insert(id, entry))).collect();
    if !was_on_cycle && Reads::new(deck).on_cycle(data) {
        for (id, entry) in previous {
            restore(deck, &id, entry);
        }
        return Err(DataFault::Cycle { data: name }.into());
    }
    Ok(())
}

/// Whether renaming `old` (lowercased) to `new` in `query`, giving `rewritten`, changes what
/// it reads beyond the rename: a relation of its own named `new` would take the reads.
fn captures(query: &Query, rewritten: &Query, old: &str, new: &str) -> bool {
    let new = new.to_ascii_lowercase();
    let mut expected = query.inputs();
    expected.remove(old);
    expected.insert(new.clone());
    rewritten.inputs() != expected || query.bound_names().contains(&new)
}

/// `set_column_type`: converts the overlay cells without loss; the data stops being inline.
///
/// With a base, the session has the worker check its cells first.
pub(super) fn set_column_type(deck: &mut Deck, data: &DataId, column: &str, ty: ColumnType) -> Result<(), OpError> {
    manual(deck, data)?;
    whole(deck, data)?;
    let Some(entry) = deck.data.get_mut(data) else {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    };
    // `Table::set_column_type` leaves the table unchanged when it refuses.
    let d = Arc::make_mut(entry);
    d.table_mut().ok_or_else(|| OpError::data_derived(data.as_str()))?.set_column_type(column, ty)?;
    d.inline = false;
    Ok(())
}

/// Refuses a missing data, or one derived from a query: its rows are never edited by hand.
fn manual(deck: &Deck, data: &DataId) -> Result<(), OpError> {
    match deck.data.get(data) {
        None => Err(DataFault::Missing { data: data.as_str().to_owned() }.into()),
        Some(d) if d.table().is_none() => Err(OpError::data_derived(data.as_str())),
        Some(_) => Ok(()),
    }
}

/// The columns of `data`: none for a query data until its result is known.
fn columns_of(data: &Data) -> &[Column] {
    data.table().map_or(&[], |t| &t.columns)
}

/// Refuses a missing or truncated data.
fn whole(deck: &Deck, data: &DataId) -> Result<(), OpError> {
    if !deck.data.contains_key(data) {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    }
    if deck.truncated.contains(data) {
        return Err(DataFault::Truncated { data: data.as_str().to_owned() }.into());
    }
    Ok(())
}

/// `bind_data`: points a chart or table param at `data`, projected on `columns`.
///
/// Rebinding to another data without `columns` keeps the columns the view
/// reads (for a chart, those its series use), which the new data must have. A second view of the data makes it shared.
/// A query data's columns are left to the session, which holds its result.
pub(super) fn bind_data(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    target: &Target,
    param: &str,
    data: &DataId,
    columns: Option<Vec<String>>,
) -> Result<(), OpError> {
    let (kind, params, _) = target_ref(deck, target)?;
    let host = match &declared(widget(registry, kind)?, param)?.ty {
        ParamType::Host { host: h @ (HostType::Chart | HostType::Table) } => *h,
        ParamType::List { .. } => return Err(DataFault::NotView { param: param.to_owned() }.into()),
        _ => return Err(ParamTypeSnafu { param, expected: Shape::ChartOrTableOption.required() }.build().into()),
    };
    let Some(bound) = deck.data.get(data) else {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    };
    if deck.truncated.contains(data) {
        return Err(DataFault::Truncated { data: data.as_str().to_owned() }.into());
    }
    // A query data's columns are its result's, which only the session holds: it checks them
    // against the result before the op runs (`bind_query_checked`), so none is checked here.
    let derived = bound.table().is_none();
    let has = |column: &str| derived || columns_of(bound).iter().any(|c| c.id == column);
    let current = params.value(param).cloned();
    if let Some(columns) = &columns {
        let mut seen = HashSet::new();
        if columns.is_empty() || !columns.iter().all(|c| valid_id(c) && seen.insert(c.as_str())) {
            return Err(DataError::columns_invalid().into());
        }
        if let Some(missing) = columns.iter().find(|c| !has(c)) {
            return Err(DataError::no_column(missing).into());
        }
    } else if let Some(old) = current.as_ref().and_then(|v| v.get("data")).and_then(Value::as_str)
        && old != data.as_str()
        && let Some(missing) = used_columns(deck, host, current.as_ref()).into_iter().find(|c| !has(c))
    {
        return Err(DataFault::ColumnMissing { column: missing, data: bound.name.clone() }.into());
    }
    let mut option = match current {
        Some(Value::Object(o)) => o,
        _ => Map::new(),
    };
    option.insert("data".to_owned(), Value::String(data.as_str().to_owned()));
    if let Some(columns) = columns {
        option.insert("columns".to_owned(), Value::from(columns));
    }
    let subject = target.element.as_deref().unwrap_or(&target.slide);
    let normalized = host::host_option(host, subject, &Value::Object(option))?;
    let (_, params) = target_mut(deck, target)?;
    params.set(param, ParamValue::Value(normalized));
    // Only an inline data needs the walk: once shared, it stays shared.
    if deck.data.get(data).is_some_and(|d| d.inline)
        && deck.data_users(data).len() > 1
        && let Some(entry) = deck.data.get_mut(data)
    {
        Arc::make_mut(entry).inline = false;
    }
    Ok(())
}

/// The data a view may switch to with `bind_data` without columns, in id order.
///
/// Its own data, then each other data of the deck holding every column the view reads, inline or
/// not; a truncated data, which `bind_data` refuses, is left out.
#[must_use]
pub fn bind_choices(deck: &Deck, host: HostType, option: &Value) -> Vec<DataId> {
    let current = option.get("data").and_then(Value::as_str);
    let used = used_columns(deck, host, Some(option));
    deck.data
        .iter()
        .filter(|(id, data)| {
            Some(id.as_str()) == current
                || (!deck.truncated.contains(*id)
                    && used.iter().all(|c| columns_of(data).iter().any(|col| col.id == *c)))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// The columns a view reads of its data: its projection, else `read_columns`.
///
/// A ref left dangling by a merge names no column to keep.
fn used_columns(deck: &Deck, host: HostType, option: Option<&Value>) -> Vec<String> {
    match option.and_then(|v| v.get("columns")).and_then(Value::as_array) {
        Some(projected) => projected.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        None => option
            .and_then(|v| v.get("data"))
            .and_then(Value::as_str)
            .and_then(|old| deck.data.get(old))
            .map(|d| read_columns(host, option, columns_of(d)))
            .unwrap_or_default(),
    }
}

/// The columns a view without projection reads: those a chart uses, every one of a table.
fn read_columns(host: HostType, option: Option<&Value>, columns: &[Column]) -> Vec<String> {
    let used = match (host, option) {
        (HostType::Chart, Some(option)) => {
            let names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
            serde_json::from_value::<ChartOption>(option.clone()).map_or(names.len(), |o| o.used_columns(&names))
        }
        _ => columns.len(),
    };
    columns.iter().take(used).map(|c| c.id.clone()).collect()
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
