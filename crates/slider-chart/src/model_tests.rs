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
