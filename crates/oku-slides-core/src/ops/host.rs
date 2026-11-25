//! Params the host draws (`host(chart)`, `host(table)`, `host(path)`): their options are checked.
//!
//! Chart and table options name deck data; inline data reach the deck only through
//! `prepare_ops`, which lifts them into data, so an apply refuses them.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use snafu::ResultExt;

use oku_slides_chart::{apply_option_patch, check_ref_and_inline, parse_option, parse_table, patch_table};

use super::{declared, typed};
use crate::error::{
    ChartSnafu, DataFault, ExpectedType, InvalidValueSnafu, OpError, ParamTypeSnafu, ParseValueSnafu, Shape,
    TableSnafu, ValueReason,
};
use crate::model::{Deck, Element};
use crate::params::{ParamValue, Params};
use crate::prepare_view::brings_inline;
use crate::shape_path::ShapePath;
use crate::widget::{HostType, WidgetRegistry, WidgetSchema};

/// Forme attendue d'un paramètre de l'hôte `host`, pour `ParamType`.
pub(super) fn expected_host(host: HostType) -> ExpectedType {
    match host {
        HostType::Chart => Shape::ChartOption.required(),
        HostType::Table => Shape::TableOption.required(),
        HostType::Path => Shape::PathOption.required(),
    }
}

/// Checked options of host `host`, in their serialized form.
///
/// `subject` names the element (or the param) in the error.
///
/// # Errors
///
/// `chart-ref-and-inline` for a ref next to inline data, `op-data-inline` for inline data:
/// only `prepare_ops` turns them into deck data.
pub(crate) fn host_option(host: HostType, subject: &str, v: &Value) -> Result<Value, OpError> {
    let serialized = match host {
        // `parse_option` reads the options and checks their shape.
        HostType::Chart => {
            check_inline(subject, v)?;
            serde_json::to_value(parse_option(v).context(ChartSnafu { subject })?)
        }
        HostType::Table => {
            check_inline(subject, v)?;
            serde_json::to_value(parse_table(v).context(TableSnafu { subject })?)
        }
        // Un chemin que le widget ne saurait pas dessiner est refusé ici.
        HostType::Path => {
            let path: ShapePath = typed(subject, v.clone())?;
            if path.commands().is_none() {
                return Err(InvalidValueSnafu { field: subject, reason: ValueReason::Path }.build().into());
            }
            serde_json::to_value(path)
        }
    };
    Ok(serialized.context(ParseValueSnafu { field: subject })?)
}

/// Refuses raw options, or a merge patch, that bring inline data.
fn check_inline(subject: &str, v: &Value) -> Result<(), OpError> {
    check_ref_and_inline(v).context(ChartSnafu { subject })?;
    if brings_inline(v) {
        return Err(DataFault::Inline { subject: subject.to_owned() }.into());
    }
    Ok(())
}

/// Vérifie que `param`, déclaré par `schema`, est un paramètre d'hôte `want`.
fn ensure_host(schema: &WidgetSchema, param: &str, want: HostType) -> Result<(), OpError> {
    let p = declared(schema, param)?;
    if p.ty.host() != Some(want) {
        return Err(ParamTypeSnafu { param, expected: expected_host(want) }.build().into());
    }
    Ok(())
}

/// Refuses inconsistent host params; the checked values replace the ones given.
///
/// `subject`: what carries them.
pub(super) fn check_host_params(
    registry: &WidgetRegistry,
    kind: &str,
    params: &mut Params,
    subject: &str,
) -> Result<(), OpError> {
    let Some(schema) = registry.get(kind) else { return Ok(()) };
    for p in &schema.params {
        let Some(host) = p.ty.host() else { continue };
        match params.get(&p.name) {
            None => {}
            Some(ParamValue::Value(v)) => {
                let normalized = host_option(host, subject, v)?;
                params.set(&p.name, ParamValue::Value(normalized));
            }
            Some(_) => return Err(ParamTypeSnafu { param: &p.name, expected: expected_host(host) }.build().into()),
        }
    }
    Ok(())
}

/// Refuse un élément (ou un de ses enfants) dont un paramètre d'hôte est incohérent.
///
/// Les paramètres d'hôte de l'élément et de ses enfants sont réécrits normalisés.
pub(super) fn check_hosts(registry: &WidgetRegistry, element: &mut Element) -> Result<(), OpError> {
    check_host_params(registry, &element.kind, &mut element.params, &element.id)?;
    for (_, children) in element.params.slots_mut() {
        for child in children {
            check_hosts(registry, child)?;
        }
    }
    Ok(())
}

/// Rewrites the host params of `deck` (slides and nested elements) in their checked form, by the
/// same path as the ops. A value the host does not read stays as is: validation reports it.
pub(crate) fn normalize_hosts(registry: &WidgetRegistry, deck: &mut Deck) {
    for slide in &mut deck.slides {
        normalize_params(registry, &slide.kind, &mut slide.params);
    }
}

fn normalize_params(registry: &WidgetRegistry, kind: &str, params: &mut Params) {
    if let Some(schema) = registry.get(kind) {
        for p in &schema.params {
            let Some(host) = p.ty.host() else { continue };
            let normalized = match params.get(&p.name) {
                Some(ParamValue::Value(v)) => host_option(host, "", v).ok(),
                _ => None,
            };
            if let Some(v) = normalized {
                params.set(&p.name, ParamValue::Value(v));
            }
        }
    }
    for (_, children) in params.slots_mut() {
        for child in children {
            normalize_params(registry, &child.kind, &mut child.params);
        }
    }
}

fn read<T: DeserializeOwned>(params: &Params, param: &str, host: HostType) -> Result<T, OpError> {
    match params.get(param) {
        Some(ParamValue::Value(v)) => typed(param, v.clone()),
        None => Err(InvalidValueSnafu { field: param, reason: ValueReason::NoValue }.build().into()),
        Some(_) => Err(ParamTypeSnafu { param, expected: expected_host(host) }.build().into()),
    }
}

fn write<T: Serialize>(params: &mut Params, param: &str, option: &T) -> Result<(), OpError> {
    let v = serde_json::to_value(option).context(ParseValueSnafu { field: param })?;
    params.set(param, ParamValue::Value(v));
    Ok(())
}

/// `set_chart_option` / `set_table_option` : merge patch sur les options de l'hôte `host`.
pub(super) fn set_option(
    schema: &WidgetSchema,
    params: &mut Params,
    param: &str,
    host: HostType,
    patch: &Value,
    subject: &str,
) -> Result<(), OpError> {
    ensure_host(schema, param, host)?;
    match host {
        HostType::Chart => {
            check_inline(subject, patch)?;
            let (next, _) = apply_option_patch(&read(params, param, host)?, patch).context(ChartSnafu { subject })?;
            write(params, param, &next)
        }
        HostType::Table => {
            check_inline(subject, patch)?;
            let (next, _) = patch_table(&read(params, param, host)?, patch).context(TableSnafu { subject })?;
            write(params, param, &next)
        }
        // Un chemin se remplace entier (`set_param`) : il n'a pas d'options à fusionner.
        HostType::Path => Err(ParamTypeSnafu { param, expected: Shape::SetParamOnly.required() }.build().into()),
    }
}
