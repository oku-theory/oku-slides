//! Texte d'un élément hi-fi : une boîte source en élément `text`, corps
//! riche et style de la boîte.
//!
//! L'élément porte le style dominant de la boîte, celui du plus grand nombre de
//! caractères : taille, couleur, police (son rôle `display` ou `body` quand
//! c'est celle du thème), alignement, interligne ; gras et italique s'ils
//! couvrent tout le texte ; l'espacement des lettres (`tracking`, `spc`
//! source) ; l'ancrage (`valign`) et les marges (`inset`) de la source ; les
//! retraits (`indent` : `marL`, `indent`) des paragraphes du premier niveau,
//! puce suspendue comprise. Une marge ou un retrait hors des bornes du cœur y
//! est ramené (`ParagraphStyle`). Un run d'une autre taille ou d'une autre
//! couleur la porte. Une autre police est perdue (`RunStyle(Font)`), un autre
//! espacement des lettres aussi (`RunStyle(Tracking)`), comme un autre
//! alignement, interligne ou retrait de paragraphe, ou un texte justifié
//! (`ParagraphStyle`). L'interligne, en em de la taille du paragraphe, rend le
//! pas de ligne de PowerPoint ([`LineBox`]) : le widget `text` garde les bords
//! de ligne Typst par défaut (capitales, ligne de base), le pas y vaut la
//! hauteur des capitales de la police plus l'interligne. Les marges le
//! recalent : la première ligne de base est descendue où PowerPoint la pose
//! (`LineBox::ascent` sous le haut, pas la hauteur des capitales), la dernière
//! remontée de la descente de sa ligne, que Typst ne compte pas ; ainsi
//! l'ancrage haut, milieu ou bas tombe juste. L'espace entre paragraphes
//! (`spacing`, pt) rend celui de PowerPoint : la descente de la ligne, l'espace
//! après et l'espace avant, la montée de la ligne suivante, moins ses
//! capitales ; un autre espace après ou avant est perdu (`ParagraphStyle`). Un
//! surlignage devient la pastille de la palette (`chart.palette`) de même
//! couleur, sinon il est perdu.
//!
//! Un placeholder de slide remplit un paramètre du widget de son layout
//! ([`placeholder_text`]) : ses runs se marquent contre le style du
//! placeholder de layout, que le widget pose.
//!
//! Comme au retrofit ([`crate::retrofit::text`]) : capitales appliquées,
//! paragraphes vides omis, runs voisins de même forme fusionnés, niveaux de
//! liste aplatis, liens sûrs seulement, bornes du texte riche.
//!
//! Écritures : la police d'un rôle du thème donne aussi ses familles `ea` et
//! `cs` ([`super::theme::role_families`]) ; un run dont les caractères d'Asie
//! de l'Est ou complexes demandent une autre famille la perd
//! (`RunStyle(Font)`). La langue est celle du deck (`lang`) ; un paragraphe
//! d'un autre sens d'écriture que celle-ci est perdu (`ParagraphStyle`).

use oku_slides_core::{
    Element, Frame, Paragraph, Run, STYLE_INDENT_MAX, STYLE_INSET_MAX, STYLE_SPACING_MAX, STYLE_TRACKING_MAX,
    TokenValue,
};

use super::element::ElementCtx;
use super::page::{Page, round2};
use super::typst::{DEFAULT_SIZE, Fit, FontName, LineBox, spacing};
use crate::report::{LossKind, RunEffect};
use crate::retrofit::text::{bound, effects, list_kind, note, safe_link, shown};
use crate::source::{self, Align, Anchor, Color, LevelStyle, Script, TextBox};

/// Le style que les runs d'un texte reçoivent de leur boîte, et n'ont pas à
/// redire : un run d'une autre taille, d'une autre couleur, gras ou
/// italique sur une boîte qui ne l'est pas, porte sa différence.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct Base {
    /// Taille, en pt page.
    pub(super) size: Option<f64>,
    /// Couleur Oku.
    pub(super) color: Option<String>,
    /// Police Oku ([`font_name`]).
    pub(super) font: Option<String>,
    /// Gras.
    pub(super) bold: bool,
    /// Italique.
    pub(super) italic: bool,
    /// Espacement des lettres, en pt page ([`tracking`]) ; 0 : aucun.
    pub(super) tracking: f64,
}

