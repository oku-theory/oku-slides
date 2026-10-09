use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::*;

fn ds() -> Dataset {
    serde_json::from_value(serde_json::json!({
        "source": [["Mois", "Ventes"], ["Jan", 10], ["Fév", 12]],
        "ids": ["h", "a", "b"],
        "columns": ["m", "v"]
    }))
    .unwrap_or_else(|e| panic!("{e}"))
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

fn reason(e: &ChartError) -> String {
    e.localize(&Localizer::new(Some("en")))
}

fn is_fresh(id: &str) -> bool {
    id.len() == 8 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn aligned(d: &Dataset) -> bool {
    d.ids.len() == d.source.len() && d.columns.len() == d.source.first().map_or(0, Vec::len)
}

#[test]
fn insert_row_without_id_gets_a_fresh_hex_id() {
    let mut d = ds();
    DataOp::InsertRow { at: 3, cells: vec![], id: None }.apply(&mut d).unwrap();
    assert_that!(d.ids.len()).is_equal_to(4);
    assert_that!(is_fresh(&d.ids[3])).is_true();
    assert_that!(d.ids[..3].to_vec()).is_equal_to(strings(&["h", "a", "b"]));
    for _ in 0..50 {
        DataOp::InsertRow { at: 1, cells: vec![], id: None }.apply(&mut d).unwrap();
    }
    let unique: std::collections::HashSet<_> = d.ids.iter().collect();
    assert_that!(unique.len()).is_equal_to(54);
}

#[test]
fn insert_row_with_id_keeps_it() {
    let mut d = ds();
    DataOp::InsertRow { at: 1, cells: vec![], id: Some("neuf".into()) }.apply(&mut d).unwrap();
    assert_that!(d.ids).is_equal_to(strings(&["h", "neuf", "a", "b"]));
}

#[test]
fn insert_row_with_taken_id_is_refused() {
    let mut d = ds();
    let e = DataOp::InsertRow { at: 1, cells: vec![], id: Some("a".into()) }.apply(&mut d).unwrap_err();
    assert_that!(reason(&e)).contains("already used");
    assert_that!(d).is_equal_to(ds());
}

#[test]
fn insert_row_with_invalid_id_is_refused() {
    let mut d = ds();
    assert_that!(DataOp::InsertRow { at: 1, cells: vec![], id: Some("a b".into()) }.apply(&mut d)).is_err();
    assert_that!(d).is_equal_to(ds());
}

#[test]
fn insert_column_without_id_gets_a_fresh_id() {
    let mut d = ds();
    DataOp::InsertColumn { at: 1, cells: vec![], id: None }.apply(&mut d).unwrap();
    assert_that!(d.columns.len()).is_equal_to(3);
    assert_that!(is_fresh(&d.columns[1])).is_true();
    assert_that!((d.columns[0].as_str(), d.columns[2].as_str())).is_equal_to(("m", "v"));
}

#[test]
fn insert_column_with_id_keeps_it() {
    let mut d = ds();
    DataOp::InsertColumn { at: 2, cells: vec![], id: Some("marge".into()) }.apply(&mut d).unwrap();
    assert_that!(d.columns).is_equal_to(strings(&["m", "v", "marge"]));
}

#[test]
fn insert_column_with_taken_id_is_refused() {
    let mut d = ds();
    let e = DataOp::InsertColumn { at: 2, cells: vec![], id: Some("m".into()) }.apply(&mut d).unwrap_err();
    assert_that!(reason(&e)).contains("already used");
}

#[test]
fn remove_row_drops_its_id() {
    let mut d = ds();
    DataOp::RemoveRow { at: 1 }.apply(&mut d).unwrap();
    assert_that!(d.ids).is_equal_to(strings(&["h", "b"]));
}

#[test]
fn remove_column_drops_its_id() {
    let mut d = ds();
    DataOp::RemoveColumn { at: 0 }.apply(&mut d).unwrap();
    assert_that!(d.columns).is_equal_to(strings(&["v"]));
    assert_that!(d.source[1].clone()).is_equal_to(vec![Cell::Number(10.0)]);
}

#[test]
fn set_cell_insert_column_remove_column_leave_row_ids_alone() {
    let mut d = ds();
    DataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) }.apply(&mut d).unwrap();
    assert_that!(d.source[1][1].clone()).is_equal_to(Cell::Number(99.0));
    DataOp::InsertColumn { at: 2, cells: vec![Cell::Text("Marge".into())], id: None }.apply(&mut d).unwrap();
    DataOp::RemoveColumn { at: 2 }.apply(&mut d).unwrap();
    assert_that!(d.ids).is_equal_to(strings(&["h", "a", "b"]));
    assert_that!(d.columns).is_equal_to(strings(&["m", "v"]));
}

