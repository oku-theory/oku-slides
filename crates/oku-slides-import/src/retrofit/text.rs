//! Texte source → texte riche ou plain du cœur, dans ses bornes.
//!
//! Le style vient du thème cible : seuls passent le gras, l'italique, les
//! liens sûrs, les listes (un niveau), les capitales (le texte est mis en
//! majuscules) et la couleur des runs, quand son jeton est lisible sur le fond
//! du texte ([`text_color`]). Le reste est perdu et dit au rapport.

use oku_slides_core::{ListKind, PLAIN_MAX, Paragraph, RICH_BYTES_MAX, RICH_PARAGRAPHS_MAX, Run};

use super::color::{ColorTokens, text_color};
use crate::report::{LossKind, RunEffect};
use crate::source::{self, Bullet, Caps, Rgba, TextBox};

/// Jeton de l'encre du texte : la couleur par défaut, jamais émise.
const INK: &str = "ink";

/// Texte riche projeté et ses pertes (chacune une fois).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TextOut {
    /// Paragraphes, dans les bornes `RICH_PARAGRAPHS_MAX` et `RICH_BYTES_MAX`.
    pub paragraphs: Vec<Paragraph>,
    /// Pertes.
    pub losses: Vec<LossKind>,
}

/// Ajoute `loss` à `losses` s'il n'y est pas (une perte, ou une perte sur un item).
pub(crate) fn note<T: PartialEq>(losses: &mut Vec<T>, loss: T) {
    if !losses.contains(&loss) {
        losses.push(loss);
    }
}

/// Les effets d'un run que le texte du cœur n'exprime pas.
pub(crate) fn effects(style: &source::RunStyle) -> impl Iterator<Item = RunEffect> {
    [
        (style.underline, RunEffect::Underline),
        (style.strike, RunEffect::Strike),
        (style.baseline != 0, RunEffect::Baseline),
        (style.caps == Caps::Small, RunEffect::Caps),
        (style.highlight.is_some(), RunEffect::Highlight),
    ]
    .into_iter()
    .filter_map(|(lost, e)| lost.then_some(e))
}

/// Le texte d'un run tel qu'il s'affiche : en majuscules pour `Caps::All`.
pub(crate) fn shown(r: &source::Run) -> String {
    match r.style.caps {
        Caps::All => r.text.to_uppercase(),
        Caps::None | Caps::Small => r.text.clone(),
    }
}

/// Même forme : les runs se fusionnent.
fn same_format(a: &Run, b: &Run) -> bool {
    a.bold == b.bold && a.italic == b.italic && a.color == b.color && a.link == b.link
}

/// Lien gardé : `http`, `https` ou `mailto`, quelle que soit la casse.
pub(crate) fn safe_link(link: Option<&str>) -> Option<String> {
    let link = link?;
    let lower = link.to_ascii_lowercase();
    ["http://", "https://", "mailto:"].iter().any(|s| lower.starts_with(s)).then(|| link.to_owned())
}

/// Liste du cœur pour une puce source.
pub(crate) fn list_kind(bullet: &Bullet) -> Option<ListKind> {
    match bullet {
        Bullet::None => None,
        Bullet::Char { .. } => Some(ListKind::Bullet),
        Bullet::AutoNum { .. } => Some(ListKind::Number),
    }
}

/// Taille de `p` en JSON, comme la mesure `text_fits`.
fn json_len(p: &Paragraph) -> usize {
    serde_json::to_vec(p).map_or(usize::MAX, |b| b.len())
}

/// Le plus long début de `p` dont la forme JSON tient en `budget` octets ;
/// `None` si même un caractère ne tient pas.
fn cut(p: &Paragraph, budget: usize) -> Option<Paragraph> {
    let mut out = Paragraph { runs: Vec::new(), list: p.list };
    for run in &p.runs {
        out.runs.push(run.clone());
        if json_len(&out) <= budget {
            continue;
        }
        // Coupe du dernier run à une frontière de caractère, par dichotomie.
        let bounds: Vec<usize> = run.t.char_indices().map(|(i, _)| i).skip(1).chain([run.t.len()]).collect();
        let (mut lo, mut hi) = (0usize, bounds.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if let Some(last) = out.runs.last_mut() {
                run.t[..bounds[mid - 1]].clone_into(&mut last.t);
            }
            if json_len(&out) <= budget { lo = mid } else { hi = mid - 1 }
        }
        if lo == 0 {
            out.runs.pop();
        } else if let Some(last) = out.runs.last_mut() {
            run.t[..bounds[lo - 1]].clone_into(&mut last.t);
        }
        break;
    }
    (!out.runs.is_empty() && json_len(&out) <= budget).then_some(out)
}

