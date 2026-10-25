//! En-tête des graphiques : titre, sous-titre et légende. Dessiné ici, pas
//! par charts-rs, dont le repère de légende a une taille fixe (25×20) calée
//! sur le haut du texte : ici tout suit les tailles de texte du thème, et
//! charts-rs laisse la place de l'en-tête en haut du graphique.

use std::fmt::Write;

use charts_rs::{ChartBase, measure_text_width_family};

use crate::option::HAlign;
use crate::theme::ChartTheme;

/// Hauteur d'une ligne de titre, de sous-titre ou de légende, en tailles de texte.
const LINE: f32 = 1.4;
/// Repère de légende, en tailles du texte de légende : largeur, rayon du
/// point, épaisseur du trait, écart avant le texte, écart entre deux entrées.
const MARKER_WIDTH: f32 = 2.0;
const MARKER_RADIUS: f32 = 0.45;
const MARKER_STROKE: f32 = 1.0 / 6.0;
const MARKER_GAP: f32 = 0.3;
const ENTRY_GAP: f32 = 0.8;
/// Écart entre l'en-tête et le tracé, en tailles du texte de légende.
const BOTTOM_GAP: f32 = 0.4;

/// Titres et légende d'un graphique, et leur alignement.
#[derive(Debug)]
pub(crate) struct Header<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub title_align: HAlign,
    pub legend: bool,
    pub legend_align: HAlign,
}

impl Header<'_> {
    /// Dessine l'en-tête au-dessus de la zone de `base` et décale celle-ci
    /// d'autant ; charts-rs n'y dessine plus ni titre ni légende. Rend le
    /// fragment SVG à ajouter au graphique (vide sans en-tête).
    pub(crate) fn place(&self, base: &mut ChartBase, theme: &ChartTheme) -> String {
        base.title_text.clear();
        base.sub_title_text.clear();
        base.legend_show = Some(false);

        let left = base.margin.left;
        let width = (base.width - base.margin.left - base.margin.right).max(0.0);
        let mut y = base.margin.top;
        let mut svg = String::new();
        let mut text = |y: f32, size: f32, align: HAlign, fill: &str, bold: bool, content: &str| {
            let (x, anchor) = match align {
                HAlign::Left => (left, "start"),
                HAlign::Center => (left + width / 2.0, "middle"),
                HAlign::Right => (left + width, "end"),
            };
            let content = ellipsis(&theme.font_family, size, content, width);
            text_tag(&mut svg, x, y, size, anchor, fill, bold, &theme.font_family, &content);
        };
        if !self.title.is_empty() {
            let h = theme.title_size * LINE;
            text(y + h / 2.0, theme.title_size, self.title_align, &theme.text, true, self.title);
            y += h;
        }
        if !self.subtitle.is_empty() {
            let h = theme.subtitle_size * LINE;
            text(y + h / 2.0, theme.subtitle_size, self.title_align, &theme.text_muted, false, self.subtitle);
            y += h;
        }
        let entries = if self.legend { legend_entries(base) } else { Vec::new() };
        if !entries.is_empty() {
            y += legend(&mut svg, &entries, theme, self.legend_align, (left, width), y);
        }
        if y > base.margin.top {
            y += theme.legend_size * BOTTOM_GAP;
        }
        base.margin.top = y;
        if svg.is_empty() { svg } else { format!("<g>\n{svg}</g>\n") }
    }
}

/// Ajoute l'en-tête à la fin du SVG du graphique (par-dessus son fond).
pub(crate) fn append(svg: String, header: &str) -> String {
    match svg.rfind("</svg>") {
        Some(end) if !header.is_empty() => format!("{}{header}{}", &svg[..end], &svg[end..]),
        _ => svg,
    }
}

/// Nom et couleur des séries nommées, comme la légende de charts-rs.
fn legend_entries(base: &ChartBase) -> Vec<(String, String)> {
    let colors = &base.series_colors;
    base.series_list
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.name.is_empty())
        .map(|(i, s)| {
            let c =
                colors.get(s.index.unwrap_or(i) % colors.len().max(1)).map(charts_rs::Color::hex).unwrap_or_default();
            (s.name.clone(), c)
        })
        .collect()
}

