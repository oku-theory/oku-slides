//! Les graphiques prenaient leur police dans une table indexée par le nom du
//! thème : un thème personnalisé retombait sur la police par défaut. Elle suit
//! désormais `font.body` des jetons du thème (t-554).

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::base;
use oku_slides_render::{Fonts, Renderer};
use serde_json::json;
use speculoos::prelude::*;

fn deck_with_one_chart() -> Deck {
    let option = json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 3], ["Fév", 5]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    });
    serde_json::from_value(json!({ "slides": [
        { "id": "libre", "kind": "blank", "params": {
          "elements": [{ "id": "g", "kind": base::CHART, "params": { "option": option },
                         "frame": { "x": 100, "y": 100, "w": 900, "h": 500 } }] } }
    ] }))
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Polices livrées utilisées par les thèmes intégrés.
fn theme_fonts() -> Fonts {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts/");
    Fonts::new(
        ["inter-400.ttf", "instrument-sans-latin-standard-normal.ttf"]
            .map(|f| std::fs::read(format!("{dir}{f}")).unwrap_or_else(|e| panic!("{f} illisible : {e}"))),
    )
}

/// La page SVG (graphique embarqué) d'un rendu avec les jetons `tokens`.
fn chart_svgs(tokens: Option<serde_json::Value>) -> String {
    let mut r = Renderer::with_builtins(theme_fonts()).unwrap_or_else(|e| panic!("{e}"));
    r.set_theme(tokens.and_then(|t| t.as_object().cloned())).unwrap_or_else(|e| panic!("{e}"));
    let compiled = r.render(&deck_with_one_chart(), &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    compiled.svg(0).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn chart_font_follows_theme_font_body() {
    // Le SVG du graphique est embarqué dans la page : sa police le fait changer.
    let inter = chart_svgs(Some(json!({ "font": { "body": ["Inter"] } })));
    let other = chart_svgs(Some(json!({ "font": { "body": ["Instrument Sans"] } })));
    assert_that!(inter == other).is_false();
}

#[test]
fn chart_font_without_font_body_differs_from_a_themed_font() {
    let themed = chart_svgs(Some(json!({ "font": { "body": ["Inter"] } })));
    assert_that!(chart_svgs(None) == themed).is_false();
}
