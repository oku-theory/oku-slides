//! Paramètres fournis par l'hôte (`host(chart)`, `host(table)`) : contrôle
//! des valeurs, options et données.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use snafu::ResultExt;

use oku_slides_chart::{
    ChartOption, DataOp, apply_option_patch, chart_data, parse_option, parse_table, patch_table, table_data,
};

use super::{declared, typed, widget};
use crate::error::{ChartSnafu, InvalidValueSnafu, OpError, ParamTypeSnafu, ParseValueSnafu, TableSnafu};
use crate::locate::Walk;
use crate::model::Element;
use crate::params::{ParamValue, Params};
use crate::widget::{HostType, WidgetRegistry, WidgetSchema};

/// Forme attendue d'un paramètre de l'hôte `host`, pour `ParamType`.
pub(super) fn expected_host(host: HostType) -> &'static str {
    match host {
        HostType::Chart => "a chart option",
        HostType::Table => "a table option",
    }
}

/// Options de l'hôte `host` contrôlées, rendues sous leur forme sérialisée.
///
/// `subject` nomme l'élément (ou le paramètre) dans l'erreur.
pub(crate) fn host_option(host: HostType, subject: &str, v: &Value) -> Result<Value, OpError> {
    let serialized = match host {
        // `parse_option` lit les options et les passe par `normalize`.
        HostType::Chart => serde_json::to_value(parse_option(v).context(ChartSnafu { subject })?),
        HostType::Table => serde_json::to_value(parse_table(v).context(TableSnafu { subject })?),
    };
    Ok(serialized.context(ParseValueSnafu { field: subject })?)
}

/// Vérifie que `param`, déclaré par `schema`, est un paramètre d'hôte `want`.
fn ensure_host(schema: &WidgetSchema, param: &str, want: HostType) -> Result<(), OpError> {
    let p = declared(schema, param)?;
    if p.ty.host() != Some(want) {
        return Err(ParamTypeSnafu { param, expected: expected_host(want) }.build().into());
    }
    Ok(())
}

/// Refuse des paramètres d'hôte incohérents (`subject` : qui les porte).
pub(super) fn check_host_params(
    registry: &WidgetRegistry,
    kind: &str,
    params: &Params,
    subject: &str,
) -> Result<(), OpError> {
    let Some(schema) = registry.get(kind) else { return Ok(()) };
    for p in &schema.params {
        let Some(host) = p.ty.host() else { continue };
        match params.get(&p.name) {
            None => {}
            Some(ParamValue::Value(v)) => {
                host_option(host, subject, v)?;
            }
            Some(_) => return Err(ParamTypeSnafu { param: &p.name, expected: expected_host(host) }.build().into()),
        }
    }
    Ok(())
}

/// Refuse un élément (ou un de ses enfants) dont un paramètre d'hôte est incohérent.
pub(super) fn check_hosts(registry: &WidgetRegistry, element: &Element) -> Result<(), OpError> {
    let nested = Walk::over(&element.params, Some(&element.id)).map(|l| l.element);
    for e in std::iter::once(element).chain(nested) {
        check_host_params(registry, &e.kind, &e.params, &e.id)?;
    }
    Ok(())
}

fn read<T: DeserializeOwned>(params: &Params, param: &str, host: HostType) -> Result<T, OpError> {
    match params.get(param) {
        Some(ParamValue::Value(v)) => typed(param, v.clone()),
        None => Err(InvalidValueSnafu { field: param, reason: "the parameter has no value" }.build().into()),
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
            let (next, _) = apply_option_patch(&read(params, param, host)?, patch).context(ChartSnafu { subject })?;
            write(params, param, &next)
        }
        HostType::Table => {
            let (next, _) = patch_table(&read(params, param, host)?, patch).context(TableSnafu { subject })?;
            write(params, param, &next)
        }
    }
}

/// `set_data` : applique `change` aux données d'un graphique ou d'un tableau.
pub(super) fn set_data(
    registry: &WidgetRegistry,
    kind: &str,
    params: &mut Params,
    param: &str,
    change: &DataOp,
    subject: &str,
) -> Result<(), OpError> {
    let p = declared(widget(registry, kind)?, param)?;
    match p.ty.host() {
        Some(host @ HostType::Chart) => {
            let option: ChartOption = read(params, param, host)?;
            if option.dataset.is_none() {
                // Garde le refus historique (`InvalidValue`) pour un graphique sans dataset.
                return Err(InvalidValueSnafu { field: "dataset", reason: "the chart has no dataset" }.build().into());
            }
            let next = chart_data(&option, change).context(ChartSnafu { subject })?;
            write(params, param, &next)
        }
        Some(host @ HostType::Table) => {
            let next = table_data(&read(params, param, host)?, change).context(TableSnafu { subject })?;
            write(params, param, &next)
        }
        None => Err(ParamTypeSnafu { param, expected: "a chart or table option" }.build().into()),
    }
}
