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

fn all_ops() -> Vec<DataOp> {
    vec![
        DataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) },
        DataOp::SetCell { row: 0, col: 0, value: Cell::Null },
        DataOp::InsertRow { at: 1, cells: vec![Cell::Null; 4], id: None },
        DataOp::InsertRow { at: 9, cells: vec![], id: None },
        DataOp::RemoveRow { at: 1 },
        DataOp::RemoveRow { at: 7 },
        DataOp::InsertColumn { at: 0, cells: vec![Cell::Null; 4], id: None },
        DataOp::InsertColumn { at: 9, cells: vec![], id: None },
        DataOp::RemoveColumn { at: 0 },
        DataOp::RemoveColumn { at: 9 },
        DataOp::Replace { source: vec![] },
        DataOp::Replace { source: vec![vec![]] },
        DataOp::Replace { source: vec![vec![Cell::Null; 2], vec![Cell::Null; 5]] },
    ]
}

#[test]
fn apply_with_ragged_or_empty_source_does_not_panic() {
    let t = |s: &str| Cell::Text(s.into());
    let sources: Vec<Vec<Vec<Cell>>> = vec![
        vec![vec![t("a")], vec![t("x"), Cell::Number(1.0), Cell::Number(2.0)], vec![t("y"), Cell::Number(3.0)]],
        vec![vec![t("a"), t("b"), t("c"), t("d")], vec![t("x")], vec![]],
        vec![],
        vec![vec![]],
    ];
    for source in sources {
        for op in all_ops() {
            for (ids, columns) in
                [(vec![], vec![]), (strings(&["h"]), strings(&[])), (strings(&["a"; 5]), strings(&["x", "y", "z"]))]
            {
                let mut d = Dataset { source: source.clone(), ids, columns };
                let _ = op.apply(&mut d);
                asserting(&format!("{op:?} sur {source:?} laisse des ids mal alignés")).that(&aligned(&d)).is_true();
            }
        }
    }
}

#[test]
fn validate_ids_wrong_length_and_invalid_id_reports_the_length_first() {
    let mut d = ds();
    d.ids = strings(&["h", "a b"]);
    let e = validate_ids(&d).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(Message::new("chart-data-ids-length").localize(&fr));
    assert_that!(e.localize(&fr)).is_not_equal_to(Message::new("chart-data-ids-invalid").localize(&fr));
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

fn colored_lines() -> ChartOption {
    serde_json::from_value(serde_json::json!({
        "dataset": { "source": [["M", "A", "B", "C"], ["x", 1, 2, 3]] },
        "xAxis": { "type": "category" },
        "series": [
            { "type": "line", "itemStyle": { "color": "#aa0000" } },
            { "type": "bar", "itemStyle": { "color": "#00bb00" } },
            { "type": "line", "itemStyle": { "color": "#0000cc" } }
        ]
    }))
    .unwrap_or_else(|e| panic!("{e}"))
}

fn colors(c: &ChartOption) -> Vec<Option<String>> {
    c.series.iter().map(|s| s.item_style.as_ref().and_then(|i| i.color.clone())).collect()
}

#[test]
fn chart_data_remove_middle_column_drops_its_own_series() {
    let c = chart_data(&colored_lines(), &DataOp::RemoveColumn { at: 2 }).unwrap();
    assert_that!(colors(&c)).is_equal_to(vec![Some("#aa0000".to_owned()), Some("#0000cc".to_owned())]);
    assert_that!(c.series.iter().map(|s| s.kind).collect::<Vec<_>>())
        .is_equal_to(vec![crate::SeriesType::Line, crate::SeriesType::Line]);
}

#[test]
fn chart_data_insert_column_adds_an_uncolored_series_at_its_place() {
    let change = DataOp::InsertColumn { at: 2, cells: vec![Cell::Text("N".into())], id: None };
    let c = chart_data(&colored_lines(), &change).unwrap();
    assert_that!(colors(&c)).is_equal_to(vec![
        Some("#aa0000".to_owned()),
        None,
        Some("#00bb00".to_owned()),
        Some("#0000cc".to_owned()),
    ]);
    // Le voisin d'avant est une ligne.
    assert_that!(c.series[1].kind).is_equal_to(crate::SeriesType::Line);
}

#[test]
fn chart_data_insert_column_first_takes_the_type_of_the_series_after() {
    let mut option = colored_lines();
    option.series[0].kind = crate::SeriesType::Bar;
    let change = DataOp::InsertColumn { at: 1, cells: vec![], id: None };
    let c = chart_data(&option, &change).unwrap();
    assert_that!((c.series.len(), c.series[0].kind, colors(&c)[0].clone())).is_equal_to((
        4,
        crate::SeriesType::Bar,
        None,
    ));
}

#[test]
fn chart_data_remove_category_column_keeps_positional_behaviour() {
    let c = chart_data(&colored_lines(), &DataOp::RemoveColumn { at: 0 }).unwrap();
    assert_that!(colors(&c)).is_equal_to(vec![Some("#aa0000".to_owned()), Some("#00bb00".to_owned())]);
}

#[test]
fn chart_data_on_a_pie_keeps_its_single_series() {
    let pie: ChartOption = serde_json::from_value(serde_json::json!({
        "dataset": { "source": [["N", "V", "W"], ["a", 1, 2]] },
        "series": [{ "type": "pie", "itemStyle": { "color": "#aa0000" } }]
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    let c = chart_data(&pie, &DataOp::RemoveColumn { at: 1 }).unwrap();
    assert_that!(colors(&c)).is_equal_to(vec![Some("#aa0000".to_owned())]);
}

fn styled_lines() -> ChartOption {
    serde_json::from_value(serde_json::json!({
        "dataset": { "source": [["M", "A", "B"], ["x", 1, 2]] },
        "xAxis": { "type": "category" },
        "series": [
            { "type": "line", "stack": "s", "smooth": true, "areaStyle": {},
              "label": { "show": true }, "itemStyle": { "color": "#aa0000" } },
            { "type": "line", "stack": "s", "itemStyle": { "color": "#00bb00" } }
        ]
    }))
    .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn chart_data_insert_column_copies_the_neighbour_style_flags_but_not_its_color() {
    let change = DataOp::InsertColumn { at: 2, cells: vec![], id: None };
    let c = chart_data(&styled_lines(), &change).unwrap();
    let s = &c.series[1];
    assert_that!((s.stack.as_deref(), s.smooth, s.area_style.is_some())).is_equal_to((Some("s"), Some(true), true));
    assert_that!((s.item_style.is_none(), s.label.is_none(), s.name.is_none())).is_equal_to((true, true, true));
}

#[test]
fn chart_data_insert_column_first_copies_the_next_series_stack() {
    let change = DataOp::InsertColumn { at: 1, cells: vec![], id: None };
    let c = chart_data(&styled_lines(), &change).unwrap();
    assert_that!((c.series[0].stack.as_deref(), c.series[0].item_style.is_none())).is_equal_to((Some("s"), true));
}
