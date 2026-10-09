//! Conversion d'une valeur JSON (op `set_param`) selon le type déclaré du
//! paramètre.

use serde_json::Value;
use snafu::ResultExt;

use super::list::{ListSpec, list_value};
use crate::error::{ChoiceOutOfRangeSnafu, InvalidValueSnafu, OpError, ParamTypeSnafu, ParseValueSnafu, ValueReason};
use crate::model::{Grid, Paragraph};
use crate::params::ParamValue;
use crate::widget::ParamType;

/// Pistes d'une grille : 1 à 24 colonnes ou lignes.
pub const GRID_TRACKS: std::ops::RangeInclusive<u32> = 1..=24;

/// Convertit `value` pour le paramètre `param` de type `ty`. `Ok(None)` :
/// retirer le paramètre (`null`, texte blanc, texte riche vide).
///
/// Une valeur d'une autre forme JSON que celle du type rend `ParamType` ;
/// une valeur de la bonne forme mais refusée rend `InvalidValue`,
/// `ChoiceOutOfRange`, `InvalidChart` ou `InvalidTable`. Un slot se remplit par
/// `add_element`, jamais par une valeur.
pub fn param_value(ty: &ParamType, param: &str, value: Value) -> Result<Option<ParamValue>, OpError> {
    let wrong = |expected: &'static str| -> OpError { ParamTypeSnafu { param, expected }.build().into() };
    match ty {
        ParamType::Plain | ParamType::Asset => match value {
            Value::Null => Ok(None),
            // Vide = absent ; bords rognés, comme l'UI et la relecture du replica.
            Value::String(s) if s.trim().is_empty() => Ok(None),
            Value::String(s) => Ok(Some(ParamValue::Text(s.trim().to_owned()))),
            _ => Err(wrong("a string or null")),
        },
        ParamType::Choice { values } => match value {
            Value::Null => Ok(None),
            Value::String(s) if values.contains(&s) => Ok(Some(ParamValue::Text(s))),
            Value::String(s) => Err(ChoiceOutOfRangeSnafu { param, value: s }.build().into()),
            _ => Err(wrong("one of its values or null")),
        },
        ParamType::Rich => match value {
            Value::Null => Ok(None),
            Value::Array(a) if a.is_empty() => Ok(None),
            v @ Value::Array(_) => {
                let paragraphs: Vec<Paragraph> = serde_json::from_value(v).context(ParseValueSnafu { field: param })?;
                Ok(Some(ParamValue::Rich(paragraphs)))
            }
            _ => Err(wrong("a list of paragraphs or null")),
        },
        ParamType::Flag => match value {
            Value::Null => Ok(None),
            v @ Value::Bool(_) => Ok(Some(ParamValue::Value(v))),
            _ => Err(wrong("a boolean or null")),
        },
        ParamType::Number { min, max } => match value {
            Value::Null => Ok(None),
            Value::Number(n) => {
                let x = n.as_f64().unwrap_or(f64::NAN);
                if min.is_some_and(|m| x < m) || max.is_some_and(|m| x > m) || x.is_nan() {
                    return Err(InvalidValueSnafu { field: param, reason: ValueReason::OutOfBounds }.build().into());
                }
                Ok(Some(ParamValue::Value(Value::Number(n))))
            }
            _ => Err(wrong("a number or null")),
        },
        ParamType::Grid => match value {
            Value::Null => Ok(None),
            v @ Value::Object(_) => {
                let grid: Grid = serde_json::from_value(v).context(ParseValueSnafu { field: param })?;
                if !GRID_TRACKS.contains(&grid.columns) || !GRID_TRACKS.contains(&grid.rows) {
                    return Err(InvalidValueSnafu { field: param, reason: ValueReason::GridTracks }.build().into());
                }
                let v = serde_json::to_value(grid).context(ParseValueSnafu { field: param })?;
                Ok(Some(ParamValue::Value(v)))
            }
            _ => Err(wrong("a grid or null")),
        },
        ParamType::Host { host } => match value {
            v @ Value::Object(_) => super::host::host_option(*host, param, &v).map(|v| Some(ParamValue::Value(v))),
            _ => Err(wrong(super::host::expected_host(*host))),
        },
        ParamType::Slot { .. } => Err(wrong("elements, through add_element")),
        ParamType::List { .. } => list_value(&ListSpec::of(param, ty).ok_or_else(|| wrong("a list of items"))?, value),
    }
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod value_tests;
