use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::data_ops::ChartDataOp;

fn opt(v: serde_json::Value) -> ChartOption {
    serde_json::from_value(v).unwrap()
}

#[test]
fn canonicalize_inline_cartesian_moves_to_dataset_with_same_model() {
    let inline = opt(json!({
        "xAxis": { "type": "category", "data": ["Jan", "Fév", "Mar"] },
        "yAxis": { "type": "value" },
        "series": [
            { "type": "bar", "name": "Ventes", "data": [10, 12, null] },
            { "type": "line", "name": "Objectif", "data": [11, 11, 11] }
        ]
    }));
    let canon = canonicalize(&inline).unwrap();
    assert_that!(canon.series.iter().all(|s| s.data.is_none())).is_true();
    let before = normalize(&inline).unwrap();
    let after = normalize(&canon).unwrap();
    let (ChartKind::Cartesian(b), ChartKind::Cartesian(a)) = (&before.kind, &after.kind) else {
        panic!("cartésien attendu")
    };
    assert_that!(a.categories).is_equal_to(&b.categories);
    assert_that!(a.series).is_equal_to(&b.series);
}

/// Noms et valeurs, sans les libellés de colonnes ajoutés par la canonisation.
fn values(kind: &ChartKind) -> Vec<(String, Vec<Option<f64>>)> {
    match kind {
        ChartKind::Cartesian(c) => c.series.iter().map(|s| (s.name.clone(), s.values.clone())).collect(),
        ChartKind::Pie(p) => p.slices.iter().map(|(n, v)| (n.clone(), vec![*v])).collect(),
        ChartKind::Scatter(sc) => sc
            .series
            .iter()
            .map(|s| (s.name.clone(), s.points.iter().flat_map(|(x, y)| [Some(*x), Some(*y)]).collect()))
            .collect(),
        ChartKind::Radar(r) => r
            .series
            .iter()
            .map(|s| (s.name.clone(), s.values.clone()))
            .chain(r.indicators.iter().map(|(n, m)| (n.clone(), vec![Some(*m)])))
            .collect(),
    }
}

#[test]
fn canonicalize_inline_pie_scatter_radar_keeps_their_values() {
    for v in [
        json!({ "series": [{ "type": "pie", "name": "Parts",
            "data": [{ "name": "A", "value": 3 }, { "name": "B", "value": 1 }] }] }),
        json!({ "series": [{ "type": "scatter", "name": "Mesures", "data": [[1, 2], [3, 4]] }] }),
        json!({ "radar": { "indicator": [{ "name": "Vitesse", "max": 10 }, { "name": "Coût" }] },
            "series": [{ "type": "radar", "data": [{ "name": "Offre A", "value": [7, 4] }] }] }),
    ] {
        let o = opt(v);
        let canon = canonicalize(&o).unwrap();
        assert_that!(canon.dataset).is_some();
        assert_that!(values(&normalize(&canon).unwrap().kind)).is_equal_to(values(&normalize(&o).unwrap().kind));
    }
}

#[test]
fn normalize_renamed_column_renames_the_series() {
    let mut o = canonicalize(&opt(json!({
        "xAxis": { "data": ["Jan"] },
        "series": [{ "type": "bar", "name": "Ventes", "data": [10] }]
    })))
    .unwrap();
    ChartDataOp::SetCell { row: 0, col: 1, value: Cell::Text("CA".into()) }.apply(o.dataset.as_mut().unwrap()).unwrap();
    let ChartKind::Cartesian(c) = normalize(&o).unwrap().kind else { panic!() };
    assert_that!(c.series[0].name.as_str()).is_equal_to("CA");
}

#[test]
fn sync_series_with_columns_added_column_adds_a_series() {
    let mut o = opt(json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 10]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "line" }]
    }));
    ChartDataOp::InsertColumn { at: 2, cells: vec![Cell::Text("Objectif".into()), Cell::Number(12.0)] }
        .apply(o.dataset.as_mut().unwrap())
        .unwrap();
    sync_series_with_columns(&mut o);
    assert_that!(o.series).has_length(2);
    assert_that!(normalize(&o)).is_ok();
}
