use oku_slides_chart::{ChartLabels, ChartModel, Length, canonicalize, normalize, parse_option};
use oku_slides_core::kinds::base;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{apply_blocks, element_with};

fn series(name: &str, values: &[f64]) -> source::Series {
    source::Series {
        name: name.to_owned(),
        values: values.iter().copied().map(Some).collect(),
        color: None,
        x: None,
        format: None,
    }
}

fn source_chart(kind: ChartKind, grouping: Grouping) -> source::Chart {
    source::Chart {
        kind,
        grouping,
        title: Some("Ventes".to_owned()),
        categories: vec!["T1".into(), "T2".into(), "T3".into()],
        series: vec![series("2025", &[1.0, 2.0, 3.0]), series("2026", &[4.0, 5.0, 6.0])],
    }
}

fn option_of(c: &source::Chart) -> Value {
    chart(c).unwrap().0
}

/// Le modèle que le cœur tire des options : elles passent `parse_option`.
fn model(c: &source::Chart) -> ChartModel {
    let option = parse_option(&option_of(c)).unwrap();
    normalize(&option, ChartLabels::english()).unwrap()
}

/// Ce que le cœur lit d'un graphique cartésien.
#[derive(Debug, PartialEq)]
struct Read {
    horizontal: bool,
    area: bool,
    categories: Vec<String>,
    names: Vec<String>,
    stacks: Vec<Option<String>>,
}

fn cartesian(c: &source::Chart) -> Read {
    match model(c).kind {
        oku_slides_chart::ChartKind::Cartesian(x) => Read {
            horizontal: x.horizontal,
            area: x.area,
            categories: x.categories,
            names: x.series.iter().map(|s| s.name.clone()).collect(),
            stacks: x.series.iter().map(|s| s.stack.clone()).collect(),
        },
        other => panic!("cartésien attendu : {other:?}"),
    }
}

#[test]
fn chart_of_columns_is_the_canonical_bar_option() {
    let (option, losses) = chart(&source_chart(ChartKind::Column, Grouping::Clustered)).unwrap();
    assert_that!(option).is_equal_to(json!({
        "dataset": { "source": [["", "2025", "2026"], ["T1", 1.0, 4.0], ["T2", 2.0, 5.0], ["T3", 3.0, 6.0]] },
        "xAxis": { "type": "category" },
        "yAxis": { "type": "value" },
        "series": [{ "type": "bar" }, { "type": "bar" }],
        "title": { "text": "Ventes" },
    }));
    assert_that!(losses).is_empty();
}

/// Déjà canonique : `canonicalize` rend les options telles quelles.
#[test]
fn chart_option_is_already_canonical() {
    let option = parse_option(&option_of(&source_chart(ChartKind::Line, Grouping::Standard))).unwrap();
    assert_that!(canonicalize(&option, ChartLabels::english()).unwrap()).is_equal_to(&option);
}

#[rstest]
#[case::column(ChartKind::Column, "bar", false, false)]
#[case::bar(ChartKind::Bar, "bar", true, false)]
#[case::line(ChartKind::Line, "line", false, false)]
#[case::area(ChartKind::Area, "line", false, true)]
fn chart_maps_cartesian_kinds(
    #[case] kind: ChartKind,
    #[case] series_type: &str,
    #[case] horizontal: bool,
    #[case] area: bool,
) {
    let c = source_chart(kind, Grouping::Clustered);
    assert_that!(option_of(&c)["series"][0]["type"].as_str()).is_equal_to(Some(series_type));
    let read = cartesian(&c);
    assert_that!((read.horizontal, read.area)).is_equal_to((horizontal, area));
    assert_that!(read.categories).is_equal_to(vec!["T1".to_owned(), "T2".to_owned(), "T3".to_owned()]);
    assert_that!(read.names).is_equal_to(vec!["2025".to_owned(), "2026".to_owned()]);
}

