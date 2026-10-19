use oku_slides_chart::parse_table;
use oku_slides_core::kinds::base;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::report::RunEffect;
use crate::source::{Borders, Fill, TableFlags};
use crate::test_support::{apply_blocks, element_with, para, run, run_with, text_box};

fn cell(text: &str) -> source::Cell {
    source::Cell {
        text: text_box(vec![para(vec![run(text)])]),
        fill: Fill::None,
        borders: Borders::default(),
        row_span: 1,
        col_span: 1,
        covered: false,
    }
}

fn covered() -> source::Cell {
    source::Cell { text: text_box(Vec::new()), covered: true, ..cell("") }
}

fn source_table(rows: Vec<Vec<source::Cell>>) -> source::Table {
    let flags = TableFlags { first_row: true, band_row: true, ..TableFlags::default() };
    source::Table { col_widths: Vec::new(), row_heights: Vec::new(), flags, rows }
}

fn grid(rows: usize, cols: usize) -> source::Table {
    source_table((0..rows).map(|r| (0..cols).map(|c| cell(&format!("{r}.{c}"))).collect()).collect())
}

fn data(t: &source::Table) -> (Value, Vec<LossKind>) {
    table(t).unwrap()
}

#[test]
fn table_writes_row_zero_as_the_header_and_cells_as_text() {
    let t = source_table(vec![vec![cell("Pays"), cell("Ventes")], vec![cell("FR"), cell("10")]]);
    let (option, losses) = data(&t);
    assert_that!(option).is_equal_to(json!({
        "dataset": { "source": [["Pays", "Ventes"], ["FR", "10"]] },
        "header-row": true, "header-column": false, "striped": true,
    }));
    assert_that!(losses).is_empty();
}

#[test]
fn table_option_is_accepted_by_the_table_host() {
    let (option, _) = data(&grid(3, 4));
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_flattens_merged_cells_into_their_anchor() {
    let mut anchor = cell("Fusion");
    anchor.col_span = 2;
    let t = source_table(vec![vec![anchor, covered()], vec![cell("a"), cell("b")]]);
    let (option, losses) = data(&t);
    assert_that!(option["dataset"]["source"]).is_equal_to(json!([["Fusion", null], ["a", "b"]]));
    assert_that!(losses).is_equal_to(vec![LossKind::MergedCells]);
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_pads_a_short_row_with_empty_cells() {
    let t = source_table(vec![vec![cell("a"), cell("b")], vec![cell("c")]]);
    assert_that!(data(&t).0["dataset"]["source"]).is_equal_to(json!([["a", "b"], ["c", null]]));
}

#[test]
fn table_joins_the_paragraphs_of_a_cell() {
    let mut c = cell("");
    c.text = text_box(vec![para(vec![run("Ligne 1")]), para(vec![run("Ligne 2")])]);
    assert_that!(data(&source_table(vec![vec![c]])).0["dataset"]["source"]).is_equal_to(json!([["Ligne 1 Ligne 2"]]));
}

#[test]
fn table_reports_the_effects_lost_in_cells_once() {
    let mut c = cell("");
    c.text = text_box(vec![para(vec![run_with("souligné", |s| s.underline = true)])]);
    let t = source_table(vec![vec![c.clone(), c]]);
    assert_that!(data(&t).1).is_equal_to(vec![LossKind::RunStyle(RunEffect::Underline)]);
}

/// 120 lignes × 25 colonnes : il en reste `MAX_TABLE_ROWS` = 100 et `MAX_TABLE_COLUMNS` = 20.
#[test]
fn table_cuts_to_the_core_row_and_column_bounds() {
    let (option, losses) = data(&grid(120, 25));
    let source = option["dataset"]["source"].as_array().unwrap();
    assert_that!(source.len()).is_equal_to(MAX_TABLE_ROWS);
    assert_that!(source.iter().all(|r| r.as_array().unwrap().len() == MAX_TABLE_COLUMNS)).is_true();
    assert_that!(losses).is_equal_to(vec![LossKind::TableTruncated]);
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_cuts_a_long_cell_to_the_core_character_bound() {
    let t = source_table(vec![vec![cell(&"é".repeat(1500))]]);
    let (option, losses) = data(&t);
    assert_that!(option["dataset"]["source"][0][0].as_str().unwrap().chars().count()).is_equal_to(MAX_TABLE_CELL_CHARS);
    assert_that!(losses).is_equal_to(vec![LossKind::TableTruncated]);
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_without_cells_is_none() {
    assert_that!(table(&source_table(Vec::new()))).is_none();
    assert_that!(table(&source_table(vec![Vec::new()]))).is_none();
}

#[test]
fn table_element_is_accepted_by_add_slide() {
    let (option, _) = data(&grid(4, 3));
    let deck = apply_blocks(vec![element_with("t", base::TABLE, "data", option)]);
    assert_that!(deck).is_ok();
}
