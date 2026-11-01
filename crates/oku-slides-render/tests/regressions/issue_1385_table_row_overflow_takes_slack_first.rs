//! A weighted row whose text was taller than its share shrank the whole table, even when the
//! other rows had room to spare. The overflow now comes from the rows with slack first; the
//! table shrinks only once that slack is used up.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use oku_slides_render::Compiled;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// The long wrapping text of the first row.
fn long() -> String {
    "word ".repeat(30)
}

/// A table of `rows` (one cell each) in a 300 pt wide frame of height `h`, rules painted as
/// the page; `weights` as its `row-heights`.
fn compile(h: usize, rows: &[String], weights: Option<Value>) -> Compiled {
    let source: Vec<Value> = rows.iter().map(|r| json!([r])).collect();
    let mut data = json!({ "header-row": false, "dataset": { "source": source } });
    if let Some(w) = weights {
        data["row-heights"] = w;
    }
    let deck = json!({
        "tokens": { "component": { "base": { "table": { "rule-color": "bg" } } } },
        "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [
            { "id": "t", "kind": base::TABLE, "params": { "data": data },
                "frame": { "x": 10, "y": 10, "w": 300, "h": h } }] } }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"));
    crate::common::renderer().render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Extent of the table's ink along x (`along_x`) or y, in px.
fn ink(c: &Compiled, along_x: bool) -> usize {
    let runs = crate::common::ink_runs(c, "t", along_x);
    let (Some(first), Some(last)) = (runs.first(), runs.last()) else { panic!("no ink") };
    last.1 - first.0
}

/// Width of the three-row table's ink in a frame of height `h`: it narrows when the table shrinks.
fn ink_width(h: usize, weights: Option<Value>) -> usize {
    ink(&compile(h, &[long(), "x".to_owned(), "y".to_owned()], weights), true)
}

/// Frame height where the natural rows fit, but the long row exceeds a third of it.
const ROOMY: usize = 300;
/// Frame height below the natural rows: no slack can cover the long row.
const TIGHT: usize = 120;

#[test]
fn issue_1385_long_row_alone_is_taller_than_its_share() {
    // The precondition of the next test: with equal weights, the long row overflows its third.
    let text = ink(&compile(ROOMY, &[long()], None), false);
    assert_that!(text).is_greater_than(ROOMY / 3);
}

#[test]
fn issue_1385_tall_row_beside_rows_with_slack_keeps_its_text_size() {
    let natural = ink_width(ROOMY, None);
    assert_that!(ink_width(ROOMY, Some(json!([1, 1, 1])))).is_equal_to(natural);
}

#[test]
fn issue_1385_rows_beyond_the_slack_shrink_like_unweighted_rows() {
    // The weights ask most of the height for the short rows; once the slack is gone every row
    // keeps its content height, so the table shrinks exactly as without weights.
    assert_that!(ink_width(TIGHT, Some(json!([1, 4, 4])))).is_equal_to(ink_width(TIGHT, None));
}

#[test]
fn issue_1385_rows_beyond_the_slack_do_shrink() {
    // The precondition of the previous test: at this height the table is scaled down.
    assert_that!(ink_width(TIGHT, None)).is_less_than(ink_width(ROOMY, None));
}
