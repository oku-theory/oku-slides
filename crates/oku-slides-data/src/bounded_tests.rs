use rstest::rstest;
use speculoos::prelude::*;

use crate::limits::{MAX_COLUMNS, MAX_OVERLAY, MAX_ROWS};
use crate::{Data, NewTable, Table};

/// A list being bounded.
#[derive(Debug, Clone, Copy)]
enum List {
    RowIds,
    Columns,
    Overlay,
    RowCells,
}

/// `n` comma-separated items of `list`.
fn items(list: List, n: usize) -> String {
    let item = |i: usize| match list {
        List::RowIds => format!("\"r{i}\""),
        List::Columns => format!("{{\"id\":\"c{i}\",\"name\":\"C\",\"type\":\"number\"}}"),
        List::Overlay => format!("\"r{i}/c\":1"),
        List::RowCells => "1".to_owned(),
    };
    (0..n).map(item).collect::<Vec<_>>().join(",")
}

/// JSON of a table holding `n` items of `list`; when `cut`, the JSON breaks right after them.
fn table_json(list: List, n: usize, cut: bool) -> String {
    let items = items(list, n);
    let (head, tail) = match list {
        List::RowIds => (r#"{"generation":"g","columns":[],"rows":["#.to_owned(), "]}"),
        List::Columns => (r#"{"generation":"g","rows":[],"columns":["#.to_owned(), "]}"),
        List::Overlay => (r#"{"generation":"g","columns":[],"rows":[],"cells":{"#.to_owned(), "}}"),
        List::RowCells => (r#"{"columns":[],"rows":[{"cells":["#.to_owned(), "]}]}"),
    };
    if cut { format!("{head}{items},!!!") } else { format!("{head}{items}{tail}") }
}

/// Parses the JSON of `list` as its own type (a `NewTable` for row cells, else a `Table`).
fn parse(list: List, json: &str) -> Result<(), serde_json::Error> {
    match list {
        List::RowCells => serde_json::from_str::<NewTable>(json).map(drop),
        _ => serde_json::from_str::<Table>(json).map(drop),
    }
}

#[rstest]
#[case::row_ids(List::RowIds, MAX_ROWS, "items")]
#[case::columns(List::Columns, MAX_COLUMNS, "items")]
#[case::overlay(List::Overlay, MAX_OVERLAY, "entries")]
#[case::row_cells(List::RowCells, MAX_COLUMNS, "items")]
fn table_json_over_bound_fails_while_parsing(#[case] list: List, #[case] max: usize, #[case] unit: &str) {
    let error = parse(list, &table_json(list, max + 1, true)).map_err(|e| e.to_string());
    assert_that!(error).is_err().contains(format!("at most {max} {unit}").as_str());
}

#[rstest]
#[case::row_ids(List::RowIds, MAX_ROWS)]
#[case::columns(List::Columns, MAX_COLUMNS)]
#[case::overlay(List::Overlay, MAX_OVERLAY)]
#[case::row_cells(List::RowCells, MAX_COLUMNS)]
fn table_json_at_bound_parses(#[case] list: List, #[case] max: usize) {
    assert_that!(parse(list, &table_json(list, max, false))).is_ok();
}

#[rstest]
#[case::row_ids(List::RowIds, MAX_ROWS, "items")]
#[case::columns(List::Columns, MAX_COLUMNS, "items")]
#[case::overlay(List::Overlay, MAX_OVERLAY, "entries")]
fn data_json_over_bound_fails_while_parsing(#[case] list: List, #[case] max: usize, #[case] unit: &str) {
    let json = format!(r#"{{"name":"d","kind":"manual","table":{}"#, table_json(list, max + 1, true));
    let error = serde_json::from_str::<Data>(&json).map(drop).map_err(|e| e.to_string());
    assert_that!(error).is_err().contains(format!("at most {max} {unit}").as_str());
}
