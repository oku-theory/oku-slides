use serde_json::json;
use speculoos::prelude::*;

use super::*;

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
fn apply_option_patch_then_inverse_restores_original() {
    let original = option();
    let patch = json!({
        "title": { "text": "Chiffre d'affaires", "subtext": "2026" },
        "legend": { "left": "right" },
        "xAxis": null,
        "series": [{ "type": "line", "smooth": true }]
    });
    let (next, inverse) = apply_option_patch(&original, &patch).unwrap();
    assert_that!(next.title.as_ref().unwrap().subtext.as_deref()).is_equal_to(Some("2026"));
    assert_that!(next.x_axis).is_none();
    let (restored, _) = apply_option_patch(&next, &inverse).unwrap();
    assert_that!(restored).is_equal_to(original);
}

#[test]
fn apply_option_patch_touching_dataset_is_rejected() {
    let r = apply_option_patch(&option(), &json!({ "dataset": null }));
    assert_that!(r.unwrap_err().kind()).is_equal_to(crate::ChartErrorKind::PatchTouchesDataset);
}

#[test]
fn apply_option_patch_unknown_field_is_rejected_with_its_name() {
    let r = apply_option_patch(&option(), &json!({ "tooltip": { "show": true } }));
    let msg = slider_i18n::Localize::localize(&r.unwrap_err(), slider_i18n::english());
    asserting(&msg).that(&msg.as_str()).contains("tooltip");
}

#[test]
fn chart_errors_localize_in_the_user_language() {
    use slider_i18n::{Localize, Localizer};
    let fr = Localizer::new(Some("fr"));
    let dataset = apply_option_patch(&option(), &json!({ "dataset": null })).unwrap_err();
    assert_that!(dataset.localize(&fr)).contains("utiliser set_chart_data");
    let not_object = apply_option_patch(&option(), &json!([1])).unwrap_err();
    assert_that!(not_object.localize(&fr)).is_equal_to("le patch doit être un objet JSON".to_owned());
    assert_that!(not_object.kind()).is_equal_to(crate::ChartErrorKind::InvalidOption);
    let unknown = apply_option_patch(&option(), &json!({ "tooltip": {} })).unwrap_err();
    assert_that!(unknown.localize(&fr)).starts_with("options invalides : ");
}

#[test]
fn apply_option_patch_inline_data_next_to_dataset_is_refused() {
    for patch in [json!({ "series": [{ "type": "bar", "data": [1] }] }), json!({ "xAxis": { "data": ["a"] } })] {
        let err = apply_option_patch(&option(), &patch).unwrap_err();
        assert_that!(err.kind()).is_equal_to(crate::ChartErrorKind::InvalidOption);
    }
}

/// Les patches de changement de type envoyés par l'interface (chartDeck.ts).
#[test]
fn apply_option_patch_kind_switches_from_the_ui_are_accepted() {
    let two = |o: ChartOption| {
        let mut o = o;
        o.dataset = Some(serde_json::from_value(json!({ "source": [["M", "A", "B"], ["Jan", 1, 2]] })).unwrap());
        o.series.push(o.series[0].clone());
        o
    };
    let bars = two(option());
    let (pie, _) =
        apply_option_patch(&bars, &json!({ "xAxis": null, "yAxis": null, "series": [{ "type": "pie" }] })).unwrap();
    let back = json!({ "xAxis": { "type": "category" }, "yAxis": { "type": "value" },
        "series": [{ "type": "line" }, { "type": "line" }] });
    let (lines, _) = apply_option_patch(&pie, &back).unwrap();
    assert_that!(lines.series.len()).is_equal_to(2);
}
