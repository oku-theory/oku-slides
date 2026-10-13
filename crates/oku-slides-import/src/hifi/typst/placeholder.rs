//! Ce qu'un widget de slide généré dessine en plus du décor : le fond de
//! son layout et ses placeholders de texte, chacun un champ éditable.
//!
//! Un placeholder est un bloc de code : les règles `set` du premier niveau
//! de son style (police, taille, couleur, graisse, bords de ligne, alignement,
//! puce), puis `champ(…)` (`_commun.typ`), qui pose le champ du paramètre à
//! la boîte du placeholder, à ses marges et à son ancrage. Le texte du
//! paramètre (paragraphes du cœur) prend ces règles ; ses runs les
//! surchargent. Les niveaux de liste au-delà du premier ne se distinguent
//! pas : un paragraphe du cœur n'a pas de niveau.

use super::text::{DEFAULT_SIZE, LineBox, align, anchor, inset, spacing};
use super::{Decor, Gen, GenCtx, Typst, pt, string};
use crate::fonts::LineMetrics;
use crate::hifi::page::{PAGE_H, PAGE_W};
use crate::source::{Align, Bullet, Caps, Fill, LevelStyle, Placeholder};

/// Écart entre une puce sans retrait suspendu et le texte.
const MARKER_GAP: &str = "0.25em";
/// Écart entre un numéro et le texte.
const NUMBER_GAP: &str = "0.5em";

/// Le fond `fill` (de l'objet source `id`) sur toute la page Oku, en mode
/// code ; rien sans remplissage.
pub(crate) fn background(id: &str, fill: &Fill, ctx: &GenCtx<'_>) -> Decor {
    let mut g = Gen { w: Typst::new(), ctx: *ctx, assets: Default::default(), losses: Vec::new() };
    let paint = g.fill(id, fill, PAGE_W, PAGE_H);
    if paint != "none" {
        let (w, h) = (pt(PAGE_W), pt(PAGE_H));
        g.w.line(&format!("place(top + left, rect(width: {w}, height: {h}, fill: {paint}, stroke: none))"));
    }
    Decor { code: g.w.finish(), assets: g.assets, losses: g.losses }
}

/// Le placeholder `ph`, champ du paramètre `param` (`args`, `ctx`, `t`,
/// `champ`, `color-of` et `font-of` en portée), en mode code.
pub(crate) fn placeholder(ph: &Placeholder, param: &str, ctx: &GenCtx<'_>) -> String {
    let mut g = Gen { w: Typst::new(), ctx: *ctx, assets: Default::default(), losses: Vec::new() };
    let page = ctx.page;
    let f = page.frame(&ph.frame);
    g.w.open("{");
    if let Some(level) = ph.levels.first() {
        g.level(level);
    }
    let name = string(param);
    g.w.line(&format!(
        "champ(ctx, {name}, args.at({name}), {}, {}, {}, {}, {}, {})",
        pt(f.x),
        pt(f.y),
        pt(f.w),
        pt(f.h),
        inset(page, ph.text.insets),
        anchor(ph.text.anchor)
    ));
    g.w.close("}");
    g.w.finish()
}

impl Gen<'_> {
    /// Les règles `set` du style de niveau `level`.
    fn level(&mut self, level: &LevelStyle) {
        let page = self.ctx.page;
        let (r, p) = (&level.run, &level.para);
        let size = Some(page.len(r.size)).filter(|s| *s > 0.0).unwrap_or_else(|| page.len(DEFAULT_SIZE));
        let mut text =
            format!("font: {}, size: {}, fill: {}", self.font(&r.font.value), pt(size), self.color(&r.color));
        if r.bold {
            text.push_str(", weight: \"bold\"");
        }
        if r.italic {
            text.push_str(", style: \"italic\"");
        }
        if r.char_spacing != 0.0 {
            text.push_str(&format!(", tracking: {}", pt(page.len(r.char_spacing))));
        }
        // Les bords de ligne font le pas de ligne PowerPoint, sans `leading`
        // (voir `text.rs`) ; entre deux paragraphes, les espaces avant et après.
        let metrics = self.ctx.fonts.metrics(&r.font.value).unwrap_or(LineMetrics::DEFAULT);
        let line = LineBox::new(page, size, p.line_spacing, 0.0, metrics);
        self.w.line(&format!("set text({text}, {})", line.edges(size)));
        let gap = spacing(page, size, p.space_before) + spacing(page, size, p.space_after);
        let justify = p.align == Align::Justify;
        self.w.line(&format!("set par(leading: 0pt, spacing: {}, justify: {justify})", pt(gap)));
        self.w.line(&format!("set align({})", align(p.align)));
        match r.caps {
            Caps::All => self.w.line("show: upper"),
            Caps::Small => self.w.line("show: smallcaps"),
            Caps::None => {}
        }
        // La puce est posée au retrait du paragraphe ; un retrait suspendu
        // (négatif) lui fait une boîte de sa largeur, le texte part de la marge.
        let indent = page.len(p.indent);
        let left = pt((page.len(p.margin_left) + indent.min(0.0)).max(0.0));
        let mut list = format!("indent: {left}, spacing: {}", pt(gap));
        if let Bullet::Char { char, color, size_pct, font } = &p.bullet {
            let family = font.as_deref().unwrap_or(&r.font.value);
            let fill = self.color(color.as_ref().unwrap_or(&r.color));
            let mark = format!(
                "text(font: {}, size: {}, fill: {fill}, {})",
                self.font(family),
                pt(size * size_pct),
                string(char)
            );
            if indent < 0.0 {
                list.push_str(&format!(", body-indent: 0pt, marker: box(width: {}, {mark})", pt(-indent)));
            } else {
                list.push_str(&format!(", body-indent: {MARKER_GAP}, marker: {mark}"));
            }
        }
        self.w.line(&format!("set list({list})"));
        self.w.line(&format!("set enum(indent: {left}, body-indent: {NUMBER_GAP}, spacing: {})", pt(gap)));
    }
}

#[cfg(test)]
#[path = "placeholder_tests.rs"]
mod placeholder_tests;
