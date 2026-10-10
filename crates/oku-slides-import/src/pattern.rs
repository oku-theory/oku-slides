//! Signature d'un motif : ce qui fait que deux groupes d'items se ressemblent.
//!
//! Deux groupes ont la même signature s'ils ont les mêmes sortes d'items en
//! même nombre, disposés de la même façon une fois ramenés à leur boîte
//! englobante. Elle ne dépend ni de la position, ni de l'échelle, ni de
//! l'ordre de dessin : elle regroupe les occurrences d'un même motif (une
//! frise de trois étapes répétée sur plusieurs slides) en une seule demande
//! de widget.
//!
//! Limite assumée : chaque bord est arrondi au quart le plus proche ; un bord
//! posé près d'une frontière d'arrondi (un huitième : 1/8, 3/8…) bascule d'un
//! côté ou de l'autre pour un écart de quelques pt. Deux occurrences presque
//! identiques peuvent donc donner deux signatures, et deux demandes au lieu
//! d'une : une demande en trop, jamais un rendu faux.

use std::collections::BTreeMap;

use crate::source::{Frame, Item, ItemKind};

/// Pas de la grille de disposition : chaque bord est arrondi au quart.
const STEPS: f64 = 4.0;

/// Nom d'une sorte d'item.
fn kind(item: &Item) -> &'static str {
    match item.kind {
        ItemKind::Text(_) => "text",
        ItemKind::Shape(_) => "shape",
        ItemKind::Picture(_) => "picture",
        ItemKind::Table(_) => "table",
        ItemKind::Chart(_) => "chart",
        ItemKind::Group { .. } => "group",
        ItemKind::Opaque(_) => "opaque",
    }
}

/// `v` ramené à `[0, 1]` sur `[lo, lo + len]`, arrondi au pas `1 / STEPS`.
fn step(v: f64, lo: f64, len: f64) -> u8 {
    if len <= f64::EPSILON {
        return 0;
    }
    // Borné à [0, STEPS] : la conversion ne déborde pas.
    ((v - lo) / len * STEPS).round().clamp(0.0, STEPS) as u8
}

/// La signature de `items` : `<sorte>×<nombre>` triés par sorte, puis `|`,
/// puis la boîte de chaque item ramenée à la boîte englobante, arrondie au
/// quart (`x0 y0 x1 y1`, chiffres de 0 à 4), triée.
///
/// Exemple : trois formes de 200 × 100 à x = 0, 300 et 600 donnent
/// `shape×3|0014,2034,3044` (boîte englobante de 800 de large ; `300 / 800 × 4`
/// = 1,5 s'arrondit à 2).
pub fn signature(items: &[Item]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for i in items {
        *counts.entry(kind(i)).or_default() += 1;
    }
    let kinds: Vec<String> = counts.iter().map(|(k, n)| format!("{k}×{n}")).collect();
    let frames: Vec<&Frame> = items.iter().map(|i| &i.frame).collect();
    let min = |f: fn(&Frame) -> f64| frames.iter().map(|x| f(x)).fold(f64::INFINITY, f64::min);
    let max = |f: fn(&Frame) -> f64| frames.iter().map(|x| f(x)).fold(f64::NEG_INFINITY, f64::max);
    let (x, y) = (min(|f| f.x), min(|f| f.y));
    let (w, h) = (max(|f| f.x + f.w) - x, max(|f| f.y + f.h) - y);
    let mut boxes: Vec<String> = frames
        .iter()
        .map(|f| format!("{}{}{}{}", step(f.x, x, w), step(f.y, y, h), step(f.x + f.w, x, w), step(f.y + f.h, y, h)))
        .collect();
    boxes.sort();
    format!("{}|{}", kinds.join(","), boxes.join(","))
}

#[cfg(test)]
#[path = "pattern_tests.rs"]
mod pattern_tests;