/// Dessine la légende en lignes à partir de `top`, sur la bande horizontale
/// `(gauche, largeur)` ; rend sa hauteur.
fn legend(
    svg: &mut String,
    entries: &[(String, String)],
    theme: &ChartTheme,
    align: HAlign,
    (left, width): (f32, f32),
    top: f32,
) -> f32 {
    let size = theme.legend_size;
    let marker = size * MARKER_WIDTH;
    let widths: Vec<f32> = entries
        .iter()
        .map(|(name, _)| marker + size * MARKER_GAP + text_width(&theme.font_family, size, name))
        .collect();
    let rows = rows(&widths, size * ENTRY_GAP, width);
    let line = size * LINE;
    for (r, (range, row_width)) in rows.iter().enumerate() {
        let mut x = match align {
            HAlign::Left => left,
            HAlign::Center => left + ((width - row_width) / 2.0).max(0.0),
            HAlign::Right => left + (width - row_width).max(0.0),
        };
        let cy = top + count(r) * line + line / 2.0;
        for i in range.clone() {
            let (name, color) = &entries[i];
            let stroke = size * MARKER_STROKE;
            let _ = writeln!(
                svg,
                r#"<line x1="{x}" y1="{cy}" x2="{}" y2="{cy}" stroke="{color}" stroke-width="{stroke}"/>"#,
                x + marker
            );
            let _ = writeln!(
                svg,
                r#"<circle cx="{}" cy="{cy}" r="{}" stroke="{color}" stroke-width="{stroke}" fill="{}"/>"#,
                x + marker / 2.0,
                size * MARKER_RADIUS,
                // Le fond, comme charts-rs : sans fond, le trait passe dans le point.
                theme.background.as_deref().unwrap_or("none"),
            );
            let tx = x + marker + size * MARKER_GAP;
            text_tag(svg, tx, cy, size, "start", &theme.text, false, &theme.font_family, name);
            x += widths[i] + size * ENTRY_GAP;
        }
    }
    count(rows.len()) * line
}

/// Entrées réparties en lignes de `width` au plus : (indices, largeur).
fn rows(widths: &[f32], gap: f32, width: f32) -> Vec<(std::ops::Range<usize>, f32)> {
    let mut rows = Vec::new();
    let (mut start, mut used) = (0, 0.0);
    for (i, w) in widths.iter().enumerate() {
        if i > start && used + gap + w > width {
            rows.push((start..i, used));
            (start, used) = (i, 0.0);
        }
        used += if i > start { gap + w } else { *w };
    }
    if start < widths.len() {
        rows.push((start..widths.len(), used));
    }
    rows
}

/// A count (legend lines, characters) as `f32`, for geometry.
#[expect(clippy::cast_precision_loss, reason = "counts far below 2^24, exact in f32")]
fn count(n: usize) -> f32 {
    n as f32
}

/// Largeur d'un texte ; sans police enregistrée, une estimation.
fn text_width(family: &str, size: f32, text: &str) -> f32 {
    measure_text_width_family(family, size, text)
        .map_or_else(|_| count(text.chars().count()) * size * 0.55, |b| b.width())
}

/// Le texte, coupé avec « … » s'il dépasse `width`.
fn ellipsis(family: &str, size: f32, text: &str, width: f32) -> String {
    if text_width(family, size, text) <= width {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut: String = chars.iter().collect::<String>() + "…";
        if text_width(family, size, &cut) <= width {
            return cut;
        }
    }
    String::new()
}

#[expect(clippy::too_many_arguments, reason = "un <text> SVG : position, taille, ancre, couleur, graisse, police")]
fn text_tag(
    svg: &mut String,
    x: f32,
    y: f32,
    size: f32,
    anchor: &str,
    fill: &str,
    bold: bool,
    family: &str,
    content: &str,
) {
    let weight = if bold { r#" font-weight="bold""# } else { "" };
    let _ = writeln!(
        svg,
        r#"<text x="{x}" y="{y}" font-size="{size}" font-family="{}" fill="{fill}" text-anchor="{anchor}" dominant-baseline="middle"{weight}>{}</text>"#,
        escape(family),
        escape(content),
    );
}

/// Échappe un texte pour un attribut ou un contenu XML.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
