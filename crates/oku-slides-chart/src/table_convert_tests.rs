use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Cell, ChartErrorKind, ChartLabels, ChartOption, ColumnAlign, MAX_TABLE_COLUMNS, parse_table};

fn chart(v: serde_json::Value) -> ChartOption {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn chart_to_table_keeps_the_dataset_with_header_column() {
    let c = chart(json!({ "dataset": { "source": [["M", "A"], ["x", 1]] }, "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }] }));
    let t = chart_to_table(&c).unwrap();
    assert_that!((t.dataset.source.len(), t.header_row, t.header_column)).is_equal_to((2, true, true));
}

#[test]
fn chart_to_table_pads_short_rows() {
    let c = chart(json!({ "dataset": { "source": [["M", "A", "B"], ["x", 1]] }, "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }, { "type": "bar" }] }));
    assert_that!(chart_to_table(&c).unwrap().dataset.source[1][2]).is_equal_to(Cell::Null);
}

#[test]
fn chart_to_table_without_dataset_is_a_data_op_error() {
    let c =
        chart(json!({ "xAxis": { "type": "category", "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] }));
    assert_that!(chart_to_table(&c).unwrap_err().kind()).is_equal_to(ChartErrorKind::DataOp);
}

#[test]
fn chart_to_table_beyond_max_columns_is_inconsistent() {
    let header: Vec<String> = (0..=MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
    let series: Vec<_> = (1..=MAX_TABLE_COLUMNS).map(|_| json!({ "type": "bar" })).collect();
    let c = chart(json!({ "dataset": { "source": [header] }, "xAxis": { "type": "category" }, "series": series }));
    assert_that!(chart_to_table(&c).unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn table_to_chart_gets_one_bar_series_per_value_column() {
    let t = parse_table(&json!({ "dataset": { "source": [["M", "A", "B"], ["x", 1, 2]] } })).unwrap();
    let c = table_to_chart(&t, ChartLabels::english()).unwrap();
    assert_that!(c.series.len()).is_equal_to(2);
}

#[test]
fn table_to_chart_with_text_values_keeps_them_as_missing() {
    let t = parse_table(&json!({ "dataset": { "source": [["M", "A"], ["x", "beaucoup"]] },
        "align": [ColumnAlign::Start] }))
    .unwrap();
    assert_that!(table_to_chart(&t, ChartLabels::english())).is_ok();
}

fn line_chart() -> ChartOption {
    chart(json!({ "dataset": { "source": [["M", "A", "B"], ["x", 1, 2], ["y", 3, 4]] },
        "title": { "text": "Ventes" },
        "xAxis": { "type": "category" },
        "series": [
            { "type": "line", "stack": "s", "itemStyle": { "color": "#ff0000" } },
            { "type": "line", "stack": "s" }
        ] }))
}

#[test]
fn chart_to_table_keeps_the_chart_options_without_dataset() {
    let t = chart_to_table(&line_chart()).unwrap();
    let saved = t.chart.unwrap_or_else(|| panic!("chart absent"));
    assert_that!(saved.get("dataset")).is_none();
    assert_that!(saved["series"][0]["type"].clone()).is_equal_to(json!("line"));
    assert_that!(saved["title"]["text"].clone()).is_equal_to(json!("Ventes"));
}

#[test]
fn table_to_chart_restores_type_colors_and_stacking() {
    let t = chart_to_table(&line_chart()).unwrap();
    let c = table_to_chart(&t, ChartLabels::english()).unwrap();
    let v = serde_json::to_value(&c).unwrap();
    assert_that!(v["series"][0]["type"].clone()).is_equal_to(json!("line"));
    assert_that!(v["series"][1]["type"].clone()).is_equal_to(json!("line"));
    assert_that!(v["series"][0]["itemStyle"]["color"].clone()).is_equal_to(json!("#ff0000"));
    assert_that!((v["series"][0]["stack"].clone(), v["series"][1]["stack"].clone()))
        .is_equal_to((json!("s"), json!("s")));
}

#[test]
fn table_to_chart_after_adding_a_column_keeps_type_and_adds_a_series() {
    let t = chart_to_table(&line_chart()).unwrap();
    let t = crate::table_data(
        &t,
        &crate::DataOp::InsertColumn {
            at: 3,
            cells: vec![Cell::Text("C".into()), Cell::Number(1.0), Cell::Number(2.0)],
            id: None,
        },
    )
    .unwrap();
    let c = table_to_chart(&t, ChartLabels::english()).unwrap();
    let v = serde_json::to_value(&c).unwrap();
    let types: Vec<_> = v["series"].as_array().unwrap().iter().map(|s| s["type"].clone()).collect();
    assert_that!(types).is_equal_to(vec![json!("line"), json!("line"), json!("line")]);
}

#[test]
fn table_to_chart_with_incompatible_saved_chart_falls_back_to_bars() {
    // Le camembert lit la colonne « Gone », qui n'existe plus dans le tableau.
    let t = parse_table(&json!({ "dataset": { "source": [["M", "A", "B", "C"], ["x", 1, 2, 3]] },
        "chart": { "series": [{ "type": "pie", "encode": { "itemName": "M", "value": "Gone" } }] } }))
    .unwrap();
    let c = table_to_chart(&t, ChartLabels::english()).unwrap();
    let v = serde_json::to_value(&c).unwrap();
    assert_that!(v["series"].as_array().map(Vec::len)).is_equal_to(Some(3));
    assert_that!(v["series"][0]["type"].clone()).is_equal_to(json!("bar"));
}

#[test]
fn table_to_chart_without_saved_chart_gives_bars() {
    let t = parse_table(&json!({ "dataset": { "source": [["M", "A"], ["x", 1]] } })).unwrap();
    let c = table_to_chart(&t, ChartLabels::english()).unwrap();
    assert_that!(serde_json::to_value(&c).unwrap()["series"][0]["type"].clone()).is_equal_to(json!("bar"));
}

#[test]
fn chart_to_table_keeps_ids_and_columns() {
    let c = chart(json!({ "dataset": { "source": [["M", "A"], ["x", 1], ["y", 2]],
        "ids": ["h", "zz", "qq"], "columns": ["c9", "c1"] },
        "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] }));
    let t = chart_to_table(&c).unwrap();
    assert_that!(Some(t.dataset)).is_equal_to(c.dataset);
}

#[test]
fn table_to_chart_keeps_ids_and_columns() {
    let t = parse_table(&json!({ "dataset": { "source": [["M", "A"], ["x", 1], ["y", 2]],
        "ids": ["h", "zz", "qq"], "columns": ["c9", "c1"] } }))
    .unwrap();
    let c = table_to_chart(&t, ChartLabels::english()).unwrap();
    assert_that!(c.dataset).is_equal_to(Some(t.dataset.clone()));
    let restored = table_to_chart(&chart_to_table(&c).unwrap(), ChartLabels::english()).unwrap();
    assert_that!(restored.dataset).is_equal_to(Some(t.dataset));
}
