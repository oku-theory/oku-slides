use oku_slides_a2ui::{ComponentKind, DataRef};
use oku_slides_data::{DataCell, Dataset};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{chart_grid, hex_color, table_grid};

/// A dataset with header `header`, one row `r1` and column ids `c0`, `c1`, ...
fn dataset(header: &[&str]) -> Dataset {
    Dataset {
        source: vec![
            header.iter().map(|h| DataCell::Text((*h).to_owned())).collect(),
            vec![DataCell::Number(1.0); header.len()],
        ],
        ids: vec![String::new(), "r1".to_owned()],
        columns: (0..header.len()).map(|i| format!("c{i}")).collect(),
    }
}

/// The data the grids show.
fn data() -> DataRef {
    DataRef { id: "d1".into(), name: "ventes".into(), label: None, users: 1 }
}

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
fn chart_grid_scatter_names_series_by_their_first_column() {
    let option = json!({ "series": [{ "type": "scatter" }, { "type": "scatter" }] });
    let ComponentKind::ChartData { series, .. } =
        chart_grid(&option, &dataset(&["x1", "y1", "x2", "y2"]), &[], data(), Vec::new(), None)
    else {
        panic!("ChartData expected")
    };
    let names: Vec<String> = series.into_iter().map(|s| s.name).collect();
    assert_that!(names).is_equal_to(vec!["y1".to_owned(), "y2".to_owned()]);
}

#[test]
fn chart_grid_pie_has_no_colourable_series() {
    let option = json!({ "series": [{ "type": "pie" }] });
    let ComponentKind::ChartData { series, .. } =
        chart_grid(&option, &dataset(&["M", "V"]), &[], data(), Vec::new(), None)
    else {
        panic!("ChartData expected")
    };
    assert_that!(series).is_empty();
}

#[test]
fn chart_grid_shared_stack_group_is_stacked() {
    let option = json!({ "series": [{ "type": "bar", "stack": "total" }, { "type": "line", "stack": "total" }] });
    let ComponentKind::ChartData { stacked, .. } =
        chart_grid(&option, &dataset(&["M", "A", "B"]), &[], data(), Vec::new(), None)
    else {
        panic!("ChartData expected")
    };
    assert_that!(stacked).is_equal_to(Some(true));
}

#[test]
fn chart_grid_keeps_only_valid_palette_colours() {
    let ComponentKind::ChartData { palette, .. } = chart_grid(
        &json!({ "series": [] }),
        &Dataset::default(),
        &["#ABC".into(), "x".into()],
        data(),
        Vec::new(),
        None,
    ) else {
        panic!("ChartData expected")
    };
    assert_that!(palette).is_equal_to(vec!["#aabbcc".to_owned()]);
}

#[test]
fn table_grid_without_flags_takes_the_defaults() {
    let ComponentKind::TableData { header_row, header_column, striped, align, .. } =
        table_grid(&json!({}), &dataset(&["A"]), data(), Vec::new(), None)
    else {
        panic!("TableData expected")
    };
    assert_that!((header_row, header_column, striped, align.len())).is_equal_to((true, false, false, 0));
}

#[test]
fn chart_grid_carries_the_row_and_column_ids_without_the_header() {
    let ComponentKind::ChartData { row_ids, column_ids, .. } =
        chart_grid(&json!({ "series": [{ "type": "bar" }] }), &dataset(&["M", "V"]), &[], data(), Vec::new(), None)
    else {
        panic!("ChartData expected")
    };
    assert_that!((row_ids, column_ids)).is_equal_to((vec!["r1".to_owned()], vec!["c0".to_owned(), "c1".to_owned()]));
}

/// `ui-data-shared-by` with `count`, its bidi isolation marks removed.
fn shared_by(locale: &str, count: usize) -> String {
    let text = oku_slides_i18n::Localizer::new(Some(locale)).tr_with("ui-data-shared-by", &[("count", count.into())]);
    text.chars().filter(|c| !matches!(c, '\u{2068}' | '\u{2069}')).collect()
}

#[test]
fn ui_data_shared_by_formats_in_all_locales() {
    for locale in oku_slides_i18n::available() {
        let l = oku_slides_i18n::Localizer::new(Some(locale));
        assert_that!(l.has("ui-data-shared-by")).named(locale).is_true();
        let text = shared_by(locale, 3);
        assert_that!((text.contains('{'), text.contains('3')))
            .named(&format!("{locale}: {text}"))
            .is_equal_to((false, true));
    }
    assert_that!(shared_by("fr", 3)).is_not_equal_to(shared_by("en", 3));
}

/// The pane names how many views share the data, in the plural form `count` asks for.
#[rstest]
#[case::ru_1("ru", 1, "1 представление")]
#[case::ru_2("ru", 2, "2 представления")]
#[case::ru_5("ru", 5, "5 представлений")]
#[case::ru_21("ru", 21, "21 представление")]
#[case::ru_22("ru", 22, "22 представления")]
#[case::pl_1("pl", 1, "1 widok")]
#[case::pl_2("pl", 2, "2 widoki")]
#[case::pl_5("pl", 5, "5 widoków")]
fn ui_data_shared_by_selects_its_plural_form(#[case] locale: &str, #[case] count: usize, #[case] form: &str) {
    let text = shared_by(locale, count);
    assert_that!(text.contains(form)).named(&text).is_true();
}
