//! Pose des unités d'une slide en éléments : blocs d'un flux en ordre de
//! lecture, cadres ajustés dans la zone de corps (accrochés à la grille si
//! rien ne s'y recouvre), textes d'une couverture sans slot mis dans les notes.
//! [`arrange`] le fait pour un placement donné : le projecteur peut essayer le
//! flux, puis la grille si le flux déborde de sa zone ([`super::measure`]).

use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::kinds::base;
use oku_slides_core::{Cell, Element, Grid, ParamValue, Slide, WidgetRegistry};

use super::cluster::{Unit, cells_meet, is_bare_shape, reading};
use super::content::{ContentCx, effect_losses, element};
use super::measure::Measure;
use super::profile::Shape;
use super::text::{note, plain};
use super::zone::{Fit, PageTokens, Zone, body_zone, cell_rect, grid_gap, snap, to_zone, track_size};
use crate::report::{Loss, LossKind, Placement};
use crate::source::{Fill, Item, ItemKind};

/// Les assets des éléments `list` et de leurs descendants.
pub(crate) fn collect_assets(list: &[Element], out: &mut BTreeSet<String>) {
    for e in list {
        if e.kind == base::IMAGE
            && let Some(a) = e.params.text("asset")
        {
            out.insert(a.to_owned());
        }
        for (_, children) in e.params.slots() {
            collect_assets(children, out);
        }
    }
}

/// Largeur d'une colonne et hauteur d'une ligne de `grid` dans `zone`, ramenées en pt source par `scale`.
pub(crate) fn track(zone: &Zone, grid: &Grid, gap: f64, scale: f64) -> (f64, f64) {
    let (w, h) = track_size(zone, grid, gap);
    if scale > 0.0 { (w / scale, h / scale) } else { (0.0, 0.0) }
}

/// Les blocs d'un flux : les unités en ordre de lecture, la rangée de colonnes
/// (s'il faut une pile) à la place de sa première unité ; jamais une forme sans texte.
pub(crate) fn flow_order<'u, 'a>(units: &'u [Unit<'a>], shape: &Shape, stack: bool) -> Vec<Vec<&'u Unit<'a>>> {
    let mut order: Vec<usize> = (0..units.len()).collect();
    if shape.columns >= 2 {
        // Une rangée seule se lit de gauche à droite.
        order.sort_by(|a, b| {
            let (fa, fb) = (units[*a].frame(), units[*b].frame());
            if shape.row.contains(a) && shape.row.contains(b) { fa.x.total_cmp(&fb.x) } else { reading(&fa, &fb) }
        });
    } else {
        order.sort_by(|a, b| reading(&units[*a].frame(), &units[*b].frame()));
    }
    let mut blocks: Vec<Vec<&'u Unit<'a>>> = Vec::new();
    let mut row_at: Option<usize> = None;
    for k in order {
        let u = &units[k];
        if matches!(u, Unit::Item(i) if is_bare_shape(i)) {
            continue;
        }
        if stack && shape.row.contains(&k) {
            match row_at {
                Some(at) => blocks[at].push(u),
                None => {
                    row_at = Some(blocks.len());
                    blocks.push(vec![u]);
                }
            }
        } else {
            blocks.push(vec![u]);
        }
    }
    blocks
}

/// Accroche les éléments placés en cadres à la grille `grid` : chacun prend la
/// cellule la plus proche de son cadre ([`snap`]), si aucun contenu (forme
/// exceptée) n'y recouvre un autre ; sinon la grille ne sait pas tenir
/// l'arrangement et tous gardent leur cadre.
pub(crate) fn snap_all(elements: &mut [Element], zone: &Zone, grid: &Grid, gap: f64) {
    let cells: Vec<Option<Cell>> = elements.iter().map(|e| e.frame.map(|f| snap(zone, grid, gap, f))).collect();
    let solid: Vec<(usize, Cell)> = cells
        .iter()
        .enumerate()
        .filter(|(k, _)| elements[*k].kind != base::SHAPE)
        .filter_map(|(k, c)| c.map(|c| (k, c)))
        .collect();
    let disjoint = solid.iter().enumerate().all(|(i, (_, a))| solid[i + 1..].iter().all(|(_, b)| !cells_meet(a, b)));
    if disjoint {
        for (e, c) in elements.iter_mut().zip(cells) {
            if c.is_some() {
                e.frame = None;
                e.cell = c;
            }
        }
    }
}

