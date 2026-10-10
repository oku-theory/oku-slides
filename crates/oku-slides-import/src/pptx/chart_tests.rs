use speculoos::prelude::*;

use super::*;
use crate::pptx::test_support::{BAR, slide_pkg, with_ctx};
use crate::pptx::xml::parse;
use crate::source::{ChartKind, Grouping, Warning, WarningKind};

fn read(xml: &str) -> (Result<Chart, ChartSkip>, Vec<Warning>) {
    let t = slide_pkg(&[]);
    let pkg = t.pkg();
    let doc = parse(xml, "t", 100_000).unwrap();
    let (r, w, _) = with_ctx(&pkg, "ppt/slides/slide1.xml", |cx| parse_chart(doc.root_element(), cx, "s1/14"));
    (r, w)
}

fn chart(xml: &str) -> Chart {
    read(xml).0.unwrap_or_else(|e| panic!("graphique attendu : {e:?}"))
}

fn space(inner: &str) -> String {
    format!(
        r#"<c:chartSpace xmlns:c="urn:c" xmlns:a="urn:a"><c:chart><c:plotArea>{inner}</c:plotArea></c:chart></c:chartSpace>"#
    )
}

fn line_values(pt_count: &str, pts: &str) -> Vec<Option<f64>> {
    chart(&space(&format!(r#"<c:lineChart><c:ser><c:val><c:numLit><c:ptCount val="{pt_count}"/>{pts}</c:numLit></c:val></c:ser></c:lineChart>"#))).series[0].values.clone()
}

#[test]
fn bar_chart_reads_kind_grouping_and_title() {
    let c = chart(BAR);
    assert_that!((c.kind, c.grouping, c.title.as_deref())).is_equal_to((
        ChartKind::Column,
        Grouping::Stacked,
        Some("Ventes 2026"),
    ));
}

#[test]
fn series_read_cached_values_with_gaps_and_na() {
    let c = chart(BAR);
    assert_that!(c.categories.clone()).is_equal_to(vec!["T1".to_owned(), "T2".to_owned(), "T3".to_owned()]);
    assert_that!((c.series[0].name.as_str(), c.series[0].values.clone()))
        .is_equal_to(("Nord", vec![Some(10.0), None, Some(12.5)]));
    assert_that!(c.series[0].color.as_ref().map(|c| c.value.to_hex())).is_equal_to(Some("#4472c4".to_owned()));
    assert_that!((c.series[1].name.as_str(), c.series[1].values.clone()))
        .is_equal_to(("Sud", vec![Some(4.0), None, Some(6.0)]));
}

#[test]
fn combo_chart_keeps_first_type_and_warns_on_the_item() {
    let (_, w) = read(BAR);
    assert_that!(w.iter().any(
        |w| w.kind == WarningKind::UnsupportedChart("combo: barChart".into()) && w.item.as_deref() == Some("s1/14")
    ))
    .is_true();
}

#[test]
fn horizontal_bars_and_default_grouping() {
    let c = chart(&space(r#"<c:barChart><c:barDir val="bar"/></c:barChart>"#));
    assert_that!((c.kind, c.grouping, chart(&space("<c:lineChart/>")).grouping)).is_equal_to((
        ChartKind::Bar,
        Grouping::Clustered,
        Grouping::Standard,
    ));
}

#[test]
fn scatter_reads_x_and_y_values() {
    let c = chart(&space(
        r#"<c:scatterChart><c:ser><c:tx><c:v>S</c:v></c:tx><c:xVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:xVal><c:yVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>5</c:v></c:pt><c:pt idx="1"><c:v>7</c:v></c:pt></c:numLit></c:yVal></c:ser></c:scatterChart>"#,
    ));
    assert_that!((c.kind, c.categories.clone(), c.series[0].values.clone())).is_equal_to((
        ChartKind::Scatter,
        vec!["1".to_owned(), "2".to_owned()],
        vec![Some(5.0), Some(7.0)],
    ));
}

#[test]
fn unreadable_point_count_follows_the_points_present() {
    assert_that!(line_values("9223372036854775807", r#"<c:pt idx="0"><c:v>1</c:v></c:pt>"#).len()).is_equal_to(1);
}

#[test]
fn huge_point_count_is_bounded_by_the_points_present() {
    // 1 point présent : au plus 1 + 64 cases, quel que soit ptCount.
    assert_that!(line_values("900000", r#"<c:pt idx="0"><c:v>1</c:v></c:pt>"#).len()).is_equal_to(1 + MAX_TRAILING_GAP);
}

#[test]
fn point_beyond_the_bounded_length_is_ignored() {
    let v = line_values("3", r#"<c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="20000"><c:v>9</c:v></c:pt>"#);
    assert_that!((v.len(), v[0], v.iter().flatten().count())).is_equal_to((2 + MAX_TRAILING_GAP, Some(1.0), 1));
}

#[test]
fn series_beyond_the_cap_are_dropped() {
    let sers =
        r#"<c:ser><c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>"#
            .repeat(MAX_SERIES + 10);
    assert_that!(chart(&space(&format!("<c:lineChart>{sers}</c:lineChart>"))).series.len()).is_equal_to(MAX_SERIES);
}

#[test]
fn unsupported_type_is_skipped_with_its_name() {
    assert_that!(read(&space("<c:bubbleChart/>")).0).is_equal_to(Err(ChartSkip::Unsupported("bubbleChart".to_owned())));
}

#[test]
fn chart_without_plot_area_is_malformed() {
    assert_that!(read(r#"<c:chartSpace xmlns:c="urn:c"/>"#).0).is_equal_to(Err(ChartSkip::Malformed("plotArea")));
}