#[rstest]
#[case(Grouping::Stacked, vec![])]
#[case(Grouping::PercentStacked, vec![LossKind::PercentStacked])]
fn chart_stacks_series_of_a_stacked_chart(#[case] grouping: Grouping, #[case] losses: Vec<LossKind>) {
    let c = source_chart(ChartKind::Column, grouping);
    assert_that!(chart(&c).unwrap().1).is_equal_to(losses);
    assert_that!(cartesian(&c).stacks).is_equal_to(vec![Some(STACK.to_owned()); 2]);
}

#[test]
fn chart_of_clustered_columns_does_not_stack() {
    assert_that!(cartesian(&source_chart(ChartKind::Column, Grouping::Clustered)).stacks).is_equal_to(vec![None, None]);
}

#[test]
fn chart_of_a_pie_keeps_the_first_series() {
    match model(&source_chart(ChartKind::Pie, Grouping::Clustered)).kind {
        oku_slides_chart::ChartKind::Pie(p) => {
            assert_that!(p.slices).is_equal_to(vec![
                ("T1".to_owned(), Some(1.0)),
                ("T2".to_owned(), Some(2.0)),
                ("T3".to_owned(), Some(3.0)),
            ]);
            assert_that!(p.inner).is_none();
        }
        other => panic!("secteurs attendus : {other:?}"),
    }
}

#[rstest]
#[case(ChartKind::Pie)]
#[case(ChartKind::Doughnut)]
fn chart_of_a_pie_reports_the_dropped_series(#[case] kind: ChartKind) {
    let mut c = source_chart(kind, Grouping::Clustered);
    c.series.push(series("2027", &[7.0, 8.0, 9.0]));
    assert_that!(chart(&c).unwrap().1).is_equal_to(vec![LossKind::SeriesDropped { kept: 1, dropped: 2 }]);
}

#[test]
fn chart_of_a_single_series_pie_loses_nothing() {
    let mut c = source_chart(ChartKind::Pie, Grouping::Clustered);
    c.series.truncate(1);
    assert_that!(chart(&c).unwrap().1).is_empty();
}

#[test]
fn chart_of_a_doughnut_is_a_pie_with_two_radii() {
    match model(&source_chart(ChartKind::Doughnut, Grouping::Clustered)).kind {
        oku_slides_chart::ChartKind::Pie(p) => {
            assert_that!(p.inner).is_equal_to(Some(Length::Percent("40%".into())));
            assert_that!(p.outer).is_equal_to(Some(Length::Percent("70%".into())));
        }
        other => panic!("secteurs attendus : {other:?}"),
    }
}

#[test]
fn chart_of_a_scatter_reads_numeric_categories_as_x() {
    let mut c = source_chart(ChartKind::Scatter, Grouping::Clustered);
    c.categories = vec!["0.5".into(), "1".into(), "2".into()];
    c.series[1].values[1] = None;
    match model(&c).kind {
        oku_slides_chart::ChartKind::Scatter(s) => {
            assert_that!(s.series[0].points).is_equal_to(vec![(0.5, 1.0), (1.0, 2.0), (2.0, 3.0)]);
            assert_that!(s.series[1].points).is_equal_to(vec![(0.5, 4.0), (2.0, 6.0)]);
        }
        other => panic!("nuage attendu : {other:?}"),
    }
}

#[test]
fn chart_of_a_scatter_takes_each_series_own_x() {
    let mut c = source_chart(ChartKind::Scatter, Grouping::Clustered);
    c.series[0].x = Some(vec![Some(10.0), Some(20.0), Some(30.0)]);
    // La seconde série a quatre points et un X absent : ce point-là n'est pas tracé.
    c.series[1].values.push(Some(7.0));
    c.series[1].x = Some(vec![Some(-1.0), None, Some(0.25), Some(8.0)]);
    match model(&c).kind {
        oku_slides_chart::ChartKind::Scatter(s) => {
            assert_that!(s.series[0].points).is_equal_to(vec![(10.0, 1.0), (20.0, 2.0), (30.0, 3.0)]);
            assert_that!(s.series[1].points).is_equal_to(vec![(-1.0, 4.0), (0.25, 6.0), (8.0, 7.0)]);
        }
        other => panic!("nuage attendu : {other:?}"),
    }
}

