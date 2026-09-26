use speculoos::prelude::*;

use super::*;

#[test]
fn nice_ceiling_any_value_rounds_up_to_nice_number() {
    assert_that!(nice_ceiling(87.0)).is_equal_to(100.0);
    assert_that!(nice_ceiling(12.0)).is_equal_to(20.0);
    assert_that!(nice_ceiling(2.3)).is_equal_to(2.5);
    assert_that!(nice_ceiling(0.0)).is_equal_to(1.0);
}

#[test]
fn normalize_stacked_bars_keep_their_stack_group() {
    let option: ChartOption = serde_json::from_value(serde_json::json!({
        "dataset": { "source": [["Mois", "Nord", "Sud"], ["Jan", 3, -4]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "stack": "total" }, { "type": "bar", "stack": "total" }]
    }))
    .unwrap();
    let model = normalize(&option, ChartLabels::english()).unwrap();
    let ChartKind::Cartesian(c) = model.kind else { panic!("cartésien attendu") };
    assert_that!(c.series.iter().map(|s| s.stack.as_deref()).collect::<Vec<_>>())
        .is_equal_to(vec![Some("total"), Some("total")]);
}

fn option(v: serde_json::Value) -> ChartOption {
    serde_json::from_value(v).unwrap()
}

fn waterfall_of(v: serde_json::Value) -> Result<Waterfall> {
    match normalize(&option(v), ChartLabels::english())?.kind {
        ChartKind::Waterfall(w) => Ok(w),
        other => panic!("cascade attendue : {other:?}"),
    }
}

#[test]
fn normalize_waterfall_dataset_reads_steps_and_empty_totals() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [["Poste", "Montant"], ["Début", 100], ["Ventes", 30], ["Coûts", -20], ["Fin", null]] },
        "series": [{ "type": "waterfall" }]
    }))
    .unwrap();
    assert_that!(w.categories).is_equal_to(vec!["Début".into(), "Ventes".into(), "Coûts".into(), "Fin".into()]);
    assert_that!(w.steps).is_equal_to(vec![Some(100.0), Some(30.0), Some(-20.0), None]);
    assert_that!(w.name.as_str()).is_equal_to("Montant");
}

#[test]
fn normalize_waterfall_inline_data_reads_categories_from_x_axis() {
    let w = waterfall_of(serde_json::json!({
        "xAxis": { "type": "category", "data": ["Début", "Hausse", "Fin"] },
        "series": [{ "type": "waterfall", "data": [10, 5, null] }]
    }))
    .unwrap();
    assert_that!(w.steps).is_equal_to(vec![Some(10.0), Some(5.0), None]);
}

#[test]
fn normalize_waterfall_hides_the_legend_by_default() {
    let model = normalize(
        &option(serde_json::json!({
            "dataset": { "source": [["Poste", "Montant"], ["Début", 100]] },
            "series": [{ "type": "waterfall" }]
        })),
        ChartLabels::english(),
    )
    .unwrap();
    assert_that!(model.legend_show).is_false();
}

#[test]
fn normalize_waterfall_with_two_series_fails() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [["Poste", "A", "B"], ["Début", 1, 2]] },
        "series": [{ "type": "waterfall" }, { "type": "waterfall" }]
    }));
    assert_that!(w.err().map(|e| e.kind())).is_equal_to(Some(crate::ChartErrorKind::Inconsistent));
}

#[test]
fn normalize_waterfall_rounds_the_axis_above_the_highest_running_sum() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [["Poste", "Montant"], ["Début", 3], ["Baisse", -1], ["Hausse", 4], ["Fin", null]] },
        "series": [{ "type": "waterfall" }]
    }))
    .unwrap();
    // Sommes courantes : 3, 2, 6, total 6 → 10.
    assert_that!((w.value_min, w.value_max)).is_equal_to((None, Some(10.0)));
}

#[test]
fn normalize_waterfall_going_below_zero_rounds_the_axis_down() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [["Poste", "Montant"], ["Début", 2], ["Perte", -5]] },
        "series": [{ "type": "waterfall" }]
    }))
    .unwrap();
    // Sommes courantes : 2, puis −3 → axe de −5 à 2,5 (jamais pile sur une barre).
    assert_that!((w.value_min, w.value_max)).is_equal_to((Some(-5.0), Some(2.5)));
}

#[test]
fn normalize_waterfall_keeps_declared_axis_bounds() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [["Poste", "Montant"], ["Début", 3]] },
        "yAxis": { "type": "value", "min": -1, "max": 4 },
        "series": [{ "type": "waterfall" }]
    }))
    .unwrap();
    assert_that!((w.value_min, w.value_max)).is_equal_to((Some(-1.0), Some(4.0)));
}

fn two_bars(extra: serde_json::Value) -> ChartOption {
    let mut v = serde_json::json!({
        "dataset": { "source": [["Mois", "Nord", "Sud"], ["Jan", 3, 4]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }, { "type": "bar", "itemStyle": { "color": "#C4622D" } }]
    });
    json_patch::merge(&mut v, &extra);
    option(v)
}

#[test]
fn normalize_option_color_becomes_the_palette() {
    let model =
        normalize(&two_bars(serde_json::json!({ "color": ["#112233", "#445566"] })), ChartLabels::english()).unwrap();
    assert_that!(model.palette).is_equal_to(Some(vec!["#112233".to_owned(), "#445566".to_owned()]));
}

#[test]
fn normalize_series_color_reaches_the_cartesian_series() {
    let ChartKind::Cartesian(c) = normalize(&two_bars(serde_json::json!({})), ChartLabels::english()).unwrap().kind
    else {
        panic!("cartésien attendu")
    };
    assert_that!(c.series.iter().map(|s| s.color.as_deref()).collect::<Vec<_>>())
        .is_equal_to(vec![None, Some("#C4622D")]);
}

#[test]
fn normalize_invalid_series_color_is_rejected() {
    let bad = two_bars(serde_json::json!({ "series": [{ "type": "bar", "itemStyle": { "color": "rouge" } }] }));
    let err = normalize(&bad, ChartLabels::english()).err();
    assert_that!(err.map(|e| e.kind())).is_equal_to(Some(crate::ChartErrorKind::Inconsistent));
}

#[test]
fn normalize_invalid_palette_color_is_rejected() {
    let bad = two_bars(serde_json::json!({ "color": ["#12345"] }));
    assert_that!(normalize(&bad, ChartLabels::english()).is_err()).is_true();
}
