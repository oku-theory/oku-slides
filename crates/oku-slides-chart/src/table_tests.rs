use oku_slides_i18n::{Localize, Localizer};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Cell, ChartErrorKind, DataOp};

fn table() -> TableOption {
    parse_table(&json!({
        "dataset": { "source": [["Pays", "Ventes", "Part"], ["FR", 10, 0.4], ["DE", 15, 0.6]] },
        "align": ["start", "auto", "end"]
    }))
    .unwrap_or_else(|e| panic!("{e}"))
}

fn kind_of(v: serde_json::Value) -> ChartErrorKind {
    parse_table(&v).map(|_| ()).unwrap_err().kind()
}

#[test]
fn parse_table_without_options_uses_defaults() {
    let t = parse_table(&json!({ "dataset": { "source": [["A"]] } })).unwrap();
    assert_that!((t.header_row, t.header_column, t.striped, t.align.len())).is_equal_to((true, false, false, 0));
}

#[test]
fn parse_table_with_kebab_case_options_reads_them() {
    let t = parse_table(&json!({ "dataset": { "source": [["A"]] }, "header-row": false,
        "header-column": true, "striped": true, "align": ["center"] }))
    .unwrap();
    assert_that!((t.header_row, t.header_column, t.striped, t.align)).is_equal_to((
        false,
        true,
        true,
        vec![ColumnAlign::Center],
    ));
}

#[test]
fn parse_table_unknown_field_is_invalid_option() {
    assert_that!(kind_of(json!({ "dataset": { "source": [["A"]] }, "series": [] })))
        .is_equal_to(ChartErrorKind::InvalidOption);
}

