//! Regroupement des items d'une slide en unités de contenu, avant le choix
//! du modèle.
//!
//! Une diapositive dessinée à la main empile des formes et des boîtes de
//! texte : un fond arrondi, un filet d'accent, un titre et un paragraphe
//! posés dessus forment une carte. Deux règles, dans l'ordre :
//!
//! 1. **Cartes.** Une forme remplie sans texte qui contient au moins 90 % de
//!    la surface d'une ou plusieurs boîtes de texte devient une carte (la plus
//!    petite forme qui la contient, si plusieurs). Les formes sans texte
//!    posées dans la carte (filets, barres d'accent) y sont absorbées.
//! 2. **Textes fusionnés.** Les boîtes de texte restantes, prises en ordre de
//!    lecture, dont les cellules accrochées se chevauchent et qui se suivent,
//!    deviennent un seul texte (paragraphes mis bout à bout).
//!
//! Chaque autre item reste une unité à lui seul.

use oku_slides_core::Cell;

use crate::source::{Fill, Frame, Item, ItemKind};

/// Part minimale de la surface d'une boîte de texte couverte par une forme pour qu'elle y soit posée.
const INSIDE: f64 = 0.9;

/// Une unité de contenu : ce qui deviendra un élément.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Unit<'a> {
    /// Un item seul.
    Item(&'a Item),
    /// Une carte : sa forme, ses textes (ordre de lecture), ses formes absorbées.
    Card {
        /// La forme remplie.
        shape: &'a Item,
        /// Les boîtes de texte posées dessus, en ordre de lecture.
        texts: Vec<&'a Item>,
        /// Les formes sans texte posées dessus.
        inner: Vec<&'a Item>,
    },
    /// Des boîtes de texte fusionnées, en ordre de lecture (au moins deux).
    Text(Vec<&'a Item>),
}

impl<'a> Unit<'a> {
    /// La boîte de l'unité : celle de la forme d'une carte, l'englobante de textes fusionnés.
    pub(crate) fn frame(&self) -> Frame {
        match self {
            Unit::Item(i) | Unit::Card { shape: i, .. } => i.frame,
            Unit::Text(items) => bounds(items.iter().map(|i| &i.frame)),
        }
    }

    /// Les items source de l'unité.
    pub(crate) fn items(&self) -> Vec<&'a Item> {
        match self {
            Unit::Item(i) => vec![i],
            Unit::Card { shape, texts, inner } => {
                [*shape].into_iter().chain(texts.iter().copied()).chain(inner.iter().copied()).collect()
            }
            Unit::Text(items) => items.clone(),
        }
    }

    /// L'item qui nomme l'unité : la forme d'une carte, le premier texte fusionné.
    pub(crate) fn lead(&self) -> &'a Item {
        match self {
            Unit::Item(i) | Unit::Card { shape: i, .. } => i,
            Unit::Text(items) => items[0],
        }
    }
}

/// La boîte englobante de `frames` (vide : boîte nulle à l'origine).
pub(crate) fn bounds<'f>(frames: impl Iterator<Item = &'f Frame>) -> Frame {
    let mut acc: Option<(f64, f64, f64, f64)> = None;
    for f in frames {
        let (x0, y0, x1, y1) = acc.unwrap_or((f.x, f.y, f.x + f.w, f.y + f.h));
        acc = Some((x0.min(f.x), y0.min(f.y), x1.max(f.x + f.w), y1.max(f.y + f.h)));
    }
    let (x0, y0, x1, y1) = acc.unwrap_or_default();
    Frame::new(x0, y0, x1 - x0, y1 - y0)
}

/// Surface de l'intersection de deux boîtes.
fn overlap(a: &Frame, b: &Frame) -> f64 {
    let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    w.max(0.0) * h.max(0.0)
}

/// `inner` est-elle posée sur `outer` ? Au moins 90 % de sa surface dedans ;
/// un filet sans surface, si ses deux extrémités y sont.
fn inside(inner: &Frame, outer: &Frame) -> bool {
    let area = inner.w * inner.h;
    if area > 0.0 {
        return overlap(inner, outer) >= INSIDE * area;
    }
    let within = |x: f64, y: f64| x >= outer.x && x <= outer.x + outer.w && y >= outer.y && y <= outer.y + outer.h;
    within(inner.x, inner.y) && within(inner.x + inner.w, inner.y + inner.h)
}

