//! Regression (t-955): an explicit hi-fi table (Arial 16 pt, 30 pt rows, 100/200 pt
//! columns) was drawn smaller. The widget sized columns and rows on their content with
//! the theme's padding, then shrank the whole table to fit its frame. The import now
//! gives the widget the source widths and heights as weights, and the cell margins as
//! the table padding.

use oku_slides_core::Namespace;
use oku_slides_import::{HifiTarget, PackLimits, hifi, read_pptx};
use rstest::rstest;
use serde_json::Value;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common;

/// The first object of `v` that has the key `key`.
fn with_key<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) if o.contains_key(key) => Some(v),
        Value::Object(o) => o.values().find_map(|x| with_key(x, key)),
        Value::Array(a) => a.iter().find_map(|x| with_key(x, key)),
        _ => None,
    }
}

/// Both decks hold a 2-column table of 100 and 200 pt, rows of 30 pt, PowerPoint's default margins.
#[rstest]
#[case::kitchen(common::pptx("kitchen"))]
#[case::libreoffice(common::file("libreoffice.pptx"))]
fn hifi_table_carries_its_column_widths_row_heights_and_margins(#[case] bytes: Vec<u8>) {
    let source = read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let (base, fonts) = (common::registry(), common::app_fonts());
    let target = HifiTarget {
        registry: &base,
        base: common::base_pin(),
        essentiel: common::essentiel_pin(),
        fonts: &fonts,
        pack_id: Uuid::from_u128(1),
        theme: Namespace::parse("neutre").unwrap_or_else(|| panic!("espace de noms")),
        name: "Import".to_owned(),
        created: "2026-10-11T08:00:00Z".to_owned(),
        limits: PackLimits::default(),
    };
    let plan = hifi::project(&source, &target).unwrap_or_else(|e| panic!("projection : {e}"));
    let ops = serde_json::to_value(&plan.ops).unwrap_or_else(|e| panic!("{e}"));

    let table = with_key(&ops, "column-widths").unwrap_or_else(|| panic!("no table geometry in the plan"));
    let widths: Vec<f64> = serde_json::from_value(table["column-widths"].clone()).unwrap_or_else(|e| panic!("{e}"));
    let heights: Vec<f64> = serde_json::from_value(table["row-heights"].clone()).unwrap_or_else(|e| panic!("{e}"));
    // In source pt; LibreOffice writes its EMU rounded, within 0.1 pt.
    assert_that!(widths).has_length(2);
    assert_that!(widths[0]).is_close_to(100.0, 0.1);
    assert_that!(widths[1]).is_close_to(200.0, 0.1);
    assert_that!(heights).has_length(3);
    assert_that!(heights.iter().all(|h| (h - 30.0).abs() < 0.1)).is_true();

    // PowerPoint's default margins, 7.2 pt left and right and 3.6 pt top and bottom, in page pt
    // (the page is drawn at twice the source slide's size).
    let tokens = with_key(&ops, "padding-x").unwrap_or_else(|| panic!("no table padding token"));
    let (x, y) = (tokens["padding-x"].as_f64(), tokens["padding-y"].as_f64());
    let (x, y) = x.zip(y).unwrap_or_else(|| panic!("padding tokens: {tokens}"));
    assert_that!(x).is_close_to(14.4, 0.05);
    assert_that!(y).is_close_to(7.2, 0.05);
}
