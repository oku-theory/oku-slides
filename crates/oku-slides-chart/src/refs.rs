//! Data refs in chart and table options: `data` names a deck data, `columns` projects it.

use oku_slides_data::limits::MAX_COLUMNS;
use oku_slides_data::{DataId, valid_id};
use oku_slides_i18n::Message;
use serde_json::Value;

use crate::error::Result;
use crate::ids::all_unique;
use crate::model::inconsistent;

/// Refuses `"data": null` at the top of raw options: a ref is never unset.
///
/// # Errors
///
/// Fails with `chart-ref-null` when `v` holds `"data": null`.
pub fn check_ref_null(v: &Value) -> Result<()> {
    if v.get("data").is_some_and(Value::is_null) {
        return Err(inconsistent(Message::new("chart-ref-null")));
    }
    Ok(())
}

/// Refuses `columns` without `data`, empty, repeated, malformed or too many.
pub(crate) fn check_columns(data: Option<&DataId>, columns: Option<&[String]>) -> Result<()> {
    let Some(columns) = columns else { return Ok(()) };
    let valid = data.is_some()
        && !columns.is_empty()
        && columns.len() <= MAX_COLUMNS
        && columns.iter().all(|c| valid_id(c))
        && all_unique(columns);
    if valid { Ok(()) } else { Err(inconsistent(Message::new("chart-columns-invalid"))) }
}

/// Refuses raw options that set a data ref next to inline data.
///
/// Inline data are a non-null `dataset` or a series' non-null `data`.
///
/// # Errors
///
/// Fails with `chart-ref-and-inline` when both are present.
pub fn check_ref_and_inline(v: &Value) -> Result<()> {
    let present = |v: Option<&Value>| v.is_some_and(|d| !d.is_null());
    let inline = present(v.get("dataset"))
        || v.get("series").and_then(Value::as_array).is_some_and(|s| s.iter().any(|x| present(x.get("data"))));
    if v.get("data").is_some_and(Value::is_string) && inline {
        return Err(inconsistent(Message::new("chart-ref-and-inline")));
    }
    Ok(())
}

#[cfg(test)]
#[path = "refs_tests.rs"]
mod refs_tests;
