//! Texte d'un décor : une boîte (`block` aux marges et à l'ancrage de la
//! source) qui empile ses paragraphes, chacun un `block` (retrait, puce,
//! alignement, interligne) de runs `text`.
//!
//! Interligne : PowerPoint compte une ligne simple à 1,2 fois la plus grande
//! police du paragraphe ([`line_step`]) et, comme une ligne CSS, centre la
//! police (ascendante plus descendante) dans ce pas : la ligne de base est à
//! la moitié de l'écart, plus l'ascendante, du haut de la ligne
//! ([`LineBox`]). Les bords de ligne Typst sont posés là, sans `leading` : la
//! première ligne de base tombe où PowerPoint la met. Les espaces avant et
//! après s'additionnent ; l'espace avant du premier paragraphe est ignoré,
//! comme dans PowerPoint.

use super::{FontName, Gen, Placement, Typst, pt, string};
use crate::fonts::LineMetrics;
use crate::hifi::page::Page;
use crate::source::{Align, Anchor, Autofit, Bullet, Caps, Insets, Paragraph, Run, RunStyle, Spacing, TextBox};

/// Hauteur d'une ligne simple, en fraction de la taille de police.
const LINE: f64 = 1.2;
/// Taille d'une ligne vide sans run pour la mesurer, en pt source (défaut OOXML).
pub(crate) const DEFAULT_SIZE: f64 = 18.0;
/// Taille d'un exposant ou d'un indice, en fraction de la taille du run.
const SCRIPT_SIZE: f64 = 2.0 / 3.0;
/// Niveaux de paragraphe (`lvl` 0..8).
const LEVELS: usize = 9;
/// Plus longue numérotation alphabétique (`AAAAAAAA`) : au-delà, la lettre ne se répète plus.
const ALPHA_MAX: u32 = 8;

/// Ajustements de l'autofit `shrink` : échelle de police, réduction d'interligne.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fit {
    /// Échelle des tailles de police.
    pub(crate) font_scale: f64,
    /// Réduction de l'interligne proportionnel.
    pub(crate) line_reduction: f64,
}

impl Fit {
    /// Les ajustements de `autofit` (sans effet hors `shrink`).
    pub(crate) fn of(autofit: Autofit) -> Self {
        match autofit {
            Autofit::Shrink { font_scale, line_reduction } if font_scale > 0.0 => Self { font_scale, line_reduction },
            _ => Self { font_scale: 1.0, line_reduction: 0.0 },
        }
    }
}