/// `paragraphs` bornés à `RICH_PARAGRAPHS_MAX` paragraphes et `RICH_BYTES_MAX`
/// octets de JSON (crochets et virgules compris) ; vrai si quelque chose est coupé.
pub(crate) fn bound(paragraphs: Vec<Paragraph>) -> (Vec<Paragraph>, bool) {
    let mut out = Vec::new();
    let mut used = 2; // `[` et `]`
    let total = paragraphs.len();
    for p in paragraphs {
        if out.len() == RICH_PARAGRAPHS_MAX {
            return (out, true);
        }
        let comma = usize::from(!out.is_empty());
        let budget = RICH_BYTES_MAX.saturating_sub(used + comma);
        let len = json_len(&p);
        if len <= budget {
            used += comma + len;
            out.push(p);
            continue;
        }
        if let Some(p) = cut(&p, budget) {
            out.push(p);
        }
        return (out, true);
    }
    let cut_short = out.len() < total;
    (out, cut_short)
}

/// Le texte riche des boîtes `boxes` mises bout à bout (paragraphes dans
/// l'ordre), projeté sur les couleurs `tokens`, posé sur le fond `bg`.
///
/// Un paragraphe sans texte est omis ; les runs adjacents de même forme sont
/// fusionnés ; un niveau de liste > 0 est aplati (perte `ListLevels`). La
/// couleur d'un run est émise si son jeton est lisible sur `bg`
/// ([`text_color`]) et n'est pas `ink`, l'encre par défaut.
pub(crate) fn rich<'t>(boxes: impl IntoIterator<Item = &'t TextBox>, tokens: &ColorTokens, bg: Rgba) -> TextOut {
    let mut losses = Vec::new();
    let mut paragraphs = Vec::new();
    for sp in boxes.into_iter().flat_map(|tb| &tb.paragraphs) {
        let mut out: Vec<Run> = Vec::new();
        for r in sp.runs.iter().filter(|r| !r.text.is_empty()) {
            for e in effects(&r.style) {
                note(&mut losses, LossKind::RunStyle(e));
            }
            let color = match text_color(&r.style.color, bg, tokens) {
                Ok(token) => token.filter(|t| t != INK),
                Err(loss) => {
                    note(&mut losses, loss);
                    None
                }
            };
            let run = Run {
                t: shown(r),
                bold: r.style.strong(),
                italic: r.style.italic,
                color,
                link: safe_link(r.style.link.as_deref()),
                ..Run::default()
            };
            match out.last_mut() {
                Some(last) if same_format(last, &run) => last.t.push_str(&run.t),
                _ => out.push(run),
            }
        }
        if out.is_empty() {
            continue;
        }
        if sp.level > 0 {
            note(&mut losses, LossKind::ListLevels);
        }
        paragraphs.push(Paragraph { runs: out, list: list_kind(&sp.style.bullet) });
    }
    let (paragraphs, truncated) = bound(paragraphs);
    if truncated {
        note(&mut losses, LossKind::TextTruncated);
    }
    TextOut { paragraphs, losses }
}

/// Le texte plain de `tb` (titre, sous-titre) : paragraphes et sauts de ligne
/// ramenés à une espace, espaces fusionnées, au plus `PLAIN_MAX` octets coupés
/// à une frontière de caractère, capitales appliquées ; pertes : effets de texte, coupe.
pub(crate) fn plain(tb: &TextBox) -> (String, Vec<LossKind>) {
    let mut losses = Vec::new();
    let mut words = Vec::new();
    for sp in &tb.paragraphs {
        let mut text = String::new();
        for r in sp.runs.iter().filter(|r| !r.text.is_empty()) {
            for e in effects(&r.style) {
                note(&mut losses, LossKind::RunStyle(e));
            }
            text.push_str(&shown(r));
        }
        words.extend(text.split_whitespace().map(str::to_owned));
    }
    let mut s = words.join(" ");
    if s.len() > PLAIN_MAX {
        let end = (0..=PLAIN_MAX).rev().find(|i| s.is_char_boundary(*i)).unwrap_or(0);
        s.truncate(end);
        s.truncate(s.trim_end().len());
        note(&mut losses, LossKind::TextTruncated);
    }
    (s, losses)
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