/// Le texte `tb` en élément `text` d'id `id` posé en `frame`, écarté des
/// bords de `inset` (pt page, avant le recalage des lignes de base), et ses
/// pertes ; pas d'élément s'il est vide.
pub(super) fn text_element(
    id: String,
    tb: &TextBox,
    frame: Frame,
    mut inset: [f64; 4],
    ctx: &ElementCtx<'_>,
) -> (Option<Element>, Vec<LossKind>) {
    let mut losses = Vec::new();
    let page = ctx.base.page;
    let colors = ctx.base.colors;
    let fit = Fit::of(tb.autofit);
    let size_of = |r: &source::Run| page.len(r.style.size * fit.font_scale);
    let runs = || tb.paragraphs.iter().flat_map(|p| &p.runs).filter(|r| !r.text.is_empty() && r.text != "\n");
    let base = Base {
        size: dominant(runs().map(|r| (size_of(r), weight(r)))),
        color: dominant(runs().map(|r| (colors.write(&r.style.color), weight(r)))),
        font: dominant(runs().map(|r| (font_name(&r.style.font.value, ctx), weight(r)))).flatten(),
        bold: runs().all(|r| r.style.strong()),
        italic: runs().all(|r| r.style.italic),
        tracking: dominant(runs().map(|r| (tracking(&r.style, page), weight(r)))).unwrap_or_default(),
    };
    // Un espacement des lettres au-delà des bornes du cœur y est ramené : perdu.
    if runs().any(|r| page.len(r.style.char_spacing).abs() > STYLE_TRACKING_MAX) {
        note(&mut losses, LossKind::RunStyle(RunEffect::Tracking));
    }
    let family = dominant(runs().map(|r| (r.style.font.value.as_str(), weight(r))));
    let paragraphs = rich_paragraphs(tb, &base, ctx, &mut losses);
    if paragraphs.is_empty() {
        return (None, losses);
    }

    // Alignement et interligne : ceux des paragraphes qui ont du texte.
    let shown_paras: Vec<&source::Paragraph> =
        tb.paragraphs.iter().filter(|p| p.runs.iter().any(|r| !r.text.trim().is_empty())).collect();
    let para_weight = |p: &source::Paragraph| p.runs.iter().map(weight).sum::<usize>();
    let metrics = ctx.base.line_metrics(family);
    // La plus grande police d'un paragraphe (pt page) et sa ligne PowerPoint.
    let line_of = |p: &source::Paragraph| {
        let size = p.runs.iter().map(size_of).fold(f64::NAN, f64::max);
        let size = if size > 0.0 { size } else { page.len(DEFAULT_SIZE) };
        (size, LineBox::new(page, size, p.style.line_spacing, fit.line_reduction, metrics))
    };
    let lead_of = |p: &source::Paragraph| {
        let (size, line) = line_of(p);
        round2(line.step / size - metrics.cap_height)
    };
    let align = dominant(shown_paras.iter().map(|p| (p.style.align, para_weight(p)))).unwrap_or_default();
    let lead = dominant(shown_paras.iter().map(|p| (lead_of(p), para_weight(p))));
    // Espace entre paragraphes : de la dernière ligne de base de l'un au haut
    // des capitales du suivant, où Typst le compte. PowerPoint pose la ligne
    // de base suivante à la descente de la ligne, plus l'espace après de l'un
    // et l'espace avant de l'autre, plus la montée de la ligne suivante.
    let gap_of = |a: &source::Paragraph, b: &source::Paragraph| {
        let (size_a, size_b) = (line_of(a).0, line_of(b).0);
        spacing(page, size_a, a.style.space_after) + spacing(page, size_b, b.style.space_before)
    };
    let raw_spacing = |a: &source::Paragraph, b: &source::Paragraph| {
        let ((_, line_a), (size_b, line_b)) = (line_of(a), line_of(b));
        round2(line_a.descent() + gap_of(a, b) + line_b.ascent - metrics.cap_height * size_b)
    };
    let spacing_of = |a: &source::Paragraph, b: &source::Paragraph| raw_spacing(a, b).clamp(0.0, STYLE_SPACING_MAX);
    let pairs = || shown_paras.windows(2).map(|w| (w[0], w[1]));
    // Un espace entre paragraphes au-delà de la borne du cœur y est ramené : perdu.
    if pairs().any(|(a, b)| raw_spacing(a, b) > STYLE_SPACING_MAX) {
        note(&mut losses, LossKind::ParagraphStyle);
    }
    let gap = dominant(pairs().map(|(a, b)| (round2(gap_of(a, b)), para_weight(b))));
    let between = dominant(pairs().map(|(a, b)| (spacing_of(a, b), para_weight(b))));
    let mixed = shown_paras.iter().any(|p| p.style.align != align || Some(lead_of(p)) != lead)
        || pairs().any(|(a, b)| Some(round2(gap_of(a, b))) != gap);
    if mixed || align == Align::Justify {
        note(&mut losses, LossKind::ParagraphStyle);
    }

    // Retraits : ceux des paragraphes du premier niveau (les autres niveaux
    // sont aplatis, perte `ListLevels`) ; un autre retrait est perdu.
    let top_level = || shown_paras.iter().filter(|p| p.level == 0);
    let indent_of = |p: &source::Paragraph| [round2(page.len(p.style.margin_left)), round2(page.len(p.style.indent))];
    let indent = dominant(top_level().map(|p| (indent_of(p), para_weight(p))));
    if top_level().any(|p| Some(indent_of(p)) != indent) {
        note(&mut losses, LossKind::ParagraphStyle);
    }
    let indent = indent.filter(|i| *i != [0.0, 0.0]).map(|[margin, first]| {
        let bounded = [margin.clamp(0.0, STYLE_INDENT_MAX), first.clamp(-STYLE_INDENT_MAX, STYLE_INDENT_MAX)];
        if !(0.0..=STYLE_INDENT_MAX).contains(&margin) || !(-STYLE_INDENT_MAX..=STYLE_INDENT_MAX).contains(&first) {
            note(&mut losses, LossKind::ParagraphStyle);
        }
        bounded
    });

    // Typst pose le haut des capitales de la première ligne au bord de la
    // zone et la dernière ligne de base au bord bas.
    if let (Some(first), Some(last)) = (shown_paras.first(), shown_paras.last()) {
        let ((size, top), (_, bottom)) = (line_of(first), line_of(last));
        inset[0] += top.ascent - metrics.cap_height * size;
        inset[2] += bottom.descent();
    }
    // Une marge hors des bornes du cœur (négative : une ligne trop serrée,
    // une marge source négative) y est ramenée : le texte se décale, perdu.
    let raw = inset.map(round2);
    let inset = raw.map(|v| v.clamp(0.0, STYLE_INSET_MAX));
    if raw.iter().any(|v| !(0.0..=STYLE_INSET_MAX).contains(v)) {
        note(&mut losses, LossKind::ParagraphStyle);
    }

    let mut e = Element::text(id, paragraphs);
    e.frame = Some(frame);
    let style = &mut e.style;
    style.size = base.size.map(TokenValue::Number);
    style.color = base.color;
    style.font = base.font;
    style.weight = base.bold.then(|| TokenValue::Name("bold".to_owned()));
    style.italic = base.italic.then_some(true);
    style.tracking = (base.tracking != 0.0).then_some(base.tracking);
    style.align = Some(align_name(align).to_owned());
    style.leading = lead.map(TokenValue::Number);
    style.spacing = between;
    style.valign = Some(valign_name(tb.anchor).to_owned());
    style.inset = Some(inset);
    style.indent = indent;
    (Some(e), losses)
}