impl Gen<'_> {
    /// La boîte de texte `tb` dessinée en `at`.
    pub(super) fn text_box(&mut self, at: &Placement, tb: &TextBox) {
        let f = at.frame;
        let page = self.ctx.page;
        let (inset, anchor) = (inset(page, tb.insets), anchor(tb.anchor));
        let fit = Fit::of(tb.autofit);
        let mut numbers = Numbering::default();
        // Le tampon est sorti le temps du dessin : les runs lisent `self`.
        let mut buf = std::mem::take(&mut self.w);
        let w = &mut buf;
        let opened = at.open(w);
        w.open(&format!("block(width: {}, height: {}, inset: {inset}, {{", pt(f.w), pt(f.h)));
        if tb.columns > 1 {
            w.open(&format!("align({anchor}, columns({}, stack(", tb.columns));
        } else {
            w.open(&format!("align({anchor}, stack("));
        }
        let mut after = 0.0;
        let mut last_size = DEFAULT_SIZE * fit.font_scale;
        for (n, p) in tb.paragraphs.iter().enumerate() {
            let size = p.runs.iter().map(|r| r.style.size * fit.font_scale).fold(f64::NAN, f64::max);
            let size = if size.is_finite() && size > 0.0 { size } else { last_size };
            last_size = size;
            if n > 0 {
                let gap = after + spacing(page, page.len(size), p.style.space_before);
                if gap > 0.0 {
                    w.line(&format!("{},", pt(gap)));
                }
            }
            after = spacing(page, page.len(size), p.style.space_after);
            let marker = numbers.next(p);
            self.paragraph(w, p, size, fit, marker.as_deref());
        }
        // `align` est une instruction du bloc de code : sans virgule.
        w.close(if tb.columns > 1 { ")))" } else { "))" });
        w.close("}),");
        Placement::close(w, opened);
        self.w = buf;
    }

    /// Un paragraphe dont la plus grande police fait `size` pt source.
    fn paragraph(&self, w: &mut Typst, p: &Paragraph, size: f64, fit: Fit, number: Option<&str>) {
        let page = self.ctx.page;
        let s = &p.style;
        let indent = page.len(s.indent);
        let left = (page.len(s.margin_left) + indent.min(0.0)).max(0.0);
        if left > 0.0 {
            w.open(&format!("block(width: 100%, inset: (left: {}), {{", pt(left)));
        } else {
            w.open("block(width: 100%, {");
        }
        let size_pt = page.len(size);
        let family = p.runs.iter().find(|r| !r.text.trim().is_empty()).map(|r| r.style.font.value.as_str());
        let metrics = family.and_then(|f| self.ctx.fonts.metrics(f)).unwrap_or(LineMetrics::DEFAULT);
        let line = LineBox::new(page, size_pt, s.line_spacing, fit.line_reduction, metrics);
        let mut par = format!("justify: {}, leading: 0pt", s.align == Align::Justify);
        if indent < 0.0 {
            par.push_str(&format!(", hanging-indent: {}", pt(-indent)));
        } else if indent > 0.0 {
            par.push_str(&format!(", first-line-indent: (amount: {}, all: true)", pt(indent)));
        }
        w.line(&format!("set text(size: {}, {})", pt(size_pt), line.edges(size_pt)));
        w.line(&format!("set par({par})"));
        w.line(&format!("set align({})", align(s.align)));
        let visible = p.runs.iter().any(|r| !r.text.trim().is_empty());
        let first = p.runs.first().filter(|_| visible);
        if let Some(marker) = first.and_then(|r| self.marker(&s.bullet, &r.style, fit, number)) {
            if indent < 0.0 {
                w.line(&format!("box(width: {}, {marker})", pt(-indent)));
            } else {
                w.line(&format!("{marker} + h(0.25em)"));
            }
        }
        if p.runs.is_empty() {
            // Une ligne vide garde sa hauteur.
            w.line("text(\"\\u{200b}\")");
        }
        for r in &p.runs {
            w.line(&self.run(r, fit));
        }
        w.close("}),");
    }

    /// La puce ou le numéro `number` d'un paragraphe dont le premier run a le style `first`.
    fn marker(&self, bullet: &Bullet, first: &RunStyle, fit: Fit, number: Option<&str>) -> Option<String> {
        let size = first.size * fit.font_scale;
        match bullet {
            Bullet::None => None,
            Bullet::Char { char, color, size_pct, font } => {
                let family = font.as_deref().unwrap_or(&first.font.value);
                let color = self.color(color.as_ref().unwrap_or(&first.color));
                let size = self.ctx.page.len(size * size_pct);
                Some(format!("text(font: {}, size: {}, fill: {color}, {})", self.font(family), pt(size), string(char)))
            }
            Bullet::AutoNum { .. } => {
                let size = self.ctx.page.len(size);
                let (font, color) = (self.font(&first.font.value), self.color(&first.color));
                Some(format!("text(font: {font}, size: {}, fill: {color}, {})", pt(size), string(number?)))
            }
        }
    }

    /// Un run : `text` et ses décorations ; un saut de ligne `linebreak()`.
    fn run(&self, r: &Run, fit: Fit) -> String {
        if r.text == "\n" {
            return "linebreak()".to_owned();
        }
        let s = &r.style;
        let mut size = s.size * fit.font_scale;
        let mut args = format!("font: {}", self.font(&s.font.value));
        if s.baseline != 0 {
            let shift = -f64::from(s.baseline) / 100.0 * size;
            size *= SCRIPT_SIZE;
            args.push_str(&format!(", baseline: {}", pt(self.ctx.page.len(shift))));
        }
        args = format!("{args}, size: {}", pt(self.ctx.page.len(size)));
        if let Some(w) = weight_arg(s) {
            args.push_str(&format!(", weight: {w}"));
        }
        if s.italic {
            args.push_str(", style: \"italic\"");
        }
        args.push_str(&format!(", fill: {}", self.color(&s.color)));
        if s.char_spacing != 0.0 {
            args.push_str(&format!(", tracking: {}", pt(self.ctx.page.len(s.char_spacing))));
        }
        let body = if s.caps == Caps::All { format!("upper({})", string(&r.text)) } else { string(&r.text) };
        let mut out = format!("text({args}, {body})");
        if s.caps == Caps::Small {
            out = format!("smallcaps({out})");
        }
        if s.underline {
            out = format!("underline({out})");
        }
        if s.strike {
            out = format!("strike({out})");
        }
        if let Some(h) = &s.highlight {
            out = format!("highlight(fill: {}, {out})", self.color(h));
        }
        if let Some(link) = &s.link {
            out = format!("link({}, {out})", string(link));
        }
        out
    }

    /// La police de la famille source `family` ([`super::GenCtx::font_name`]) : un
    /// rôle du thème par `font-of`, pour suivre les polices du thème du deck ;
    /// une autre famille retenue en dur ; sinon celle du corps.
    pub(super) fn font(&self, family: &str) -> String {
        match self.ctx.font_name(family) {
            Some(FontName::Role(role)) => format!("font-of(t, {})", string(role)),
            Some(FontName::Family(f)) => format!("({},)", string(f)),
            None => "font-of(t, \"body\")".to_owned(),
        }
    }
}

