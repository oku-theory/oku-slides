//! Repli d'un groupe (ou d'un SmartArt dessiné) : une pile de cartes et
//! d'éléments.
//!
//! Les groupes imbriqués sont aplatis ; une feuille avec texte devient une
//! carte (premier paragraphe en `title` s'il y en a plusieurs, le reste en
//! `body`) ; une image, un tableau, un graphique gardent leur élément ; une
//! forme sans texte (flèche, connecteur, décor) est omise. La pile est en
//! colonnes si les centres des enfants s'étalent plus en largeur qu'en
//! hauteur, sinon en lignes.
//!
//! Le repli n'est dit (demande de widget, décision 5 de la spec) que s'il perd
//! quelque chose : une forme omise, un motif régulier d'au moins trois
//! feuilles (une rangée ou une colonne de boîtes semblables, également
//! espacées) dont la pile ne garde pas la géométrie, ou un SmartArt.

use oku_slides_core::{Element, ParamValue};

use super::content::{ContentCx, Ground, body, card, leaf, plain_of, transform};
use crate::report::Loss;
use crate::source::{Frame, Item, ItemKind, TextBox};

/// Repli des groupes : une pile.
pub(crate) const STACK: &str = "stack";
/// Écart relatif toléré entre des tailles ou des pas « semblables ».
const SIMILAR: f64 = 0.75;

/// Les feuilles d'un groupe, groupes imbriqués aplatis, dans l'ordre de dessin.
/// Un groupe imbriqué retourné ou tourné porte une perte `Transform` ; `smartart`
/// devient vrai si l'un d'eux est un SmartArt.
fn leaves<'a>(items: &'a [Item], out: &mut Vec<&'a Item>, smartart: &mut bool, losses: &mut Vec<Loss>) {
    for i in items {
        match &i.kind {
            ItemKind::Group { items, smartart: nested } => {
                transform(i, losses);
                *smartart |= *nested;
                leaves(items, out, smartart, losses);
            }
            _ => out.push(i),
        }
    }
}

/// La carte d'une feuille de groupe avec texte : le premier paragraphe en
/// titre s'il y en a plusieurs, le reste en corps.
fn leaf_card(id: String, item: &Item, tb: &TextBox, cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Element {
    transform(item, losses);
    let filled: Vec<&crate::source::Paragraph> =
        tb.paragraphs.iter().filter(|p| p.runs.iter().any(|r| !r.text.trim().is_empty())).collect();
    let (title, rest) = match filled.as_slice() {
        [first, rest @ ..] if !rest.is_empty() => {
            let head = TextBox { paragraphs: vec![(*first).clone()], ..tb.clone() };
            let rest = TextBox { paragraphs: rest.iter().map(|p| (*p).clone()).collect(), ..tb.clone() };
            (Some(plain_of(&head, &item.id, losses)), rest)
        }
        _ => (None, tb.clone()),
    };
    card(id, None, title, body(&[&rest], Ground::Card(false), cx, &item.id, losses), false)
}

/// Le plus petit sur le plus grand de `a` et `b` (1 si tous deux nuls).
fn ratio(a: f64, b: f64) -> f64 {
    if a.max(b) <= 0.0 { 1.0 } else { a.min(b) / a.max(b) }
}

/// L'étendue (max - min) de `values`.
fn spread(values: impl Iterator<Item = f64>) -> f64 {
    let (lo, hi) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
    hi - lo
}

/// Toutes les valeurs de `values` sont-elles semblables (à 25 % près) ?
fn similar(values: impl Iterator<Item = f64> + Clone) -> bool {
    let lo = values.clone().fold(f64::INFINITY, f64::min);
    let hi = values.fold(0.0, f64::max);
    ratio(lo, hi) >= SIMILAR
}

/// Un motif régulier : au moins trois boîtes de tailles semblables, centrées
/// sur une même rangée (ou colonne) à un quart de leur côté près, et également
/// espacées (pas semblables).
pub(crate) fn regular(frames: &[Frame]) -> bool {
    if frames.len() < 3 || !similar(frames.iter().map(|f| f.w)) || !similar(frames.iter().map(|f| f.h)) {
        return false;
    }
    let centers: Vec<(f64, f64)> = frames.iter().map(|f| (f.x + f.w / 2.0, f.y + f.h / 2.0)).collect();
    let spread_of = |f: fn(&(f64, f64)) -> f64| spread(centers.iter().map(f));
    let row = spread_of(|c| c.0) >= spread_of(|c| c.1);
    let side = frames.iter().map(|f| if row { f.h } else { f.w }).fold(f64::INFINITY, f64::min);
    if spread_of(if row { |c: &(f64, f64)| c.1 } else { |c: &(f64, f64)| c.0 }) > 0.25 * side {
        return false;
    }
    let mut along: Vec<f64> = centers.iter().map(|c| if row { c.0 } else { c.1 }).collect();
    along.sort_by(f64::total_cmp);
    similar(along.windows(2).map(|w| w[1] - w[0]))
}

/// Le repli d'un groupe aux enfants `items` (SmartArt dessiné si `smartart`)
/// et sa sorte de repli, si elle est à dire.
pub(crate) fn group(
    items: &[Item],
    smartart: bool,
    id: &str,
    cx: &ContentCx<'_>,
    losses: &mut Vec<Loss>,
) -> (Option<Element>, Option<String>) {
    let mut flat = Vec::new();
    let mut smartart = smartart;
    leaves(items, &mut flat, &mut smartart, losses);
    let mut children = Vec::new();
    let mut frames = Vec::new();
    let mut omitted = false;
    for item in &flat {
        let child_id = format!("{id}-{}", children.len() + 1);
        let child = match item.text() {
            Some(tb) if item.has_text() => Some(leaf_card(child_id, item, tb, cx, losses)),
            _ => match &item.kind {
                ItemKind::Shape(_) => {
                    omitted = true;
                    None
                }
                _ => leaf(item, child_id, cx, losses),
            },
        };
        if let Some(c) = child {
            children.push(c);
            frames.push(item.frame);
        }
    }
    if children.is_empty() {
        return (None, None);
    }
    let spread_of = |f: fn(&Frame) -> f64| spread(frames.iter().map(f));
    let layout = if spread_of(|f| f.x + f.w / 2.0) > spread_of(|f| f.y + f.h / 2.0) { "columns" } else { "rows" };
    let mut e = Element::new(id, STACK);
    e.params.set("layout", ParamValue::Text(layout.to_owned()));
    e.params.set("children", ParamValue::Slot(children));
    let lossy = omitted || smartart || regular(&frames);
    (Some(e), lossy.then(|| STACK.to_owned()))
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod group_tests;