/// Le texte `tb` d'un placeholder de slide en paragraphes du cœur, sur le
/// style `level` du placeholder de layout dont le widget pose le champ, et
/// ses pertes. Un run maigre ou droit sur un niveau gras ou italique le
/// devient (un run du cœur ajoute du gras, il n'en ôte pas) : perte
/// `RunStyle(Weight)` ou `RunStyle(Slant)`.
pub(super) fn placeholder_text(
    tb: &TextBox,
    level: &LevelStyle,
    ctx: &ElementCtx<'_>,
) -> (Vec<Paragraph>, Vec<LossKind>) {
    let page = ctx.base.page;
    let r = &level.run;
    let size = Some(page.len(r.size)).filter(|s| *s > 0.0).unwrap_or_else(|| page.len(DEFAULT_SIZE));
    let base = Base {
        size: Some(size),
        color: Some(ctx.base.colors.write(&r.color)),
        font: font_name(&r.font.value, ctx),
        bold: r.strong(),
        italic: r.italic,
        tracking: tracking(r, page),
    };
    let mut losses = Vec::new();
    // Le widget pose les retraits du niveau ; un autre retrait est perdu.
    let level_indent = (level.para.margin_left, level.para.indent);
    let shown = tb.paragraphs.iter().filter(|p| p.level == 0 && p.runs.iter().any(|r| !r.text.trim().is_empty()));
    if shown.into_iter().any(|p| (p.style.margin_left, p.style.indent) != level_indent) {
        note(&mut losses, LossKind::ParagraphStyle);
    }
    let paragraphs = rich_paragraphs(tb, &base, ctx, &mut losses);
    (paragraphs, losses)
}

