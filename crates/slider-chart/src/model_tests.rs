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
