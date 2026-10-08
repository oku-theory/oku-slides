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
use crate::labels::ChartLabels;
use crate::model::normalize;
use crate::option::ChartOption;

/// Applique `patch` et renvoie les nouvelles options et le patch inverse.
pub fn apply_option_patch(option: &ChartOption, patch: &Value) -> Result<(ChartOption, Value)> {
    let (next, inverse): (ChartOption, Value) = merge_patch(option, patch)?;
    // Avec un `dataset`, les données en ligne le contrediraient : la grille
    // éditerait l'un, le graphique montrerait l'autre.
    let inline = next.series.iter().any(|s| s.data.is_some())
        || [&next.x_axis, &next.y_axis].into_iter().flatten().any(|a| a.data.is_some());
    ensure!(next.dataset.is_none() || !inline, InvalidPatchSnafu { reason: Message::new("chart-patch-inline-data") });
    normalize(&next, ChartLabels::english())?;
    Ok((next, inverse))
}

/// Merge patch sur `option` hors `dataset` ; rend le résultat lu et le patch inverse.
pub(crate) fn merge_patch<T: Serialize + DeserializeOwned>(option: &T, patch: &Value) -> Result<(T, Value)> {
    ensure!(patch.get("dataset").is_none(), PatchTouchesDatasetSnafu);
    ensure!(patch.is_object(), InvalidPatchSnafu { reason: Message::new("chart-patch-object") });
    let before = serde_json::to_value(option).context(InvalidOptionSnafu)?;
    let inverse = reverse_merge_patch(&before, patch);
    let mut after = before;
    json_patch::merge(&mut after, patch);
    let next = serde_json::from_value(after).context(InvalidOptionSnafu)?;
    Ok((next, inverse))
}

/// Patch qui ramène `patch(before)` à `before`.
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
