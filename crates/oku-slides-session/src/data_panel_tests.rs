use oku_slides_a2ui::ComponentKind;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{chart_data, hex_color, table_data};

#[rstest]
#[case::short(json!("#ABC"), Some("#aabbcc"))]
#[case::long(json!(" #1F4E79 "), Some("#1f4e79"))]
#[case::named(json!("red"), None)]
#[case::bad_length(json!("#abcd"), None)]
#[case::not_hex(json!("#ggg"), None)]
#[case::not_text(json!(3), None)]
fn hex_color_normalizes_or_refuses(#[case] value: Value, #[case] expected: Option<&str>) {
    assert_that!(hex_color(&value)).is_equal_to(expected.map(str::to_owned));
}

#[test]
fn chart_data_scatter_names_series_by_their_first_column() {
    let option = json!({ "dataset": { "source": [["x1", "y1", "x2", "y2"]] },
        "series": [{ "type": "scatter" }, { "type": "scatter" }] });
    let ComponentKind::ChartData { series, .. } = chart_data(&option, &[]) else { panic!("ChartData attendu") };
    let names: Vec<String> = series.into_iter().map(|s| s.name).collect();
    assert_that!(names).is_equal_to(vec!["y1".to_owned(), "y2".to_owned()]);
}

#[test]
fn chart_data_pie_has_no_colourable_series() {
    let option = json!({ "dataset": { "source": [["M", "V"]] }, "series": [{ "type": "pie" }] });
    let ComponentKind::ChartData { series, .. } = chart_data(&option, &[]) else { panic!("ChartData attendu") };
    assert_that!(series).is_empty();
}

#[test]
fn chart_data_shared_stack_group_is_stacked() {
    let option = json!({ "series": [{ "type": "bar", "stack": "total" }, { "type": "line", "stack": "total" }] });
    let ComponentKind::ChartData { stacked, .. } = chart_data(&option, &[]) else { panic!("ChartData attendu") };
    assert_that!(stacked).is_equal_to(Some(true));
}

#[test]
fn chart_data_keeps_only_valid_palette_colours() {
    let ComponentKind::ChartData { palette, .. } = chart_data(&json!({ "series": [] }), &["#ABC".into(), "x".into()])
    else {
        panic!("ChartData attendu")
    };
    assert_that!(palette).is_equal_to(vec!["#aabbcc".to_owned()]);
}

#[test]
fn table_data_without_flags_takes_the_defaults() {
    let ComponentKind::TableData { header_row, header_column, striped, align, .. } =
        table_data(&json!({ "dataset": { "source": [["A"]] } }))
    else {
        panic!("TableData attendu")
    };
    assert_that!((header_row, header_column, striped, align.len())).is_equal_to((true, false, false, 0));
}
