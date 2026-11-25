//! `set_chart_option` : JSON Merge Patch (RFC 7396) sur les options, hors
//! `dataset`. Le résultat est revalidé ; l'inverse est un autre merge patch.
//!
//! Rappel RFC 7396 : `null` supprime un champ, un tableau remplace le tableau
//! entier (modifier une série = renvoyer tout `series`).

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use oku_slides_i18n::Message;
use snafu::{ResultExt, ensure};

use crate::error::{InvalidOptionSnafu, InvalidPatchSnafu, PatchTouchesDatasetSnafu, Result};
use crate::model::check_option;
use crate::option::ChartOption;
use crate::refs::check_ref_null;

/// Applies `patch` and returns the new options and the inverse patch.
///
/// # Errors
///
/// Fails if the patch touches `dataset`, is not an object, sets `data` to
/// `null`, adds inline data next to a `dataset` or a data ref, or makes the options invalid
/// or inconsistent.
pub fn apply_option_patch(option: &ChartOption, patch: &Value) -> Result<(ChartOption, Value)> {
    let (next, inverse): (ChartOption, Value) = merge_patch(option, patch)?;
    // Next to a `dataset` or a data ref, inline data would contradict it: the grid would
    // edit one, the chart show the other. `prepare_ops` turns such a patch into a whole option.
    let inline = next.series.iter().any(|s| s.data.is_some())
        || [&next.x_axis, &next.y_axis].into_iter().flatten().any(|a| a.data.is_some());
    let has_data = next.dataset.is_some() || next.data.is_some();
    ensure!(!has_data || !inline, InvalidPatchSnafu { reason: Message::new("chart-patch-inline-data") });
    check_option(&next)?;
    Ok((next, inverse))
}

/// Merge patch sur `option` hors `dataset` ; rend le résultat lu et le patch inverse.
pub(crate) fn merge_patch<T: Serialize + DeserializeOwned>(option: &T, patch: &Value) -> Result<(T, Value)> {
    ensure!(patch.get("dataset").is_none(), PatchTouchesDatasetSnafu);
    ensure!(patch.is_object(), InvalidPatchSnafu { reason: Message::new("chart-patch-object") });
    check_ref_null(patch)?;
    let before = serde_json::to_value(option).context(InvalidOptionSnafu)?;
    let inverse = reverse_merge_patch(&before, patch);
    let mut after = before;
    json_patch::merge(&mut after, patch);
    let next = serde_json::from_value(after).context(InvalidOptionSnafu)?;
    Ok((next, inverse))
}

/// Patch qui ramène `patch(before)` à `before`.
#[must_use]
pub fn reverse_merge_patch(before: &Value, patch: &Value) -> Value {
    let (Value::Object(p), Value::Object(b)) = (patch, before) else {
        return before.clone();
    };
    let mut out = Map::new();
    for (key, pv) in p {
        let reverted = match b.get(key) {
            None => Value::Null,
            Some(bv) if pv.is_object() && bv.is_object() => reverse_merge_patch(bv, pv),
            Some(bv) => bv.clone(),
        };
        out.insert(key.clone(), reverted);
    }
    Value::Object(out)
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod patch_tests;