/// Met les textes de `unit` (groupes compris) dans `moved`, un paragraphe par
/// boîte, avec la perte `MovedToNotes` ; ses items sans texte n'ont pas de
/// place (`NoSlot`).
pub(crate) fn move_to_notes(unit: &Unit<'_>, moved: &mut Vec<String>, losses: &mut Vec<Loss>) {
    fn walk(item: &Item, moved: &mut Vec<String>, losses: &mut Vec<Loss>) {
        if let crate::source::ItemKind::Group { items, .. } = &item.kind {
            for child in items {
                walk(child, moved, losses);
            }
            return;
        }
        match item.text().filter(|_| item.has_text()) {
            Some(tb) => {
                let (text, lost) = plain(tb);
                for l in lost {
                    note(losses, Loss::on(&item.id, l));
                }
                moved.push(text);
                note(losses, Loss::on(&item.id, LossKind::MovedToNotes));
            }
            None => note(losses, Loss::on(&item.id, LossKind::NoSlot)),
        }
    }
    for item in unit.items() {
        walk(item, moved, losses);
    }
}

/// Le rapport hauteur / largeur de chaque image des items `items` (groupes
/// compris), par asset : image, forme remplie d'une image, repli d'un intraduisible.
pub(crate) fn aspects(items: &[&Item]) -> BTreeMap<String, f64> {
    fn walk(item: &Item, out: &mut BTreeMap<String, f64>) {
        let asset = match &item.kind {
            ItemKind::Picture(p) => Some(&p.asset),
            ItemKind::Shape(s) => match &s.fill {
                Fill::Picture { asset } => Some(asset),
                _ => None,
            },
            ItemKind::Opaque(o) => o.fallback.as_ref(),
            ItemKind::Group { items, .. } => {
                for child in items {
                    walk(child, out);
                }
                None
            }
            _ => None,
        };
        let f = &item.frame;
        if let Some(a) = asset
            && f.w > 0.0
            && f.h > 0.0
        {
            out.entry(a.clone()).or_insert(f.h / f.w);
        }
    }
    let mut out = BTreeMap::new();
    for item in items {
        walk(item, &mut out);
    }
    out
}

/// Ce que [`arrange`] pose sur une slide.
#[derive(Debug, Clone, Default)]
pub(crate) struct Built<'a> {
    /// Les éléments du slot cible.
    pub elements: Vec<Element>,
    /// Pertes des unités, chacune une fois.
    pub losses: Vec<Loss>,
    /// Les items source rendus par un repli, avec la sorte du repli.
    pub fallbacks: Vec<(&'a Item, String)>,
    /// Les sortes de repli employées, sans doublon.
    pub kinds: Vec<String>,
    /// L'élément projeté depuis chaque item source.
    pub origins: Vec<(String, String)>,
    /// Les textes mis dans les notes (couverture sans slot).
    pub moved: Vec<String>,
    /// En grille : chaque contenu tient dans son cadre (estimé).
    pub fits: bool,
}

