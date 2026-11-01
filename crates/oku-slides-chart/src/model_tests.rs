use speculoos::prelude::*;

use super::*;
use crate::ChartErrorKind;

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
fn normalize_waterfall_dataset_reads_steps_and_total_column() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [
            ["Poste", "Montant", "Total"],
            ["Début", 100, false], ["Ventes", 30, false], ["Coûts", -20, false], ["Fin", null, true]
        ] },
        "series": [{ "type": "waterfall" }]
    }))
    .unwrap();
    assert_that!(w.categories).is_equal_to(vec!["Début".into(), "Ventes".into(), "Coûts".into(), "Fin".into()]);
    assert_that!(w.steps).is_equal_to(vec![Some(100.0), Some(30.0), Some(-20.0), None]);
    assert_that!(w.totals).is_equal_to(vec![false, false, false, true]);
    assert_that!(w.name.as_str()).is_equal_to("Montant");
}

#[test]
fn normalize_waterfall_empty_value_without_total_is_not_a_total() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [["Poste", "Montant"], ["Début", 100], ["Fin", null]] },
        "series": [{ "type": "waterfall" }]
    }))
    .unwrap();
    assert_that!(w.totals).is_equal_to(vec![false, false]);
}

#[test]
fn normalize_waterfall_inline_data_fails() {
    let w = waterfall_of(serde_json::json!({
        "xAxis": { "type": "category", "data": ["Début", "Hausse", "Fin"] },
        "series": [{ "type": "waterfall", "data": [10, 5, null] }]
    }));
    assert_that!(w.err().map(|e| e.kind())).is_equal_to(Some(crate::ChartErrorKind::Inconsistent));
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

#[test]
fn normalize_waterfall_total_rows_ignore_their_value_for_the_axis() {
    let w = waterfall_of(serde_json::json!({
        "dataset": { "source": [["Poste", "Montant", "Total"], ["Début", 3, false], ["Fin", 900, true]] },
        "series": [{ "type": "waterfall" }]
    }))
    .unwrap();
    assert_that!(w.value_max).is_equal_to(Some(5.0));
}

fn two_bars(extra: &serde_json::Value) -> ChartOption {
    let mut v = serde_json::json!({
        "dataset": { "source": [["Mois", "Nord", "Sud"], ["Jan", 3, 4]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }, { "type": "bar", "itemStyle": { "color": "#C4622D" } }]
    });
    json_patch::merge(&mut v, extra);
    option(v)
}

#[test]
fn normalize_option_color_becomes_the_palette() {
    let model =
        normalize(&two_bars(&serde_json::json!({ "color": ["#112233", "#445566"] })), ChartLabels::english()).unwrap();
    assert_that!(model.palette).is_equal_to(Some(vec!["#112233".to_owned(), "#445566".to_owned()]));
}

#[test]
fn normalize_series_color_reaches_the_cartesian_series() {
    let ChartKind::Cartesian(c) = normalize(&two_bars(&serde_json::json!({})), ChartLabels::english()).unwrap().kind
    else {
        panic!("cartésien attendu")
    };
    assert_that!(c.series.iter().map(|s| s.color.as_deref()).collect::<Vec<_>>())
        .is_equal_to(vec![None, Some("#C4622D")]);
}

#[test]
fn normalize_invalid_series_color_is_rejected() {
    let bad = two_bars(&serde_json::json!({ "series": [{ "type": "bar", "itemStyle": { "color": "rouge" } }] }));
    let err = normalize(&bad, ChartLabels::english()).err();
    assert_that!(err.map(|e| e.kind())).is_equal_to(Some(crate::ChartErrorKind::Inconsistent));
}

#[test]
fn normalize_invalid_palette_color_is_rejected() {
    let bad = two_bars(&serde_json::json!({ "color": ["#12345"] }));
    assert_that!(normalize(&bad, ChartLabels::english()).is_err()).is_true();
}

fn bars_with_ids(ids: &serde_json::Value, columns: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 3]], "ids": ids, "columns": columns },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    })
}

fn parse_kind(v: &serde_json::Value) -> crate::ChartErrorKind {
    parse_option(v).map(|_| ()).unwrap_err().kind()
}

#[test]
fn parse_option_ids_of_wrong_length_is_inconsistent() {
    let v = bars_with_ids(&serde_json::json!(["h"]), &serde_json::json!(["m", "v"]));
    assert_that!(parse_kind(&v)).is_equal_to(crate::ChartErrorKind::Inconsistent);
}

#[test]
fn parse_option_columns_of_wrong_length_is_inconsistent() {
    let v = bars_with_ids(&serde_json::json!(["h", "a"]), &serde_json::json!(["m", "v", "w"]));
    assert_that!(parse_kind(&v)).is_equal_to(crate::ChartErrorKind::Inconsistent);
}

#[test]
fn parse_option_duplicate_ids_is_inconsistent() {
    let v = bars_with_ids(&serde_json::json!(["h", "a"]), &serde_json::json!(["m", "m"]));
    assert_that!(parse_kind(&v)).is_equal_to(crate::ChartErrorKind::Inconsistent);
}

