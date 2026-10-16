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

#[test]
fn title_on_two_paragraphs_keeps_its_line_break() {
    let c =
        chart(&BAR.replace(
            "<a:t> 2026</a:t></a:r></a:p>",
            "<a:t> 2026</a:t></a:r></a:p><a:p><a:r><a:t>Nord</a:t></a:r></a:p>",
        ));
    assert_that!(c.title.as_deref()).is_equal_to(Some("Ventes 2026\nNord"));
}

const MULTI_LEVEL: &str = r#"<c:barChart><c:ser><c:cat><c:multiLvlStrRef><c:f>Feuil1!$A$2:$B$5</c:f><c:multiLvlStrCache><c:ptCount val="4"/><c:lvl><c:pt idx="0"><c:v>Jan</c:v></c:pt><c:pt idx="1"><c:v>Fév</c:v></c:pt><c:pt idx="2"><c:v>Jan</c:v></c:pt><c:pt idx="3"><c:v>Fév</c:v></c:pt></c:lvl><c:lvl><c:pt idx="0"><c:v>2025</c:v></c:pt><c:pt idx="2"><c:v>2026</c:v></c:pt></c:lvl></c:multiLvlStrCache></c:multiLvlStrRef></c:cat><c:val><c:numLit><c:ptCount val="4"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser></c:barChart>"#;

#[test]
fn multi_level_categories_read_their_leaf_level() {
    let c = chart(&space(MULTI_LEVEL));
    assert_that!(c.categories.clone()).is_equal_to(vec![
        "Jan".to_owned(),
        "Fév".to_owned(),
        "Jan".to_owned(),
        "Fév".to_owned(),
    ]);
}

#[test]
fn multi_level_categories_warn_that_outer_levels_are_dropped() {
    let (_, w) = read(&space(MULTI_LEVEL));
    assert_that!(w.iter().map(|w| w.kind.clone()).collect::<Vec<_>>())
        .is_equal_to(vec![WarningKind::Approximated("multiLvlStrRef".into())]);
}

/// Un cache numérique `numLit` des valeurs `v`, de format `code` s'il y en a un.
fn num_lit(code: Option<&str>, v: &[&str]) -> String {
    let format = code.map(|c| format!("<c:formatCode>{c}</c:formatCode>")).unwrap_or_default();
    let pts: String = v.iter().enumerate().map(|(i, v)| format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#)).collect();
    format!(r#"<c:numLit>{format}<c:ptCount val="{}"/>{pts}</c:numLit>"#, v.len())
}

#[test]
fn scatter_reads_numeric_x_of_each_series() {
    let ser = |x: &[&str], y: &[&str]| {
        format!("<c:ser><c:xVal>{}</c:xVal><c:yVal>{}</c:yVal></c:ser>", num_lit(None, x), num_lit(None, y))
    };
    let c = chart(&space(&format!(
        "<c:scatterChart>{}{}</c:scatterChart>",
        ser(&["1", "2.5"], &["5", "7"]),
        ser(&["-3", "x", "9"], &["1", "2", "3"])
    )));
    let xs: Vec<_> = c.series.iter().map(|s| s.x.clone()).collect();
    assert_that!(xs).is_equal_to(vec![Some(vec![Some(1.0), Some(2.5)]), Some(vec![Some(-3.0), None, Some(9.0)])]);
}

#[test]
fn scatter_with_text_x_keeps_them_as_categories_only() {
    let c = chart(&space(
        r#"<c:scatterChart><c:ser><c:xVal><c:strLit><c:ptCount val="1"/><c:pt idx="0"><c:v>Lundi</c:v></c:pt></c:strLit></c:xVal><c:yVal><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>3</c:v></c:pt></c:numLit></c:yVal></c:ser></c:scatterChart>"#,
    ));
    assert_that!((c.categories.clone(), c.series[0].x.clone())).is_equal_to((vec!["Lundi".to_owned()], None));
}

#[test]
fn line_values_ignore_x_even_when_numeric() {
    let c = chart(&space(&format!(
        "<c:lineChart><c:ser><c:cat>{}</c:cat><c:val>{}</c:val></c:ser></c:lineChart>",
        num_lit(None, &["2024", "2025"]),
        num_lit(None, &["1", "2"])
    )));
    assert_that!(c.series[0].x.clone()).is_none();
}

#[rstest::rstest]
#[case(Some("0%"), Some("0%"))]
#[case(Some("#,##0.00\\ \"€\""), Some("#,##0.00\\ \"€\""))]
#[case(Some("General"), None)]
#[case(Some("  "), None)]
#[case(None, None)]
fn series_read_the_format_code_of_their_values(#[case] code: Option<&str>, #[case] expected: Option<&str>) {
    let xml = space(&format!("<c:barChart><c:ser><c:val>{}</c:val></c:ser></c:barChart>", num_lit(code, &["0.5"])));
    assert_that!(chart(&xml).series[0].format.as_deref()).is_equal_to(expected);
}