/// Les paragraphes du cœur de `tb` sur le style `base`, dans les bornes du
/// texte riche ; pertes dans `losses`.
fn rich_paragraphs(tb: &TextBox, base: &Base, ctx: &ElementCtx<'_>, losses: &mut Vec<LossKind>) -> Vec<Paragraph> {
    let (page, colors) = (ctx.base.page, ctx.base.colors);
    let fit = Fit::of(tb.autofit);
    let mut paragraphs = Vec::new();
    for sp in &tb.paragraphs {
        let mut out: Vec<Run> = Vec::new();
        for r in sp.runs.iter().filter(|r| !r.text.is_empty()) {
            if r.text == "\n" {
                // Un saut de ligne prend la forme du run qui le précède.
                match out.last_mut() {
                    Some(last) => last.t.push('\n'),
                    None => out.push(Run::plain("\n")),
                }
                continue;
            }
            for e in effects(&r.style).filter(|e| *e != RunEffect::Highlight) {
                note(losses, LossKind::RunStyle(e));
            }
            if font_name(&r.style.font.value, ctx) != base.font || !scripts_covered(r, base, ctx) {
                note(losses, LossKind::RunStyle(RunEffect::Font));
            }
            #[expect(clippy::float_cmp, reason = "two spacings rounded to the hundredth: exact equality is intended")]
            let other_tracking = tracking(&r.style, page) != base.tracking;
            if other_tracking {
                note(losses, LossKind::RunStyle(RunEffect::Tracking));
            }
            // Un run du cœur ajoute du gras ou de l'italique, il n'en ôte pas.
            if base.bold && !r.style.strong() {
                note(losses, LossKind::RunStyle(RunEffect::Weight));
            }
            if base.italic && !r.style.italic {
                note(losses, LossKind::RunStyle(RunEffect::Slant));
            }
            let highlight = r.style.highlight.as_ref().and_then(|h| {
                let n = pastille(h, ctx);
                if n.is_none() {
                    note(losses, LossKind::RunStyle(RunEffect::Highlight));
                }
                n
            });
            let (color, size) = (colors.write(&r.style.color), page.len(r.style.size * fit.font_scale));
            let run = Run {
                t: shown(r),
                bold: r.style.strong() && !base.bold,
                italic: r.style.italic && !base.italic,
                color: (base.color.as_ref() != Some(&color)).then_some(color),
                size: (base.size != Some(size)).then_some(TokenValue::Number(size)),
                link: safe_link(r.style.link.as_deref()),
                highlight,
            };
            match out.last_mut() {
                Some(last) if same_format(last, &run) => last.t.push_str(&run.t),
                _ => out.push(run),
            }
        }
        if out.iter().all(|r| r.t.trim().is_empty()) {
            continue;
        }
        if sp.level > 0 {
            note(losses, LossKind::ListLevels);
        }
        // Le sens d'écriture d'un élément est celui de la langue du deck.
        if sp.style.rtl != ctx.base.deck_rtl() {
            note(losses, LossKind::ParagraphStyle);
        }
        paragraphs.push(Paragraph { runs: out, list: list_kind(&sp.style.bullet), heading: None });
    }
    let (paragraphs, truncated) = bound(paragraphs);
    if truncated {
        note(losses, LossKind::TextTruncated);
    }
    paragraphs
}

