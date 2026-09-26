//! Apparence de l'interface : elle prend les couleurs, les polices et les
//! formes du thème des slides, lues dans les jetons résolus par Typst.

use std::collections::BTreeMap;

use oku_slides_render::{chart_palette, resolve_color};
use serde::Serialize;
use serde_json::Value;

/// Apparence de l'interface, tirée des jetons du thème des slides.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Appearance {
    /// `light` ou `dark` : contrôles natifs, barres de défilement.
    pub scheme: String,
    /// Couleurs des slides (fond, encre, accent…), résolues en hex.
    pub slide: BTreeMap<String, String>,
    /// Couleurs de l'interface (`desk`, `sheet`, `piece`, `ink`…). Une couleur
    /// absente est dérivée par l'interface à partir des couleurs des slides.
    pub app: BTreeMap<String, String>,
    /// Formes de l'interface : `radius-sm`, `radius-md`, `radius-lg` (px), `elevation`.
    pub metrics: BTreeMap<String, f64>,
    /// Familles de police : titres et texte courant.
    pub font_display: Vec<String>,
    pub font_body: Vec<String>,
    /// Palette des graphiques du thème (jetons `chart.palette`), en hex :
    /// l'interface la propose pour colorer une série.
    pub chart_palette: Vec<String>,
}

fn token_fonts(fonts: &serde_json::Map<String, Value>, name: &str) -> Vec<String> {
    let mut v = fonts.get(name);
    for _ in 0..8 {
        match v {
            Some(Value::String(s)) if fonts.contains_key(s) => v = fonts.get(s),
            Some(Value::String(s)) => return vec![s.clone()],
            Some(Value::Array(a)) => return a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect(),
            _ => break,
        }
    }
    Vec::new()
}

/// Apparence à partir des jetons résolus par Typst.
pub fn appearance(tokens: &Value) -> Appearance {
    let empty = serde_json::Map::new();
    let colors = tokens.get("color").and_then(Value::as_object).unwrap_or(&empty);
    let fonts = tokens.get("font").and_then(Value::as_object).unwrap_or(&empty);
    let app = tokens.get("app").and_then(Value::as_object).unwrap_or(&empty);
    let slide = colors.iter().filter_map(|(k, v)| Some((k.clone(), resolve_color(colors, v)?))).collect();
    let app_colors = app
        .iter()
        .filter(|(k, _)| *k != "scheme")
        .filter_map(|(k, v)| Some((k.clone(), resolve_color(colors, v)?)))
        .collect();
    let metrics = app.iter().filter_map(|(k, v)| Some((k.clone(), v.as_f64()?))).collect();
    Appearance {
        metrics,
        scheme: app.get("scheme").and_then(Value::as_str).unwrap_or("light").to_owned(),
        slide,
        app: app_colors,
        font_display: token_fonts(fonts, "display"),
        font_body: token_fonts(fonts, "body"),
        chart_palette: chart_palette(tokens),
    }
}
