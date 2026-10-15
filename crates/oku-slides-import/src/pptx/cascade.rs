//! La cascade des styles texte : la seule fonction qui résout l'héritage OOXML.
//!
//! Couches, du plus faible au plus fort : `TextChain` (défauts de la
//! présentation, `txStyles` du master, `lstStyle` des placeholders du master
//! puis du layout) ; le `fontRef` de `p:style` ; le `lstStyle` de la forme ;
//! `pPr` ; `rPr`. Les couleurs sont résolues à la fin, avec le `ColorCtx` de
//! la part qui affiche le texte. Ce qui reste vide prend les défauts du lecteur.

use crate::pptx::color::ColorCtx;
use crate::pptx::text::{BodyProps, BulletKind, FontRef, LEVELS, LevelProps, ParaProps, RawText, RunProps};
use crate::source::{
    Align, Anchor, Autofit, Bullet, Caps, Color, Insets, LevelStyle, ParaStyle, Paragraph, Rgba, Run, RunStyle,
    Spacing, Styled, TextBox, family_weight,
};

const DEFAULT_SIZE_PT: f64 = 18.0;

/// Couches héritées, de la plus faible à la plus forte.
#[derive(Debug)]
pub(crate) struct TextChain<'a> {
    pub levels: Vec<&'a LevelProps>,
    pub bodies: Vec<&'a BodyProps>,
}

/// Ce que la cascade ne trouve pas dans les couches.
#[derive(Debug)]
pub(crate) struct TextEnv<'a> {
    pub major: &'a str,
    pub minor: &'a str,
    pub colors: &'a ColorCtx,
    pub font_ref: RunProps,
}

impl TextEnv<'_> {
    fn default_color(&self) -> Color {
        self.colors.slot("tx1").map_or_else(
            || Styled::default_value(Rgba::new(0, 0, 0)),
            |(slot, v)| Styled::mapped(v, "tx1", &slot, Vec::new()),
        )
    }
}

fn level_props(chain: &TextChain<'_>, env: &TextEnv<'_>, lst: &LevelProps, level: usize) -> ParaProps {
    let mut acc = ParaProps::default();
    for l in &chain.levels {
        if let Some(p) = l.get(level) {
            acc.overlay(p);
        }
    }
    acc.run.overlay(&env.font_ref);
    if let Some(p) = lst.get(level) {
        acc.overlay(p);
    }
    acc
}

fn run_style(p: &RunProps, env: &TextEnv<'_>) -> RunStyle {
    let font = match &p.font {
        Some(FontRef::Major) => Styled::scheme(env.major.to_owned(), "major"),
        Some(FontRef::Named(n)) => Styled::explicit(n.clone()),
        Some(FontRef::Minor) | None => Styled::scheme(env.minor.to_owned(), "minor"),
    };
    let bold = p.bold.unwrap_or(false);
    let named = family_weight(&font.value).unwrap_or(400);
    RunStyle {
        size: p.size.unwrap_or(DEFAULT_SIZE_PT),
        bold,
        weight: if bold { named.max(700) } else { named },
        font,
        italic: p.italic.unwrap_or(false),
        underline: p.underline.unwrap_or(false),
        strike: p.strike.unwrap_or(false),
        color: p.color.as_ref().and_then(|c| c.resolve(env.colors)).unwrap_or_else(|| env.default_color()),
        highlight: p.highlight.as_ref().and_then(|c| c.resolve(env.colors)),
        baseline: p.baseline.unwrap_or(0),
        caps: p.caps.unwrap_or(Caps::None),
        char_spacing: p.char_spacing.unwrap_or(0.0),
        link: p.link.clone(),
    }
}

fn para_style(p: &ParaProps, env: &TextEnv<'_>) -> ParaStyle {
    let bullet = match p.bullet.kind.clone() {
        Some(BulletKind::Char(char)) => Bullet::Char {
            char,
            color: p.bullet.color.clone().flatten().and_then(|c| c.resolve(env.colors)),
            size_pct: p.bullet.size_pct.unwrap_or(1.0),
            font: p.bullet.font.clone().flatten(),
        },
        Some(BulletKind::AutoNum { scheme, start }) => Bullet::AutoNum { scheme, start },
        Some(BulletKind::None) | None => Bullet::None,
    };
    ParaStyle {
        align: p.align.unwrap_or(Align::Left),
        indent: p.indent.unwrap_or(0.0),
        margin_left: p.margin_left.unwrap_or(0.0),
        line_spacing: p.line_spacing.unwrap_or(Spacing::Percent(1.0)),
        space_before: p.space_before.unwrap_or(Spacing::Points(0.0)),
        space_after: p.space_after.unwrap_or(Spacing::Points(0.0)),
        bullet,
    }
}

/// Résout un corps de texte complet.
pub(crate) fn resolve_textbox(raw: &RawText, chain: &TextChain<'_>, env: &TextEnv<'_>) -> TextBox {
    let paragraphs = raw
        .paragraphs
        .iter()
        .map(|rp| {
            let mut p = level_props(chain, env, &raw.lst, usize::from(rp.level));
            p.overlay(&rp.props);
            let runs = rp
                .runs
                .iter()
                .map(|r| {
                    let mut props = p.run.clone();
                    props.overlay(&r.props);
                    Run { text: r.text.clone(), field: r.field.clone(), style: run_style(&props, env) }
                })
                .collect();
            Paragraph { level: rp.level, style: para_style(&p, env), runs }
        })
        .collect();
    let mut body = BodyProps::default();
    for b in &chain.bodies {
        body.overlay(b);
    }
    body.overlay(&raw.body);
    let d = TextBox::default();
    TextBox {
        paragraphs,
        insets: Insets {
            l: body.insets[0].unwrap_or(d.insets.l),
            t: body.insets[1].unwrap_or(d.insets.t),
            r: body.insets[2].unwrap_or(d.insets.r),
            b: body.insets[3].unwrap_or(d.insets.b),
        },
        anchor: body.anchor.unwrap_or(Anchor::Top),
        autofit: body.autofit.unwrap_or(Autofit::None),
        wrap: body.wrap.unwrap_or(d.wrap),
        columns: body.columns.unwrap_or(d.columns),
    }
}

/// Style du niveau `level` (0..8) d'un placeholder.
pub(crate) fn resolve_level(chain: &TextChain<'_>, lst: &LevelProps, env: &TextEnv<'_>, level: usize) -> LevelStyle {
    let p = level_props(chain, env, lst, level);
    LevelStyle { para: para_style(&p, env), run: run_style(&p.run, env) }
}

/// Style de chaque niveau 0..8 d'un placeholder.
pub(crate) fn resolve_levels(chain: &TextChain<'_>, lst: &LevelProps, env: &TextEnv<'_>) -> Vec<LevelStyle> {
    (0..LEVELS).map(|l| resolve_level(chain, lst, env, l)).collect()
}

#[cfg(test)]
#[path = "cascade_tests.rs"]
mod cascade_tests;