#[test]
fn parse_table_empty_source_is_inconsistent() {
    assert_that!(kind_of(json!({ "dataset": { "source": [] } }))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_empty_header_row_is_inconsistent() {
    assert_that!(kind_of(json!({ "dataset": { "source": [[]] } }))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_ragged_rows_are_inconsistent() {
    assert_that!(kind_of(json!({ "dataset": { "source": [["A", "B"], [1]] } })))
        .is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_at_max_columns_is_accepted() {
    let header: Vec<String> = (0..MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
    assert_that!(parse_table(&json!({ "dataset": { "source": [header] } }))).is_ok();
}

#[test]
fn parse_table_beyond_max_columns_is_inconsistent() {
    let header: Vec<String> = (0..=MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
    assert_that!(kind_of(json!({ "dataset": { "source": [header] } }))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_beyond_max_rows_is_inconsistent() {
    let rows: Vec<Vec<usize>> = (0..=MAX_TABLE_ROWS).map(|i| vec![i]).collect();
    assert_that!(kind_of(json!({ "dataset": { "source": rows } }))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_cell_text_too_long_is_inconsistent() {
    let long = "x".repeat(MAX_TABLE_CELL_CHARS + 1);
    assert_that!(kind_of(json!({ "dataset": { "source": [[long]] } }))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_align_longer_than_columns_is_truncated() {
    let t = parse_table(&json!({ "dataset": { "source": [["A"]] }, "align": ["start", "end"] }))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(t.align).is_equal_to(vec![ColumnAlign::Start]);
}

fn with_ids(ids: serde_json::Value, columns: serde_json::Value) -> serde_json::Value {
    json!({ "dataset": { "source": [["A", "B"], [1, 2]], "ids": ids, "columns": columns } })
}

#[test]
fn parse_table_ids_of_wrong_length_is_inconsistent() {
    assert_that!(kind_of(with_ids(json!(["h"]), json!(["a", "b"])))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_duplicate_ids_is_inconsistent() {
    assert_that!(kind_of(with_ids(json!(["h", "h"]), json!(["a", "b"])))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_invalid_id_is_inconsistent() {
    for bad in [json!("a b"), json!(""), json!("x".repeat(65))] {
        asserting(&format!("{bad}"))
            .that(&kind_of(with_ids(json!(["h", bad]), json!(["a", "b"]))))
            .is_equal_to(ChartErrorKind::Inconsistent);
    }
}

#[test]
fn parse_table_id_of_64_chars_is_accepted() {
    assert_that!(parse_table(&with_ids(json!(["h", "x".repeat(64)]), json!(["a", "b"])))).is_ok();
}

#[test]
fn parse_table_columns_of_wrong_length_is_inconsistent() {
    assert_that!(kind_of(with_ids(json!(["h", "r"]), json!(["a"])))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_duplicate_columns_is_inconsistent() {
    assert_that!(kind_of(with_ids(json!(["h", "r"]), json!(["a", "a"])))).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_invalid_column_id_is_inconsistent() {
    for bad in [json!("a b"), json!(""), json!("x".repeat(65))] {
        asserting(&format!("{bad}"))
            .that(&kind_of(with_ids(json!(["h", "r"]), json!(["a", bad]))))
            .is_equal_to(ChartErrorKind::Inconsistent);
    }
}

#[test]
fn parse_table_column_id_of_64_chars_is_accepted() {
    assert_that!(parse_table(&with_ids(json!(["h", "r"]), json!(["a", "x".repeat(64)])))).is_ok();
}

#[test]
fn parse_table_duplicate_ids_reason_is_localized_in_french() {
    let err = parse_table(&with_ids(json!(["h", "h"]), json!(["a", "b"]))).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).contains("répété");
}

#[test]
fn table_data_insert_row_keeps_ids_aligned() {
    let next = table_data(&table(), &DataOp::InsertRow { at: 1, cells: vec![], id: None }).unwrap();
    assert_that!((next.dataset.ids.len(), next.dataset.columns.len())).is_equal_to((4, 3));
}

#[test]
fn parse_table_ragged_reason_is_localized_in_french() {
    let err = parse_table(&json!({ "dataset": { "source": [["A", "B"], [1]] } })).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).contains("même nombre de cellules");
}

#[test]
fn validate_table_non_finite_number_is_inconsistent() {
    let mut t = table();
    t.dataset.source[1][1] = Cell::Number(f64::INFINITY);
    assert_that!(validate_table(&t).unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn table_data_insert_column_at_zero_shifts_align() {
    let next = table_data(&table(), &DataOp::InsertColumn { at: 0, cells: vec![], id: None }).unwrap();
    assert_that!(next.align).is_equal_to(vec![
        ColumnAlign::Auto,
        ColumnAlign::Start,
        ColumnAlign::Auto,
        ColumnAlign::End,
    ]);
}

#[test]
fn table_data_insert_column_at_align_len_leaves_align() {
    let mut t = table();
    t.align = vec![ColumnAlign::End];
    let next = table_data(&t, &DataOp::InsertColumn { at: 1, cells: vec![], id: None }).unwrap();
    assert_that!(next.align).is_equal_to(vec![ColumnAlign::End]);
}

#[test]
fn table_data_remove_column_drops_its_alignment() {
    let next = table_data(&table(), &DataOp::RemoveColumn { at: 0 }).unwrap();
    assert_that!(next.align).is_equal_to(vec![ColumnAlign::Auto, ColumnAlign::End]);
}

#[test]
fn table_data_remove_column_beyond_align_leaves_it() {
    let mut t = table();
    t.align = vec![ColumnAlign::End];
    let next = table_data(&t, &DataOp::RemoveColumn { at: 2 }).unwrap();
    assert_that!(next.align).is_equal_to(vec![ColumnAlign::End]);
}

#[test]
fn table_data_replace_with_fewer_columns_truncates_align() {
    let next = table_data(&table(), &DataOp::Replace { source: vec![vec![Cell::Text("x".into())]] }).unwrap();
    assert_that!(next.align).is_equal_to(vec![ColumnAlign::Start]);
}

#[test]
fn table_data_set_cell_changes_only_that_cell() {
    let mut expected = table();
    expected.dataset.source[1][1] = Cell::Number(99.0);
    let next = table_data(&table(), &DataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) }).unwrap();
    // Les identifiants absents sont tirés à la première op.
    expected.dataset.ids.clone_from(&next.dataset.ids);
    expected.dataset.columns.clone_from(&next.dataset.columns);
    assert_that!(next).is_equal_to(expected);
}

#[test]
fn table_data_insert_row_at_max_rows_is_inconsistent() {
    let rows: Vec<Vec<usize>> = (0..MAX_TABLE_ROWS).map(|i| vec![i]).collect();
    let t = parse_table(&json!({ "dataset": { "source": rows } })).unwrap();
    let r = table_data(&t, &DataOp::InsertRow { at: 1, cells: vec![], id: None });
    assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn table_data_insert_column_at_max_columns_is_inconsistent() {
    let header: Vec<String> = (0..MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
    let t = parse_table(&json!({ "dataset": { "source": [header] } })).unwrap();
    let r = table_data(&t, &DataOp::InsertColumn { at: 0, cells: vec![], id: None });
    assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn table_data_replace_beyond_max_rows_is_inconsistent() {
    let source = vec![vec![Cell::Null]; MAX_TABLE_ROWS + 1];
    let r = table_data(&table(), &DataOp::Replace { source });
    assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn patch_table_then_inverse_restores_original() {
    let original = table();
    let (next, inverse) = patch_table(&original, &json!({ "striped": true, "align": null })).unwrap();
    assert_that!((next.striped, next.align.len())).is_equal_to((true, 0));
    let (restored, _) = patch_table(&next, &inverse).unwrap();
    assert_that!(restored).is_equal_to(original);
}

#[test]
fn patch_table_with_too_many_align_entries_truncates_to_the_columns() {
    let t = parse_table(&json!({ "dataset": { "source": [["A", "B"], [1, 2]] } })).unwrap();
    let (next, _) = patch_table(&t, &json!({ "align": ["start", "end", "auto"] })).unwrap();
    assert_that!(next.align).is_equal_to(vec![ColumnAlign::Start, ColumnAlign::End]);
}

#[test]
fn patch_table_touching_dataset_is_refused() {
    let r = patch_table(&table(), &json!({ "dataset": null }));
    assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::PatchTouchesDataset);
}

#[test]
fn patch_table_unknown_alignment_is_invalid_option() {
    let r = patch_table(&table(), &json!({ "align": ["left"] }));
    assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::InvalidOption);
}

#[test]
fn table_option_serializes_field_names_in_kebab_case() {
    let v = serde_json::to_value(table()).unwrap();
    assert_that!(v.get("header-row")).is_equal_to(Some(&json!(true)));
}

#[test]
fn column_align_serializes_in_lowercase() {
    assert_that!(serde_json::to_value(table().align).unwrap()).is_equal_to(json!(["start", "auto", "end"]));
}

#[test]
fn parse_table_chart_not_an_object_is_inconsistent() {
    assert_that!(kind_of(json!({ "dataset": { "source": [["A"]] }, "chart": [1] })))
        .is_equal_to(ChartErrorKind::Inconsistent);
}

#[test]
fn parse_table_strips_the_dataset_of_the_saved_chart() {
    let t = parse_table(&json!({ "dataset": { "source": [["A"]] },
        "chart": { "dataset": { "source": [["B"]] }, "series": [{ "type": "bar" }] } }))
    .unwrap();
    assert_that!(t.chart).is_equal_to(Some(json!({ "series": [{ "type": "bar" }] })));
}

#[test]
fn patch_table_strips_the_dataset_of_the_saved_chart() {
    let patch = json!({ "chart": { "dataset": { "source": [["B"]] }, "color": ["#123456"] } });
    let (next, _) = patch_table(&table(), &patch).unwrap();
    assert_that!(next.chart).is_equal_to(Some(json!({ "color": ["#123456"] })));
}
