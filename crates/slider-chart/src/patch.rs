//! `set_chart_option` : JSON Merge Patch (RFC 7396) sur les options, hors
//! `dataset`. Le résultat est revalidé ; l'inverse est un autre merge patch.
//!
//! Rappel RFC 7396 : `null` supprime un champ, un tableau remplace le tableau
//! entier (modifier une série = renvoyer tout `series`).

use serde_json::{Map, Value};

use slider_i18n::Message;
use snafu::{ResultExt, ensure};

use crate::error::{InvalidOptionSnafu, InvalidPatchSnafu, PatchTouchesDatasetSnafu, Result};
use crate::model::normalize;
use crate::option::ChartOption;

/// Applique `patch` et renvoie les nouvelles options et le patch inverse.
pub fn apply_option_patch(option: &ChartOption, patch: &Value) -> Result<(ChartOption, Value)> {
    ensure!(patch.get("dataset").is_none(), PatchTouchesDatasetSnafu);
    ensure!(patch.is_object(), InvalidPatchSnafu { reason: Message::new("chart-patch-object") });
    let before = serde_json::to_value(option).context(InvalidOptionSnafu)?;
    let inverse = reverse_merge_patch(&before, patch);
    let mut after = before;
    json_patch::merge(&mut after, patch);
    let next: ChartOption = serde_json::from_value(after).context(InvalidOptionSnafu)?;
    normalize(&next)?;
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
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn option() -> ChartOption {
        serde_json::from_value(json!({
            "title": { "text": "Ventes" },
            "dataset": { "source": [["Mois", "Ventes"], ["Jan", 10]] },
            "xAxis": { "type": "category" },
            "series": [{ "type": "bar" }]
        }))
        .unwrap()
    }

    #[test]
    fn patch_then_inverse_restores() {
        let original = option();
        let patch = json!({
            "title": { "text": "Chiffre d'affaires", "subtext": "2026" },
            "legend": { "left": "right" },
            "xAxis": null,
            "series": [{ "type": "line", "smooth": true }]
        });
        let (next, inverse) = apply_option_patch(&original, &patch).unwrap();
        assert_eq!(next.title.as_ref().unwrap().subtext.as_deref(), Some("2026"));
        assert!(next.x_axis.is_none());
        let (restored, _) = apply_option_patch(&next, &inverse).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn dataset_is_off_limits() {
        let r = apply_option_patch(&option(), &json!({ "dataset": null }));
        assert_eq!(r.unwrap_err().kind(), crate::ChartErrorKind::PatchTouchesDataset);
    }

    #[test]
    fn unknown_fields_are_rejected_with_their_name() {
        let r = apply_option_patch(&option(), &json!({ "tooltip": { "show": true } }));
        let msg = slider_i18n::Localize::localize(&r.unwrap_err(), slider_i18n::english());
        assert!(msg.contains("tooltip"), "{msg}");
    }
}