#[test]
fn replace_keeps_ids_by_position_and_mints_extra() {
    let mut d = ds();
    let source = vec![vec![Cell::Null; 3]; 4];
    DataOp::Replace { source: source.clone() }.apply(&mut d).unwrap();
    assert_that!(d.source).is_equal_to(source);
    assert_that!(d.ids[..3].to_vec()).is_equal_to(strings(&["h", "a", "b"]));
    assert_that!(is_fresh(&d.ids[3])).is_true();
    assert_that!(d.columns[..2].to_vec()).is_equal_to(strings(&["m", "v"]));
    assert_that!(is_fresh(&d.columns[2])).is_true();
}

#[test]
fn replace_with_fewer_rows_drops_extra_ids() {
    let mut d = ds();
    DataOp::Replace { source: vec![vec![Cell::Text("x".into())]] }.apply(&mut d).unwrap();
    assert_that!((d.ids, d.columns)).is_equal_to((strings(&["h"]), strings(&["m"])));
}

#[test]
fn apply_with_misaligned_ids_does_not_panic() {
    let ops = vec![
        DataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) },
        DataOp::InsertRow { at: 3, cells: vec![], id: None },
        DataOp::InsertRow { at: 9, cells: vec![], id: None },
        DataOp::RemoveRow { at: 2 },
        DataOp::RemoveRow { at: 7 },
        DataOp::InsertColumn { at: 1, cells: vec![], id: None },
        DataOp::InsertColumn { at: 9, cells: vec![], id: None },
        DataOp::RemoveColumn { at: 1 },
        DataOp::RemoveColumn { at: 9 },
        DataOp::Replace { source: vec![vec![Cell::Null; 2]; 5] },
    ];
    for op in ops {
        for (ids, columns) in [(strings(&["h"]), strings(&[])), (strings(&["a"; 5]), strings(&["x", "y", "z"]))] {
            let mut d = ds();
            d.ids = ids;
            d.columns = columns;
            let _ = op.apply(&mut d);
            asserting(&format!("{op:?} laisse des ids mal alignés")).that(&aligned(&d)).is_true();
        }
    }
}

#[test]
fn ensure_ids_is_idempotent_and_keeps_valid_ids() {
    let mut d = ds();
    d.ids.pop();
    ensure_ids(&mut d);
    assert_that!(d.ids[..2].to_vec()).is_equal_to(strings(&["h", "a"]));
    assert_that!(is_fresh(&d.ids[2])).is_true();
    let once = d.clone();
    ensure_ids(&mut d);
    assert_that!(d).is_equal_to(once);
}

#[test]
fn ensure_ids_replaces_duplicate_or_invalid_ids() {
    let mut d = ds();
    d.ids = strings(&["h", "h", "a b"]);
    d.columns = strings(&["", "v", "extra"]);
    ensure_ids(&mut d);
    assert_that!(d.ids[0].as_str()).is_equal_to("h");
    assert_that!(is_fresh(&d.ids[1]) && is_fresh(&d.ids[2]) && d.ids[1] != d.ids[2]).is_true();
    assert_that!(is_fresh(&d.columns[0])).is_true();
    assert_that!(d.columns[1].as_str()).is_equal_to("v");
    assert_that!(d.columns.len()).is_equal_to(2);
}

#[test]
fn validate_ids_empty_lists_are_accepted() {
    let d = Dataset { source: ds().source, ..Dataset::default() };
    assert_that!(validate_ids(&d)).is_ok();
}

#[test]
fn apply_row_op_on_header_row_is_rejected() {
    assert_that!(DataOp::RemoveRow { at: 0 }.apply(&mut ds())).is_err();
    assert_that!(DataOp::InsertRow { at: 0, cells: vec![], id: None }.apply(&mut ds())).is_err();
}

#[test]
fn data_op_tagged_json_deserializes() {
    let op: DataOp = serde_json::from_str(r#"{"op":"set_cell","row":2,"col":1,"value":120}"#).unwrap();
    assert_that!(op).is_equal_to(DataOp::SetCell { row: 2, col: 1, value: Cell::Number(120.0) });
}

#[test]
fn data_op_insert_row_without_id_serializes_without_it() {
    let op: DataOp = serde_json::from_str(r#"{"op":"insert_row","at":1,"cells":[]}"#).unwrap();
    assert_that!(serde_json::to_value(&op).unwrap())
        .is_equal_to(serde_json::json!({"op":"insert_row","at":1,"cells":[]}));
}