/// L'espacement des lettres du style `s` (`spc`), en pt page, dans les
/// bornes du cœur.
fn tracking(s: &source::RunStyle, page: &Page) -> f64 {
    round2(page.len(s.char_spacing)).clamp(-STYLE_TRACKING_MAX, STYLE_TRACKING_MAX)
}

/// Poids d'un run dans le style dominant : son nombre de caractères.
fn weight(r: &source::Run) -> usize {
    r.text.chars().count()
}

/// La valeur de plus grand poids total, la première en cas d'égalité ;
/// `None` sans valeur.
pub(super) fn dominant<T: PartialEq>(values: impl IntoIterator<Item = (T, usize)>) -> Option<T> {
    let mut tally: Vec<(T, usize)> = Vec::new();
    for (v, w) in values {
        match tally.iter_mut().find(|(t, _)| *t == v) {
            Some((_, n)) => *n += w,
            None => tally.push((v, w)),
        }
    }
    let mut best: Option<(T, usize)> = None;
    for (v, n) in tally {
        if best.as_ref().is_none_or(|(_, b)| n > *b) {
            best = Some((v, n));
        }
    }
    best.map(|(v, _)| v)
}

/// Les caractères d'Asie de l'Est et des écritures complexes du run `r`
/// ont-ils leur famille (`ea`, `cs`) dans la police de l'élément ? Une
/// famille que l'import ne retient pas n'est pas comptée : elle manque au
/// rapport des polices.
fn scripts_covered(r: &source::Run, base: &Base, ctx: &ElementCtx<'_>) -> bool {
    let name = match base.font.as_deref() {
        None | Some("body") => FontName::Role("body"),
        Some("display") => FontName::Role("display"),
        Some(f) => FontName::Family(f),
    };
    let provided = ctx.base.families(name);
    [(Script::EastAsian, &r.style.font_ea), (Script::Complex, &r.style.font_cs)].into_iter().all(|(script, f)| {
        let used = f.as_ref().and_then(|f| ctx.base.fonts.used(&f.value));
        used.is_none_or(|u| provided.contains(&u)) || !script.occurs_in(&r.text)
    })
}

/// La police Oku de la famille source `family` ([`GenCtx::font_name`]) :
/// le nom du rôle ou la famille retenue ; `None` : le corps du thème.
fn font_name(family: &str, ctx: &ElementCtx<'_>) -> Option<String> {
    ctx.base.font_name(family).map(|n| match n {
        FontName::Role(role) => role.to_owned(),
        FontName::Family(f) => f.to_owned(),
    })
}

/// La pastille (rang dès 1 dans la palette) de la couleur `c`, si c'est un accent du thème.
fn pastille(c: &Color, ctx: &ElementCtx<'_>) -> Option<u8> {
    let colors = ctx.base.colors;
    let token = colors.write(c);
    let rank = colors.palette().iter().position(|t| *t == token)?;
    u8::try_from(rank + 1).ok()
}

/// Même forme : les runs se fusionnent.
fn same_format(a: &Run, b: &Run) -> bool {
    a.bold == b.bold
        && a.italic == b.italic
        && a.color == b.color
        && a.size == b.size
        && a.link == b.link
        && a.highlight == b.highlight
}

/// `Style.align` d'un alignement source (justifié : à gauche, perte).
fn align_name(a: Align) -> &'static str {
    match a {
        Align::Left | Align::Justify => "left",
        Align::Center => "center",
        Align::Right => "right",
    }
}

/// `Style.valign` d'un ancrage source.
fn valign_name(a: Anchor) -> &'static str {
    match a {
        Anchor::Top => "top",
        Anchor::Middle => "middle",
        Anchor::Bottom => "bottom",
    }
}

#[cfg(test)]
#[path = "element_text_tests.rs"]
mod element_text_tests;