#[test]
fn chart_with_a_number_format_loses_it() {
    let mut c = source_chart(ChartKind::Column, Grouping::Clustered);
    c.series[1].format = Some("0%".to_owned());
    let (_, losses) = chart(&c).unwrap();
    assert_that!(losses).is_equal_to(vec![LossKind::ChartFormat("0%".to_owned())]);
}

#[test]
fn chart_of_a_scatter_numbers_text_categories() {
    match model(&source_chart(ChartKind::Scatter, Grouping::Clustered)).kind {
        oku_slides_chart::ChartKind::Scatter(s) => {
            assert_that!(s.series[0].points).is_equal_to(vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
        }
        other => panic!("nuage attendu : {other:?}"),
    }
}

#[test]
fn chart_of_a_radar_takes_categories_as_axes() {
    match model(&source_chart(ChartKind::Radar, Grouping::Standard)).kind {
        oku_slides_chart::ChartKind::Radar(r) => {
            assert_that!(r.indicators.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>())
                .is_equal_to(vec!["T1", "T2", "T3"]);
            assert_that!(r.series).has_length(2);
        }
        other => panic!("radar attendu : {other:?}"),
    }
}

#[test]
fn chart_numbers_missing_categories_and_empties_missing_values() {
    let mut c = source_chart(ChartKind::Column, Grouping::Clustered);
    c.categories = vec!["A".into()];
    c.series[0].values = vec![Some(1.0), None, Some(f64::NAN)];
    let source = &option_of(&c)["dataset"]["source"];
    assert_that!(source).is_equal_to(&json!([
        ["", "2025", "2026"],
        ["A", 1.0, 4.0],
        ["2", null, 5.0],
        ["3", null, 6.0]
    ]));
}

#[test]
fn chart_without_title_has_no_title() {
    let mut c = source_chart(ChartKind::Column, Grouping::Clustered);
    c.title = Some("  ".into());
    assert_that!(option_of(&c).get("title")).is_none();
}

#[test]
fn chart_without_series_is_none() {
    let mut c = source_chart(ChartKind::Column, Grouping::Clustered);
    c.series.clear();
    assert_that!(chart(&c)).is_none();
}

#[rstest]
fn chart_element_of_every_kind_is_accepted_by_add_slide(
    #[values(
        ChartKind::Column,
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Area,
        ChartKind::Pie,
        ChartKind::Doughnut,
        ChartKind::Scatter,
        ChartKind::Radar
    )]
    kind: ChartKind,
) {
    let option = option_of(&source_chart(kind, Grouping::Stacked));
    assert_that!(apply_blocks(vec![element_with("c", base::CHART, "option", option)])).is_ok();
}

/// Au-delà des bornes de relecture du cœur, les lignes sont coupées à
/// l'émission (en-tête compris) avec la perte `ChartTruncated` (t-817).
#[rstest]
#[case(ChartKind::Column, 2)]
#[case(ChartKind::Scatter, 2)]
fn chart_beyond_the_replica_row_budget_is_cut_with_a_loss(#[case] kind: ChartKind, #[case] columns: usize) {
    // Le plus que lit le lecteur : une ligne de plus que le budget, en-tête compris.
    let points = 10_000;
    let values: Vec<f64> = (0..points).map(|i| i as f64).collect();
    let mut c = source_chart(kind, Grouping::Standard);
    c.categories = Vec::new();
    c.series = vec![series("Ventes", &values)];
    let (option, losses) = chart(&c).unwrap();
    let rows = option["dataset"]["source"].as_array().unwrap();
    assert_that!(rows.len()).is_equal_to(oku_slides_core::dataset_row_budget(columns));
    assert_that!(losses).contains(LossKind::ChartTruncated);
}

#[test]
fn chart_within_the_replica_row_budget_keeps_every_row() {
    let (option, losses) = chart(&source_chart(ChartKind::Column, Grouping::Standard)).unwrap();
    assert_that!(option["dataset"]["source"].as_array().map(Vec::len)).is_equal_to(Some(4));
    assert_that!(losses).does_not_contain(LossKind::ChartTruncated);
}
