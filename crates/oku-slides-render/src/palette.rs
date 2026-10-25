//! Couleurs des jetons résolus par Typst : suivre un nom de couleur jusqu'à
//! son hex, et lire la palette des graphiques (`chart.palette`).

use oku_slides_core::is_hex_color;
use serde_json::{Map, Value};

/// Couleur : hex (`#rgb`, `#rrggbb`, `#rrggbbaa`), ou nom d'une couleur du
/// thème (références suivies, huit au plus : une boucle ne donne rien). Une
/// chaîne en `#` qui n'est pas un hex valide ne donne rien (t-194).
#[must_use]
pub fn resolve_color(colors: &Map<String, Value>, v: &Value) -> Option<String> {
    let mut v = v.as_str()?;
    for _ in 0..8 {
        if v.starts_with('#') {
            return is_hex_color(v).then(|| v.to_ascii_lowercase());
        }
        v = colors.get(v)?.as_str()?;
    }
    None
}

/// Palette des graphiques des jetons, en `#rgb` ou `#rrggbb` minuscules ;
/// une entrée qui ne se résout pas en couleur opaque est ignorée.
pub fn chart_palette(tokens: &Value) -> Vec<String> {
    let empty = Map::new();
    let colors = tokens.get("color").and_then(Value::as_object).unwrap_or(&empty);
    let Some(entries) = tokens.pointer("/chart/palette").and_then(Value::as_array) else { return Vec::new() };
    entries.iter().filter_map(|e| resolve_color(colors, e)).filter(|c| c.len() != 9).collect()
}

/// Première famille de `font.body` des jetons ; `None` si absente ou vide.
pub(crate) fn body_font(tokens: &Value) -> Option<String> {
    let first = tokens.pointer("/font/body")?.as_array()?.first()?.as_str()?.trim();
    (!first.is_empty()).then(|| first.to_owned())
}

#[cfg(test)]
#[path = "palette_tests.rs"]
mod palette_tests;
