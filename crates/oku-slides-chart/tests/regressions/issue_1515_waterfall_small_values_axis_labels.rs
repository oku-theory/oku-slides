//! A waterfall whose running sums stayed under 0.5 repeated its axis labels ("0.2" twice).
//!
//! It kept charts-rs's scale with bounds and no split count, so ticks fell on sixths of
//! 0.5 that one-decimal labels cannot tell apart; the automatic scale now ticks by 0.1.

use oku_slides_chart::{ChartRenderer, ChartsRsRenderer, Size, parse_option};
use serde_json::json;
use speculoos::prelude::*;

/// Texts of the SVG that read as numbers, in drawing order.
fn numeric_texts(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .filter_map(|t| t.split_once('>').and_then(|(_, rest)| rest.split_once("</text>")).map(|(text, _)| text.trim()))
        .filter(|text| text.parse::<f64>().is_ok())
        .map(str::to_owned)
        .collect()
}

#[test]
fn waterfall_under_half_labels_each_tick_once() {
    let option = json!({
        "dataset": { "source": [["Poste", "Montant"], ["Début", 0.2], ["Hausse", 0.1], ["Baisse", -0.05]] },
        "series": [{ "type": "waterfall", "label": { "show": false } }]
    });
    let option = parse_option(&option).unwrap_or_else(|e| panic!("{e}"));
    let svg = ChartsRsRenderer::default()
        .render(&option, Size { width: 640.0, height: 400.0 })
        .unwrap_or_else(|e| panic!("{e}"));
    let labels = numeric_texts(&svg);
    let mut unique = labels.clone();
    unique.dedup();
    assert_that!(labels.len()).is_greater_than(3);
    assert_that!(labels).is_equal_to(&unique);
}