#[test]
fn parse_option_invalid_id_is_inconsistent() {
    for bad in [serde_json::json!("a b"), serde_json::json!(""), serde_json::json!("x".repeat(65))] {
        let v = bars_with_ids(&serde_json::json!(["h", bad.clone()]), &serde_json::json!(["m", "v"]));
        asserting(&format!("ligne {bad}")).that(&parse_kind(&v)).is_equal_to(crate::ChartErrorKind::Inconsistent);
        let v = bars_with_ids(&serde_json::json!(["h", "a"]), &serde_json::json!(["m", bad.clone()]));
        asserting(&format!("colonne {bad}")).that(&parse_kind(&v)).is_equal_to(crate::ChartErrorKind::Inconsistent);
    }
}

#[test]
fn parse_option_id_of_64_chars_is_accepted() {
    let v = bars_with_ids(&serde_json::json!(["h", "x".repeat(64)]), &serde_json::json!(["m", "y".repeat(64)]));
    assert_that!(parse_option(&v)).is_ok();
}

#[test]
fn parse_option_without_ids_is_accepted() {
    assert_that!(parse_option(&bars_with_ids(&serde_json::json!([]), &serde_json::json!([])))).is_ok();
}

fn cartesian_of(v: serde_json::Value) -> Result<Cartesian> {
    match normalize(&option(v), ChartLabels::english())?.kind {
        ChartKind::Cartesian(c) => Ok(c),
        other => panic!("cartésien attendu : {other:?}"),
    }
}

fn stacked_bars(y_axis: &serde_json::Value, gap: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "dataset": { "source": [["Q", "Nord", "Sud"], ["T1", 10, 4], ["T2", null, null], ["T3", 12.5, 6]] },
        "xAxis": { "type": "category" }, "yAxis": y_axis,
        "series": [{ "type": "bar", "stack": "s", "barCategoryGap": gap }, { "type": "bar", "stack": "s" }]
    })
}

#[test]
fn normalize_value_interval_rounds_the_stacked_maximum_to_a_tick() {
    let c = cartesian_of(stacked_bars(&serde_json::json!({ "type": "value", "interval": 5 }), None)).unwrap();
    assert_that!((c.value_min, c.value_max, c.value_splits)).is_equal_to((Some(0.0), Some(20.0), Some(4)));
}

#[test]
fn normalize_bar_category_gap_reaches_the_model() {
    let c = cartesian_of(stacked_bars(&serde_json::json!({ "type": "value" }), Some("40%"))).unwrap();
    assert_that!(c.bar_gap).is_equal_to(Some(Length::Percent("40%".into())));
}

#[rstest::rstest]
#[case::hundred(serde_json::json!("100%"))]
#[case::negative(serde_json::json!(-3))]
#[case::not_a_percent(serde_json::json!("wide"))]
fn normalize_bar_category_gap_out_of_range_is_refused(#[case] gap: serde_json::Value) {
    let mut v = stacked_bars(&serde_json::json!({ "type": "value" }), None);
    v["series"][0]["barCategoryGap"] = gap;
    assert_that!(cartesian_of(v).err().map(|e| e.kind())).is_equal_to(Some(ChartErrorKind::Inconsistent));
}

#[rstest::rstest]
#[case::no_gap(serde_json::json!("0%"))]
#[case::almost_all(serde_json::json!("99%"))]
#[case::points_beyond_a_hundred(serde_json::json!(120))]
fn normalize_bar_category_gap_in_range_is_accepted(#[case] gap: serde_json::Value) {
    let mut v = stacked_bars(&serde_json::json!({ "type": "value" }), None);
    v["series"][0]["barCategoryGap"] = gap;
    assert_that!(cartesian_of(v)).is_ok();
}

#[test]
fn normalize_bar_category_gap_on_a_line_is_refused() {
    let mut v = stacked_bars(&serde_json::json!({ "type": "value" }), None);
    v["series"] = serde_json::json!([{ "type": "line", "barCategoryGap": "20%" }]);
    assert_that!(cartesian_of(v).err().map(|e| e.kind())).is_equal_to(Some(ChartErrorKind::Inconsistent));
}

#[rstest::rstest]
#[case::absent(serde_json::json!({ "type": "value" }), true)]
#[case::hidden(serde_json::json!({ "type": "value", "splitLine": { "show": false } }), false)]
fn normalize_split_line_sets_grid_show(#[case] y_axis: serde_json::Value, #[case] shown: bool) {
    let model = normalize(&option(stacked_bars(&y_axis, None)), ChartLabels::english()).unwrap();
    assert_that!(model.grid_show).is_equal_to(shown);
}

#[test]
fn normalize_horizontal_bars_read_the_grid_from_the_x_axis() {
    let v = serde_json::json!({
        "dataset": { "source": [["Q", "Nord"], ["T1", 10]] },
        "xAxis": { "type": "value", "splitLine": { "show": false } }, "yAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    });
    assert_that!(normalize(&option(v), ChartLabels::english()).unwrap().grid_show).is_false();
}
