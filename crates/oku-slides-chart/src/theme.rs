//! Thème de rendu. Les valeurs viennent des jetons Papier (`tokens.json`) ;
//! celles par défaut ne sont que des valeurs de repli neutres.

use serde::{Deserialize, Serialize};

/// Fonts, colors (`#rrggbb`) and sizes (points) of a chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChartTheme {
    /// Famille de police, telle que déclarée dans le fichier de police. Doit
    /// être enregistrée avec [`crate::register_fonts`] et connue de Typst.
    pub font_family: String,
    /// Fond du graphique ; `None` = transparent (le fond de la slide passe).
    pub background: Option<String>,
    /// Color of titles and value labels.
    pub text: String,
    /// Secondary color: subtitle and axis ticks.
    pub text_muted: String,
    /// Color of axis lines.
    pub axis: String,
    /// Color of grid lines.
    pub grid: String,
    /// Couleurs des séries, dans l'ordre.
    pub palette: Vec<String>,
    /// Title size.
    pub title_size: f32,
    /// Subtitle size.
    pub subtitle_size: f32,
    /// Legend text size; the markers follow.
    pub legend_size: f32,
    /// Tick size; the X axis height follows.
    pub axis_size: f32,
    /// Size of the value labels on the series.
    pub label_size: f32,
    /// Width of lines and of point outlines.
    pub stroke_width: f32,
}

impl Default for ChartTheme {
    fn default() -> Self {
        ChartTheme {
            font_family: "Roboto".into(),
            background: None,
            text: "#1F1D1A".into(),
            text_muted: "#6B665E".into(),
            axis: "#C9C3B8".into(),
            grid: "#E7E2D8".into(),
            palette: ["#2F5D8A", "#C4622D", "#5B8A3C", "#8A4F7D", "#C9A227", "#3C8A86"].map(String::from).to_vec(),
            title_size: 18.0,
            subtitle_size: 13.0,
            legend_size: 12.0,
            axis_size: 12.0,
            label_size: 12.0,
            stroke_width: 2.0,
        }
    }
}

/// Converts `#rgb`, `#rrggbb` or `#rrggbbaa` (charts-rs ignores the alpha of hex strings).
///
/// Never panics. Digits are read byte by byte and only `0-9a-fA-F` counts, so a sign, a
/// non-ASCII byte or any other character makes its component read as 0. A fully invalid color
/// is therefore opaque black, except at 8 bytes, where the alpha reads 0 too (transparent).
pub(crate) fn color(hex: &str) -> charts_rs::Color {
    let h = hex.trim().trim_start_matches('#').as_bytes();
    let nibble = |i: usize| h.get(i).copied().and_then(hex_digit);
    let byte = |i: usize| nibble(i).zip(nibble(i + 1)).map_or(0, |(hi, lo)| hi * 16 + lo);
    match h.len() {
        3 => {
            let d = |i: usize| nibble(i).map_or(0, |n| n * 17);
            (d(0), d(1), d(2)).into()
        }
        8 => (byte(0), byte(2), byte(4), byte(6)).into(),
        _ => (byte(0), byte(2), byte(4)).into(),
    }
}

/// Value of one ASCII hex digit, `None` for any other byte.
fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn transparent() -> charts_rs::Color {
    (255, 255, 255, 0).into()
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
