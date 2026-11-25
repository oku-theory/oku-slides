//! `put_view_data`: the inline data of a view, stored as a data of the deck.
//!
//! Only `prepare_ops` makes the op, sealed: it comes right before the op that writes the view,
//! stripped of its inline data and naming `id`. A view that is the only user of its own inline
//! data gets its table replaced in place; any other gets a new data under `id`.

use std::sync::Arc;

use oku_slides_data::{Data, DataError, DataId, DataKind, NewTable, clean_label, valid_name};
use serde_json::Value;

use super::{Target, declared, target_mut, target_ref, widget};
use crate::error::{DataFault, OpError, ParamTypeSnafu, Shape};
use crate::model::Deck;
use crate::params::ParamValue;
use crate::widget::{HostType, ParamType, WidgetRegistry};

/// Marks a `PutViewData` made by `prepare_ops`; a deserialized one is unsealed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Seal(bool);

impl Seal {
    /// The seal `prepare_ops` puts on the ops it makes.
    pub(crate) fn sealed() -> Seal {
        Seal(true)
    }
}

/// The fields of a `PutViewData`.
pub(super) struct Put<'a> {
    pub(super) target: &'a Target,
    pub(super) param: &'a str,
    pub(super) id: DataId,
    pub(super) name: String,
    pub(super) label: Option<String>,
    pub(super) table: NewTable,
}

/// Replaces the view's own inline data in place, or adds a new one, and points the view at it.
pub(super) fn put_view_data(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    put: Put<'_>,
    seal: Seal,
) -> Result<(), OpError> {
    if !seal.0 {
        return Err(DataFault::Internal.into());
    }
    let Put { target, param, id, name, label, table } = put;
    let owner = target.element.as_deref().unwrap_or(&target.slide);
    let (table, _) = table.into_table()?;
    let label = label.as_deref().map(clean_label);
    if let Some(entry) = deck.data.get(&id) {
        let own = entry.inline && deck.data_users(&id).iter().all(|v| v.element == owner && v.param == param);
        if !own {
            return Err(DataFault::Internal.into());
        }
        let Some(entry) = deck.data.get_mut(&id) else { return Err(DataFault::Internal.into()) };
        let d = Arc::make_mut(entry);
        d.table = table;
        if label.is_some() {
            d.label = label;
        }
        deck.truncated.remove(&id);
    } else {
        let lower = name.to_lowercase();
        if deck.data.values().any(|d| d.name.to_lowercase() == lower) {
            return Err(DataFault::NameTaken { name }.into());
        }
        if !valid_name(&name) {
            return Err(DataError::invalid_name(&name).into());
        }
        deck.data.insert(id.clone(), Arc::new(Data { name, label, inline: true, kind: DataKind::Manual, table }));
    }
    point_at(deck, registry, target, param, &id)
}

/// Points an existing view bound to another data at `id`, so the next op keeps its ref.
///
/// A view the batch has not added yet is left to the op that adds it, which names `id`.
fn point_at(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    target: &Target,
    param: &str,
    id: &DataId,
) -> Result<(), OpError> {
    let Ok((kind, params, _)) = target_ref(deck, target) else { return Ok(()) };
    match &declared(widget(registry, kind)?, param)?.ty {
        ParamType::Host { host: HostType::Chart | HostType::Table } => {}
        _ => return Err(ParamTypeSnafu { param, expected: Shape::ChartOrTableOption.required() }.build().into()),
    }
    let Some(Value::Object(mut option)) = params.value(param).cloned() else { return Ok(()) };
    let bound = option.get("data").and_then(Value::as_str);
    if bound.is_none_or(|b| b == id.as_str()) {
        return Ok(());
    }
    option.insert("data".to_owned(), Value::String(id.as_str().to_owned()));
    option.remove("columns");
    let (_, params) = target_mut(deck, target)?;
    params.set(param, ParamValue::Value(Value::Object(option)));
    Ok(())
}
