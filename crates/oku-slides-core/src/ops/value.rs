//! Conversion d'une valeur JSON (op `set_param`) selon le type déclaré du
//! paramètre.

use serde_json::Value;
use snafu::ResultExt;

use super::list::{ListSpec, list_value};
use crate::error::{
    ChoiceOutOfRangeSnafu, ExpectedType, InvalidValueSnafu, OpError, ParamTypeSnafu, ParseValueSnafu, Shape,
    ValueReason,
};
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
    let wrong = |expected: ExpectedType| -> OpError { ParamTypeSnafu { param, expected }.build().into() };
    match ty {
        ParamType::Plain | ParamType::Asset => match value {
            Value::Null => Ok(None),
            // Vide = absent ; bords rognés, comme l'UI et la relecture du replica.
            Value::String(s) if s.trim().is_empty() => Ok(None),
            Value::String(s) => Ok(Some(ParamValue::Text(s.trim().to_owned()))),
            _ => Err(wrong(Shape::Text.or_null())),
        },
        ParamType::Choice { values } => match value {
            Value::Null => Ok(None),
            Value::String(s) if values.contains(&s) => Ok(Some(ParamValue::Text(s))),
            Value::String(s) => Err(ChoiceOutOfRangeSnafu { param, value: s }.build().into()),
            _ => Err(wrong(Shape::Choice.or_null())),
        },
        ParamType::Rich => match value {
            Value::Null => Ok(None),
            Value::Array(a) if a.is_empty() => Ok(None),
            v @ Value::Array(_) => {
                let paragraphs: Vec<Paragraph> = serde_json::from_value(v).context(ParseValueSnafu { field: param })?;
                Ok(Some(ParamValue::Rich(paragraphs)))
            }
            _ => Err(wrong(Shape::Paragraphs.or_null())),
        },
        ParamType::Flag => match value {
            Value::Null => Ok(None),
            v @ Value::Bool(_) => Ok(Some(ParamValue::Value(v))),
            _ => Err(wrong(Shape::Flag.or_null())),
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
            _ => Err(wrong(Shape::Number.or_null())),
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
            _ => Err(wrong(Shape::Grid.or_null())),
        },
        ParamType::Host { host } => match value {
            v @ Value::Object(_) => super::host::host_option(*host, param, &v).map(|v| Some(ParamValue::Value(v))),
            _ => Err(wrong(super::host::expected_host(*host))),
        },
        ParamType::Slot { .. } => Err(wrong(Shape::AddElementOnly.required())),
        ParamType::List { .. } => {
            list_value(&ListSpec::of(param, ty).ok_or_else(|| wrong(Shape::Items.required()))?, value)
        }
    }
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod value_tests;