/// Ce dont [`arrange`] a besoin, hors du placement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Arrange<'s, 'u, 'a> {
    /// Id de la slide Oku.
    pub id: &'s str,
    /// Les unités, dans l'ordre de dessin.
    pub units: &'u [Unit<'a>],
    /// La forme de la slide.
    pub shape: &'s Shape,
    /// La rangée de colonnes devient une pile.
    pub stack: bool,
    /// Les unités vont dans les notes (couverture sans slot).
    pub to_notes: bool,
    /// Le profil n'a pas de grille : cadres libres, jamais accrochés.
    pub free: bool,
    /// Échelle des pt source vers la page Oku.
    pub scale: f64,
    /// Jetons de page.
    pub page: &'s PageTokens,
    /// Registre du pack.
    pub registry: &'s WidgetRegistry,
    /// Couleurs et assets.
    pub content: ContentCx<'s>,
    /// Items dont le lecteur a ignoré des effets.
    pub effects: &'s BTreeSet<String>,
    /// Estimation des hauteurs, pour dire si une grille tient.
    pub measure: Measure<'s>,
}

/// Les éléments de la slide `slide` (titre et grille déjà posés) pour le
/// placement `placement`.
pub(crate) fn arrange<'a>(placement: Placement, slide: &Slide, a: &Arrange<'_, '_, 'a>) -> Built<'a> {
    let grid = slide.resolved_grid(a.registry);
    let zone = body_zone(a.page, slide.params.text("title").is_some());
    let gap = grid_gap(&grid, a.page);
    let fit = Fit::new(a.units.iter().map(|u| u.frame()).collect::<Vec<_>>().iter(), a.scale, &zone);
    let content_cx = ContentCx { track: track(&zone, &grid, gap, fit.scale), ..a.content };
    let blocks: Vec<Vec<&Unit<'a>>> = match placement {
        Placement::Flow => flow_order(a.units, a.shape, a.stack),
        // Le retrofit ne place jamais en cadres libres (hi-fi) : comme la grille.
        Placement::Grid | Placement::Free => a.units.iter().map(|u| vec![u]).collect(),
    };
    let mut out = Built { fits: true, ..Built::default() };
    for (k, block) in (1..).zip(&blocks) {
        let block_id = format!("{}-{k}", a.id);
        let mut made = Vec::new();
        for (j, unit) in (1..).zip(block) {
            for item in unit.items() {
                effect_losses(item, a.effects, &mut out.losses);
            }
            if a.to_notes {
                move_to_notes(unit, &mut out.moved, &mut out.losses);
                continue;
            }
            let unit_id = if block.len() > 1 { format!("{block_id}-{j}") } else { block_id.clone() };
            out.origins.extend(unit.items().into_iter().map(|i| (i.id.clone(), unit_id.clone())));
            let c = element(unit, unit_id, &content_cx);
            for l in c.losses {
                note(&mut out.losses, l);
            }
            if let Some(kind) = c.fallback {
                if !out.kinds.contains(&kind) {
                    out.kinds.push(kind.clone());
                }
                out.fallbacks.push((unit.lead(), kind));
            }
            let Some(mut e) = c.element else { continue };
            if placement == Placement::Grid {
                e.frame = Some(to_zone(&unit.frame(), &fit, &zone));
            }
            made.push(e);
        }
        if block.len() > 1 && !made.is_empty() {
            let mut row = Element::new(block_id, base::STACK);
            row.params.set("layout", ParamValue::Text("columns".to_owned()));
            row.params.set("children", ParamValue::Slot(made));
            out.elements.push(row);
        } else {
            out.elements.extend(made);
        }
    }
    if placement == Placement::Grid {
        if !a.free {
            snap_all(&mut out.elements, &zone, &grid, gap);
        }
        // Jugé sur les cadres finaux, accrochés : une carte remplit son cadre
        // et rogne ce qui dépasse, elle doit y tenir ; un autre contenu peut
        // déborder de son cadre, pas de la zone.
        out.fits = out.elements.iter().all(|e| {
            let Some(f) = e.frame.or_else(|| e.cell.map(|c| cell_rect(&zone, &grid, gap, &c))) else { return true };
            let need = a.measure.element(e, f.w, f.h, true);
            if e.kind == base::CARD { need <= f.h + 1.0 } else { f.y + need <= zone.h + 1.0 }
        });
    }
    out
}

#[cfg(test)]
#[path = "place_tests.rs"]
mod place_tests;
