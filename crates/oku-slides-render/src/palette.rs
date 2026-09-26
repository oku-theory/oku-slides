//! Couleurs des jetons résolus par Typst : suivre un nom de couleur jusqu'à
//! son hex, et lire la palette des graphiques (`chart.palette`).

use serde_json::{Map, Value};

/// Couleur : hex, ou nom d'une couleur du thème (références suivies, huit
/// au plus : une boucle ne donne rien).
pub fn resolve_color(colors: &Map<String, Value>, v: &Value) -> Option<String> {
    let mut v = v.as_str()?;
    for _ in 0..8 {
        if v.starts_with('#') {
            return Some(v.to_ascii_lowercase());
        }
        v = colors.get(v)?.as_str()?;
    }
    None
}

/// Palette des graphiques des jetons, en `#rgb` ou `#rrggbb` minuscules ;
/// une entrée qui ne se résout pas en couleur est ignorée.
pub fn chart_palette(tokens: &Value) -> Vec<String> {
    let empty = Map::new();
    let colors = tokens.get("color").and_then(Value::as_object).unwrap_or(&empty);
    let Some(entries) = tokens.pointer("/chart/palette").and_then(Value::as_array) else { return Vec::new() };
    entries.iter().filter_map(|e| resolve_color(colors, e)).filter(|c| is_hex(c)).collect()
}

fn is_hex(c: &str) -> bool {
    c.strip_prefix('#').is_some_and(|h| matches!(h.len(), 3 | 6) && h.chars().all(|ch| ch.is_ascii_hexdigit()))
}

#[cfg(test)]
#[path = "palette_tests.rs"]
mod palette_tests;
