//! Signature d'un motif : ce qui fait que deux groupes d'items se ressemblent.
//!
//! Deux groupes ont la même signature s'ils ont les mêmes sortes d'items en
//! même nombre, disposés de la même façon une fois ramenés à leur boîte
//! englobante. Elle ne dépend ni de la position, ni de l'échelle, ni de
//! l'ordre de dessin : elle regroupe les occurrences d'un même motif (une
//! frise de trois étapes répétée sur plusieurs slides) en une seule demande
//! de widget.
//!
//! Un trait fléché (`headEnd`, `tailEnd`) ajoute à sa boîte le sens de ses
//! pointes : deux frises dont les flèches vont en sens contraires diffèrent.
//!
//! Limite assumée : chaque bord est arrondi au quart le plus proche ; un bord
//! posé près d'une frontière d'arrondi (un huitième : 1/8, 3/8…) bascule d'un
//! côté ou de l'autre pour un écart de quelques pt. Deux occurrences presque
//! identiques peuvent donc donner deux signatures, et deux demandes au lieu
//! d'une : une demande en trop, jamais un rendu faux.

use std::collections::BTreeMap;

use crate::source::{Frame, Item, ItemKind};
use oku_slides_core::num::byte;

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
        ItemKind::SmartArt(_) => "smartart",
        ItemKind::Opaque(_) => "opaque",
    }
}

/// Sens des pointes d'un trait fléché, `""` sans pointe : `>` puis la boussole
/// (`e`, `sw`…) de chaque pointe, celle du début (`headEnd`) d'abord.
///
/// Le tracé va du coin de départ (selon les miroirs) au coin opposé ; une
/// pointe de fin suit ce sens, une pointe de début le sens contraire.
fn arrows(item: &Item) -> String {
    let ItemKind::Shape(s) = &item.kind else { return String::new() };
    let Some(stroke) = &s.stroke else { return String::new() };
    let f = &item.frame;
    let sign = |len: f64, flip: bool| {
        if len <= f64::EPSILON {
            0
        } else if flip {
            -1
        } else {
            1
        }
    };
    let (dx, dy) = (sign(f.w, f.flip_h), sign(f.h, f.flip_v));
    let compass = |dx: i8, dy: i8| {
        let ns = match dy {
            -1 => "n",
            1 => "s",
            _ => "",
        };
        let ew = match dx {
            -1 => "w",
            1 => "e",
            _ => "",
        };
        format!("{ns}{ew}")
    };
    let mut out = String::new();
    if stroke.head.is_some() {
        out.push_str(&compass(-dx, -dy));
    }
    if stroke.tail.is_some() {
        if !out.is_empty() {
            out.push('+');
        }
        out.push_str(&compass(dx, dy));
    }
    if out.is_empty() { out } else { format!(">{out}") }
}

/// `v` ramené à `[0, 1]` sur `[lo, lo + len]`, arrondi au pas `1 / STEPS`.
fn step(v: f64, lo: f64, len: f64) -> u8 {
    if len <= f64::EPSILON {
        return 0;
    }
    // Clamped to [0, STEPS], below 255: the conversion does not saturate.
    byte(((v - lo) / len * STEPS).clamp(0.0, STEPS))
}

/// La signature de `items` : `<sorte>×<nombre>` triés par sorte, puis `|`,
/// puis la boîte de chaque item ramenée à la boîte englobante, arrondie au
/// quart (`x0 y0 x1 y1`, chiffres de 0 à 4), suivie du sens de ses pointes
/// s'il en a (`0040>e`), triée.
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
        .zip(items)
        .map(|(f, i)| {
            let (x0, y0, x1, y1) = (step(f.x, x, w), step(f.y, y, h), step(f.x + f.w, x, w), step(f.y + f.h, y, h));
            format!("{x0}{y0}{x1}{y1}{}", arrows(i))
        })
        .collect();
    boxes.sort();
    format!("{}|{}", kinds.join(","), boxes.join(","))
}

#[cfg(test)]
#[path = "pattern_tests.rs"]
mod pattern_tests;