/// Une forme remplie sans texte : un fond possible de carte.
fn is_ground(item: &Item) -> bool {
    matches!(&item.kind, ItemKind::Shape(s) if !item.has_text() && matches!(s.fill, Fill::Solid { .. } | Fill::Gradient { .. }))
}

/// Une forme sans texte (fond, filet, décor).
pub(crate) fn is_bare_shape(item: &Item) -> bool {
    matches!(item.kind, ItemKind::Shape(_)) && !item.has_text()
}

/// Une boîte de texte (pas une forme) qui porte du texte.
fn is_text(item: &Item) -> bool {
    matches!(item.kind, ItemKind::Text(_)) && item.has_text()
}

/// Ordre de lecture : de haut en bas, puis de gauche à droite.
pub(crate) fn reading(a: &Frame, b: &Frame) -> std::cmp::Ordering {
    a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x))
}

/// Deux cellules se chevauchent-elles ?
pub(crate) fn cells_meet(a: &Cell, b: &Cell) -> bool {
    a.col < b.col + b.cols && b.col < a.col + a.cols && a.row < b.row + b.rows && b.row < a.row + a.rows
}

/// Les unités de `items` (ordre de dessin), cartes puis textes fusionnés ;
/// `cell` accroche une boîte source à la grille de référence de la slide.
///
/// Les unités suivent l'ordre de dessin de leur premier item.
pub(crate) fn units<'a>(items: &[&'a Item], cell: impl Fn(&Frame) -> Cell) -> Vec<Unit<'a>> {
    let area = |i: &Item| i.frame.w * i.frame.h;
    // Chaque texte va sur la plus petite forme qui le porte.
    let ground = |t: &Item| {
        items
            .iter()
            .enumerate()
            .filter(|(_, g)| is_ground(g) && inside(&t.frame, &g.frame))
            .min_by(|(_, a), (_, b)| area(a).total_cmp(&area(b)))
            .map(|(k, _)| k)
    };
    let mut owner: Vec<Option<usize>> = items.iter().map(|i| if is_text(i) { ground(i) } else { None }).collect();
    let cards: Vec<usize> = (0..items.len()).filter(|k| owner.contains(&Some(*k))).collect();
    for (k, i) in items.iter().enumerate() {
        if owner[k].is_none() && !cards.contains(&k) && is_bare_shape(i) {
            owner[k] = cards
                .iter()
                .filter(|c| inside(&i.frame, &items[**c].frame))
                .min_by(|a, b| area(items[**a]).total_cmp(&area(items[**b])))
                .copied();
        }
    }
    // Textes restants, en ordre de lecture : fusion des voisins dont les cellules se chevauchent.
    let mut loose: Vec<usize> = (0..items.len()).filter(|k| owner[*k].is_none() && is_text(items[*k])).collect();
    loose.sort_by(|a, b| reading(&items[*a].frame, &items[*b].frame));
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for k in loose {
        match runs.last_mut() {
            Some(run) if run.last().is_some_and(|p| cells_meet(&cell(&items[*p].frame), &cell(&items[k].frame))) => {
                run.push(k);
            }
            _ => runs.push(vec![k]),
        }
    }
    let mut out = Vec::new();
    for (k, item) in items.iter().enumerate() {
        if cards.contains(&k) {
            let mut texts: Vec<&Item> =
                (0..items.len()).filter(|t| owner[*t] == Some(k) && is_text(items[*t])).map(|t| items[t]).collect();
            texts.sort_by(|a, b| reading(&a.frame, &b.frame));
            let inner =
                (0..items.len()).filter(|t| owner[*t] == Some(k) && !is_text(items[*t])).map(|t| items[t]).collect();
            out.push(Unit::Card { shape: item, texts, inner });
        } else if owner[k].is_some() {
            continue;
        } else if let Some(run) = runs.iter().find(|r| r.iter().min() == Some(&k)) {
            out.push(match run.as_slice() {
                [single] => Unit::Item(items[*single]),
                _ => Unit::Text(run.iter().map(|t| items[*t]).collect()),
            });
        } else if !runs.iter().any(|r| r.contains(&k)) {
            out.push(Unit::Item(item));
        }
    }
    out
}

#[cfg(test)]
#[path = "cluster_tests.rs"]
mod cluster_tests;