/// L'alignement horizontal Typst d'un paragraphe (justifié : `left` et `par(justify)`).
pub(super) fn align(a: Align) -> &'static str {
    match a {
        Align::Left | Align::Justify => "left",
        Align::Center => "center",
        Align::Right => "right",
    }
}

/// Les marges intérieures `i` d'une boîte, en dictionnaire Typst de pt page.
pub(super) fn inset(page: &Page, i: Insets) -> String {
    format!(
        "(top: {}, right: {}, bottom: {}, left: {})",
        pt(page.len(i.t)),
        pt(page.len(i.r)),
        pt(page.len(i.b)),
        pt(page.len(i.l))
    )
}

/// L'alignement vertical Typst d'un ancrage.
pub(super) fn anchor(a: Anchor) -> &'static str {
    match a {
        Anchor::Top => "top",
        Anchor::Middle => "horizon",
        Anchor::Bottom => "bottom",
    }
}

/// Le pas de ligne PowerPoint (pt page, d'une ligne de base à la suivante)
/// d'un interligne source `s` pour une police de `size_pt` pt page, réduit de
/// `reduction` (autofit `shrink`) : `1,2 × p × T`, ou `v` pt exacts.
fn line_step(page: &Page, size_pt: f64, s: Spacing, reduction: f64) -> f64 {
    match s {
        Spacing::Percent(v) => LINE * (v - reduction).max(0.0) * size_pt,
        Spacing::Points(v) => page.len(v),
    }
}

/// Une ligne de texte PowerPoint : son pas et la place de sa ligne de base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineBox {
    /// Pas, en pt page, d'une ligne de base à la suivante ([`line_step`]).
    pub(crate) step: f64,
    /// Du haut de la ligne à sa ligne de base, en pt page : la moitié de ce
    /// que le pas laisse à la police (négative si elle déborde), plus son
    /// ascendante.
    pub(crate) ascent: f64,
}

impl LineBox {
    /// La ligne d'un interligne source `s`, pour une police de `size_pt` pt
    /// page aux métriques `m`, réduit de `reduction` (autofit `shrink`).
    pub(crate) fn new(page: &Page, size_pt: f64, s: Spacing, reduction: f64, m: LineMetrics) -> Self {
        let step = line_step(page, size_pt, s, reduction);
        let gap = step - (m.ascender + m.descender) * size_pt;
        Self { step, ascent: gap / 2.0 + m.ascender * size_pt }
    }

    /// De la ligne de base au bas de la ligne, en pt page.
    pub(crate) fn descent(self) -> f64 {
        self.step - self.ascent
    }

