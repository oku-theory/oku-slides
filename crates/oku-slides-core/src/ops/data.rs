//! The data ops: add, edit, bind, remove, rename and retype a data.
//!
//! Each checks first and writes last, so a refused op leaves the deck intact.
//! Ids come filled in by `prepare_ops`: an id left out is refused here.

use std::collections::HashSet;
use std::sync::Arc;

use oku_slides_chart::ChartOption;
use oku_slides_data::{
    ColumnType, Data, DataError, DataId, DataKind, NewTable, Table, TableOp, clean_label, valid_id, valid_name,
};
use serde_json::{Map, Value};

use super::{Target, declared, host, target_mut, target_ref, widget};
use crate::error::{DataFault, OpError, ParamTypeSnafu, Shape};
use crate::model::Deck;
use crate::params::ParamValue;
use crate::widget::{HostType, ParamType, WidgetRegistry};

/// `add_data`: a free id and name, a valid table; the deck bounds are checked by `apply_all`.
pub(super) fn add_data(
    deck: &mut Deck,
    id: Option<DataId>,
    name: String,
    label: Option<&str>,
    table: NewTable,
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
    let (table, _) = table.into_table()?;
    let label = label.map(clean_label);
    deck.data.insert(id, Arc::new(Data { name, label, inline: false, kind: DataKind::Manual, table }));
    Ok(())
}

/// `edit_data`: an existing, whole data; a projected column is never removed.
pub(super) fn edit_data(deck: &mut Deck, data: &DataId, change: &TableOp) -> Result<(), OpError> {
    if !deck.data.contains_key(data) {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    }
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
    d.table.apply(change)?;
    d.inline = false;
    if replace {
        deck.truncated.remove(data);
    }
    Ok(())
}

/// Views a refusal names before counting the others.
const NAMED_VIEWS: usize = 10;

/// `remove_data`: a data no view shows; a truncated one too, and its mark goes with it.
pub(super) fn remove_data(deck: &mut Deck, data: &DataId) -> Result<(), OpError> {
    if !deck.data.contains_key(data) {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    }
    let users = deck.data_users(data);
    if !users.is_empty() {
        let more = users.len().saturating_sub(NAMED_VIEWS);
        let views = users.into_iter().take(NAMED_VIEWS).map(|v| v.element).collect();
        return Err(DataFault::InUse { views, more }.into());
    }
    deck.data.remove(data);
    deck.truncated.remove(data);
    Ok(())
}

/// `rename_data`: a valid name no other data has (case ignored); the data stops being inline.
pub(super) fn rename_data(deck: &mut Deck, data: &DataId, name: String) -> Result<(), OpError> {
    whole(deck, data)?;
    let lower = name.to_lowercase();
    if deck.data.iter().any(|(id, d)| id != data && d.name.to_lowercase() == lower) {
        return Err(DataFault::NameTaken { name }.into());
    }
    if !valid_name(&name) {
        return Err(DataError::invalid_name(&name).into());
    }
    let Some(entry) = deck.data.get_mut(data) else {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    };
    let d = Arc::make_mut(entry);
    d.name = name;
    d.inline = false;
    Ok(())
}

/// `set_column_type`: converts the overlay cells without loss; the data stops being inline.
///
/// With a base, the session has the worker check its cells first.
pub(super) fn set_column_type(deck: &mut Deck, data: &DataId, column: &str, ty: ColumnType) -> Result<(), OpError> {
    whole(deck, data)?;
    let Some(entry) = deck.data.get_mut(data) else {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    };
    // `Table::set_column_type` leaves the table unchanged when it refuses.
    let d = Arc::make_mut(entry);
    d.table.set_column_type(column, ty)?;
    d.inline = false;
    Ok(())
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
    let has = |column: &str| bound.table.columns.iter().any(|c| c.id == column);
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
                    && used.iter().all(|c| data.table.columns.iter().any(|col| col.id == *c)))
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
            .map(|d| read_columns(host, option, &d.table))
            .unwrap_or_default(),
    }
}

/// The columns a view without projection reads: those a chart uses, every one of a table.
fn read_columns(host: HostType, option: Option<&Value>, table: &Table) -> Vec<String> {
    let used = match (host, option) {
        (HostType::Chart, Some(option)) => {
            let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
            serde_json::from_value::<ChartOption>(option.clone()).map_or(names.len(), |o| o.used_columns(&names))
        }
        _ => table.columns.len(),
    };
    table.columns.iter().take(used).map(|c| c.id.clone()).collect()
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
