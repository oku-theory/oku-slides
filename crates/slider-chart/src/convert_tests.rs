use serde_json::json;
use speculoos::prelude::*;

use super::convert;
use crate::labels::ChartLabels;
use crate::model::{ChartKind, normalize};
use crate::option::{Cell, ChartOption, SeriesType};

fn opt(v: serde_json::Value) -> ChartOption {
    serde_json::from_value(v).unwrap()
}

fn bars() -> ChartOption {
    opt(json!({
        "title": { "text": "Ventes" },
        "dataset": { "source": [["Mois", "Nord", "Sud"], ["Jan", 3, 4], ["Fév", 5, null], ["Mar", 7, 6]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }, { "type": "bar" }]
    }))
}

fn en() -> &'static ChartLabels {
    ChartLabels::english()
}

#[test]
fn convert_bars_to_radar_keeps_the_table_and_the_title() {
    let radar = convert(&bars(), SeriesType::Radar, en()).unwrap();
    assert_that!(radar.dataset).is_equal_to(bars().dataset);
    assert_that!(radar.title).is_equal_to(bars().title);
    let ChartKind::Radar(r) = normalize(&radar, en()).unwrap().kind else { panic!("radar attendu") };
    assert_that!(r.indicators.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>())
        .is_equal_to(vec!["Jan", "Fév", "Mar"]);
    assert_that!(r.series.len()).is_equal_to(2);
}

#[test]
fn convert_bars_to_scatter_pairs_each_series_with_numbered_x() {
    let scatter = convert(&bars(), SeriesType::Scatter, en()).unwrap();
    let ChartKind::Scatter(s) = normalize(&scatter, en()).unwrap().kind else { panic!("nuage attendu") };
    assert_that!(s.series[0].name.as_str()).is_equal_to("Nord");
    assert_that!(s.series[0].points).is_equal_to(vec![(1.0, 3.0), (2.0, 5.0), (3.0, 7.0)]);
    // Une valeur absente n'est pas un point.
    assert_that!(s.series[1].points).is_equal_to(vec![(1.0, 4.0), (3.0, 6.0)]);
    let header = &scatter.dataset.as_ref().unwrap().source[0];
    assert_that!(header[0].clone()).is_equal_to(Cell::Text("Nord (x)".into()));
}

#[test]
fn convert_scatter_back_to_lines_uses_x_as_categories() {
    let scatter = convert(&bars(), SeriesType::Scatter, en()).unwrap();
    let lines = convert(&scatter, SeriesType::Line, en()).unwrap();
    let ChartKind::Cartesian(c) = normalize(&lines, en()).unwrap().kind else { panic!("lignes attendues") };
    assert_that!(c.categories).is_equal_to(vec!["1".to_owned(), "2".to_owned(), "3".to_owned()]);
    assert_that!(c.series[0].values).is_equal_to(vec![Some(3.0), Some(5.0), Some(7.0)]);
    assert_that!(c.series[1].values).is_equal_to(vec![Some(4.0), None, Some(6.0)]);
    // Le nuage n'a pas de colonne de libellés : l'en-tête par défaut la nomme.
    assert_that!(c.category_label.as_str()).is_equal_to("Category");
}

#[test]
fn convert_to_pie_keeps_the_first_series() {
    let pie = convert(&bars(), SeriesType::Pie, en()).unwrap();
    let ChartKind::Pie(p) = normalize(&pie, en()).unwrap().kind else { panic!("secteurs attendus") };
    assert_that!(p.slices).is_equal_to(vec![
        ("Jan".to_owned(), Some(3.0)),
        ("Fév".to_owned(), Some(5.0)),
        ("Mar".to_owned(), Some(7.0)),
    ]);
    assert_that!(pie.x_axis).is_none();
}

#[test]
fn convert_scatter_with_numeric_categories_keeps_them_as_x() {
    let lines = opt(json!({
        "dataset": { "source": [["Année", "CA"], ["2024", 1], ["2026", 3]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "line" }]
    }));
    let scatter = convert(&lines, SeriesType::Scatter, en()).unwrap();
    let ChartKind::Scatter(s) = normalize(&scatter, en()).unwrap().kind else { panic!("nuage attendu") };
    assert_that!(s.series[0].points).is_equal_to(vec![(2024.0, 1.0), (2026.0, 3.0)]);
}

#[test]
fn convert_every_kind_to_every_kind_stays_drawable() {
    let kinds = [SeriesType::Bar, SeriesType::Line, SeriesType::Pie, SeriesType::Scatter, SeriesType::Radar];
    for from in kinds {
        let start = convert(&bars(), from, en()).unwrap();
        for to in kinds {
            let converted = convert(&start, to, en());
            asserting(&format!("{from:?} → {to:?}")).that(&converted.is_ok()).is_true();
        }
    }
}

#[test]
fn convert_stacked_bars_to_lines_keeps_the_stack() {
    let mut stacked = bars();
    for s in &mut stacked.series {
        s.stack = Some("total".into());
    }
    let lines = convert(&stacked, SeriesType::Line, en()).unwrap();
    assert_that!(lines.series.iter().map(|s| s.stack.as_deref()).collect::<Vec<_>>())
        .is_equal_to(vec![Some("total"), Some("total")]);
}

#[test]
fn convert_unstacked_bars_to_lines_stays_unstacked() {
    let lines = convert(&bars(), SeriesType::Line, en()).unwrap();
    assert_that!(lines.series.iter().all(|s| s.stack.is_none())).is_true();
}