    /// Les bords de ligne Typst (`top-edge`, `bottom-edge`) de cette ligne,
    /// en em d'une police de `size_pt` pt page : un run d'une autre taille
    /// les met à la sienne.
    pub(super) fn edges(self, size_pt: f64) -> String {
        // Au millième d'em : un centième décalerait une grande police de près d'un point.
        let em = |v: f64| {
            let v = if size_pt > 0.0 { v / size_pt } else { 0.0 };
            let v = if v.is_finite() { (v * 1000.0).round() / 1000.0 } else { 0.0 };
            format!("{}em", v + 0.0)
        };
        format!("top-edge: {}, bottom-edge: {}", em(self.ascent), em(-self.descent()))
    }
}

/// Un espacement en pt page, pour une police de `size_pt` pt page.
pub(crate) fn spacing(page: &Page, size_pt: f64, s: Spacing) -> f64 {
    match s {
        Spacing::Percent(v) => LINE * v * size_pt,
        Spacing::Points(v) => page.len(v),
    }
    .max(0.0)
}

/// Compteurs des numérotations automatiques, par niveau : un paragraphe
/// numéroté suit le précédent du même niveau et du même schéma ; tout autre
/// paragraphe de ce niveau ou d'un niveau moins profond la recommence.
#[derive(Debug, Default)]
struct Numbering {
    levels: [Option<(String, u32)>; LEVELS],
}

impl Numbering {
    /// Le numéro du paragraphe `p`, s'il est numéroté.
    fn next(&mut self, p: &Paragraph) -> Option<String> {
        let level = usize::from(p.level).min(LEVELS - 1);
        for deeper in &mut self.levels[level + 1..] {
            *deeper = None;
        }
        let Bullet::AutoNum { scheme, start } = &p.style.bullet else {
            self.levels[level] = None;
            return None;
        };
        let n = match &self.levels[level] {
            Some((s, n)) if s == scheme => n.saturating_add(1),
            _ => *start,
        };
        self.levels[level] = Some((scheme.clone(), n));
        Some(autonum(scheme, n))
    }
}

/// Le numéro `n` dans le schéma OOXML `scheme` (`arabicPeriod` → `3.`,
/// `alphaLcParenR` → `c)`, `romanUcPeriod` → `III.`) ; un schéma inconnu
/// compte en chiffres arabes suivis d'un point.
fn autonum(scheme: &str, n: u32) -> String {
    type Counter = fn(u32) -> String;
    let kinds: [(&str, Counter); 5] = [
        ("arabic", |n| n.to_string()),
        ("alphaLc", |n| alpha(n).to_lowercase()),
        ("alphaUc", alpha),
        ("romanLc", |n| roman(n).to_lowercase()),
        ("romanUc", roman),
    ];
    let (rest, value) = kinds
        .iter()
        .find_map(|(prefix, f)| scheme.strip_prefix(prefix).map(|rest| (rest, f(n))))
        .unwrap_or(("Period", n.to_string()));
    match rest {
        "ParenR" => format!("{value})"),
        "ParenBoth" => format!("({value})"),
        "Plain" => value,
        _ => format!("{value}."),
    }
}

/// `A`…`Z`, puis `AA`…`ZZ`, `AAA`… (comme PowerPoint) ; `0` donne `A`.
fn alpha(n: u32) -> String {
    let n = n.max(1) - 1;
    let letter = char::from(b'A' + u8::try_from(n % 26).unwrap_or(0));
    std::iter::repeat_n(letter, (n / 26 + 1).min(ALPHA_MAX) as usize).collect()
}

/// Chiffres romains (1..3999) ; au-delà ou 0, chiffres arabes.
fn roman(n: u32) -> String {
    if n == 0 || n > 3999 {
        return n.to_string();
    }
    const DIGITS: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = n;
    let mut out = String::new();
    for (v, s) in DIGITS {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

/// L'argument `weight` de Typst pour `s` : `"bold"` pour un gras ordinaire, sinon la graisse
/// numérique (celle que nomme la famille, que `font` ôte du nom), rien pour 400.
pub(super) fn weight_arg(s: &RunStyle) -> Option<String> {
    match (s.bold, s.weight) {
        (true, w) if w <= 700 => Some("\"bold\"".to_owned()),
        (_, 400) => None,
        (_, w) => Some(w.to_string()),
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
