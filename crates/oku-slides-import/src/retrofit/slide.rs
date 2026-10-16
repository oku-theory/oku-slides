//! Une slide source → les ops d'une slide Oku.
//!
//! Les items omis par règle ([`Decor`]) sont écartés ; les titres viennent
//! des placeholders, sinon de la taille et de la position des textes
//! ([`headings`]) ; le reste se regroupe en unités ([`units`] : cartes,
//! textes fusionnés), sans les fonds (une forme sans texte d'au moins la
//! moitié de la page), les formes sans texte des bandes d'en-tête et de pied,
//! ni les filets posés au-dessus du bas du titre. La forme de la slide choisit
//! un modèle ([`choose`]) ; la slide Oku prend la sorte du modèle, son
//! agencement (`layout`) ou sa grille (`grid`), jamais ses textes d'exemple.
//! Titre, surtitre et sous-titre vont dans les paramètres que le widget
//! déclare ; ceux qu'il ne déclare pas deviennent des contenus.
//!
//! Placement (décision 6) : une slide « titre + un corps » (au plus une
//! unité, ou une rangée de colonnes, seule ou entre des unités pleine
//! largeur) sur un modèle à blocs va dans le slot `flow`, en ordre de
//! lecture ; une rangée entourée d'autres unités devient une pile `stack` en
//! colonnes, et la slide garde l'agencement `default`. Une forme sans texte
//! n'entre jamais dans un flux. Sinon les unités vont dans le slot libre, dans
//! l'ordre de dessin, leur boîte englobante ajustée dans la zone de corps
//! ([`Fit`]) : posées en cadre (`frame`) sur un modèle libre, sans grille ;
//! sur un modèle à grille, accrochées à la cellule la plus proche ([`snap`])
//! si aucun contenu n'y recouvre un autre, sinon laissées en cadres (la
//! grille du modèle ne sait pas tenir l'arrangement).
//!
//! Slides de titre (décision 7) : la première couverture remplit les
//! paramètres du deck (`title`, `subtitle`) et prend un modèle de couverture
//! même si elle porte d'autres textes (ni image, ni tableau, ni graphique) ;
//! ceux que le widget ne peut pas tenir vont dans les notes de la slide
//! (perte `MovedToNotes`). Les couvertures suivantes sont choisies comme des
//! chapitres.
//!
//! Bornes : une slide garde au plus [`MAX_SLIDE_ITEMS`] items (ordre de
//! dessin, perte `ItemsDropped` pour le reste), car le regroupement et la
//! rangée de colonnes sont quadratiques. Les effets ignorés par le lecteur
//! (ombres, reflets, 3D) des items projetés sont dits (`Effects`).

use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::{Cell, Element, Grid, Op, ParamType, ParamValue, Slide, SlotMode, WidgetRegistry, WidgetSchema};
use serde_json::Value;

use super::cluster::{Unit, cells_meet, is_bare_shape, reading, units};
use super::content::{ContentCx, effect_losses, element, is_thin, plain_of};
use super::decor::{Decor, in_band, is_backdrop};
use super::heading::{Headings, headings};
use super::template::{ContentKind, ContentLayout, Profile, Shape, choose, content_items, shape};
use super::text::{note, plain};
use super::zone::{Fit, PageTokens, Zone, body_zone, grid_gap, snap, to_zone, track_size};
use crate::report::{Loss, LossKind, Placement, SlideReport};
use crate::source::{Item, Layout, LayoutKind, Size, SourceSlide};

/// Items gardés au plus par slide (après le décor omis) : le regroupement en
/// unités et la recherche de la rangée de colonnes sont quadratiques.
pub(crate) const MAX_SLIDE_ITEMS: usize = 500;

/// Ce dont la projection d'une slide a besoin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SlideCx<'a> {
    /// Registre du pack cible.
    pub registry: &'a WidgetRegistry,
    /// Jetons de page du thème cible.
    pub page: &'a PageTokens,
    /// Couleurs et assets, pour les contenus (`track` est fixé par slide).
    pub content: ContentCx<'a>,
    /// Layouts de la source.
    pub layouts: &'a [Layout],
    /// Taille de la page source, en pt.
    pub size: Size,
    /// Échelle des pt source vers la page Oku.
    pub scale: f64,
    /// Décor omis par règle.
    pub decor: &'a Decor,
    /// Items dont le lecteur a ignoré des effets (`content::effect_items`).
    pub effects: &'a BTreeSet<String>,
}

/// Une slide projetée.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Projected<'a> {
    /// Ses ops, dans l'ordre.
    pub ops: Vec<Op>,
    /// Son entrée au rapport.
    pub report: SlideReport,
    /// Les assets que ses éléments référencent.
    pub assets: BTreeSet<String>,
    /// Les items source rendus par un repli, avec la sorte du repli.
    pub fallbacks: Vec<(&'a Item, String)>,
    /// L'élément projeté depuis chaque item source (id d'item, id d'élément).
    pub origins: Vec<(String, String)>,
}

/// Une couverture : layout `title`, un sous-titre sans contenu, ou une couverture déduite.
fn is_cover(shape: &Shape, heads: &Headings<'_>) -> bool {
    shape.layout_kind == Some(LayoutKind::Title) || (shape.subtitle && shape.content.is_empty()) || heads.cover
}

/// Le paramètre `name` de `schema`, s'il est un texte plain non lié au deck.
fn holds(schema: Option<&WidgetSchema>, name: &str) -> bool {
    schema.and_then(|w| w.param(name)).is_some_and(|p| p.bind.is_none() && matches!(p.ty, ParamType::Plain))
}

/// Le paramètre `name` de `schema` est-il lié à la clé `name` du deck ?
fn binds(schema: Option<&WidgetSchema>, name: &str) -> bool {
    schema.and_then(|w| w.param(name)).is_some_and(|p| p.bind.as_deref() == Some(name))
}

/// Le premier slot de mode `mode` de `schema`.
fn slot(schema: Option<&WidgetSchema>, mode: SlotMode) -> Option<&str> {
    schema?.params.iter().find_map(|p| match &p.ty {
        ParamType::Slot { mode: m, .. } if *m == mode => Some(p.name.as_str()),
        _ => None,
    })
}

/// Le modèle de `shape` : parmi ceux qui ont un slot si la slide a du contenu.
fn pick<'t>(shape: &Shape, profiles: &[Profile<'t>]) -> Option<Profile<'t>> {
    if !shape.content.is_empty() {
        let roomy: Vec<Profile<'t>> = profiles.iter().filter(|p| p.has_slot).cloned().collect();
        if let Some(p) = choose(shape, &roomy) {
            return Some(p.clone());
        }
    }
    choose(shape, profiles).cloned()
}

/// Les assets des éléments `list` et de leurs descendants.
fn collect_assets(list: &[Element], out: &mut BTreeSet<String>) {
    for e in list {
        if e.kind == "image"
            && let Some(a) = e.params.text("asset")
        {
            out.insert(a.to_owned());
        }
        for (_, children) in e.params.slots() {
            collect_assets(children, out);
        }
    }
}

/// Un décor de la slide, omis sans perte : un fond, une forme sans texte
/// dans une bande d'en-tête ou de pied, ou un filet au-dessus du bas du titre.
fn is_slide_decor(item: &Item, size: Size, title_bottom: f64, track: (f64, f64)) -> bool {
    if !is_bare_shape(item) {
        return false;
    }
    let f = &item.frame;
    is_backdrop(item, size) || in_band(f, size) || (is_thin(f, track) && f.y + f.h <= title_bottom)
}

/// La grille de référence d'une slide avant le choix du modèle : la grille
/// par défaut (12 × 6) sur la zone de corps titrée, les items ajustés dedans.
struct Reference {
    zone: Zone,
    grid: Grid,
    gap: f64,
    fit: Fit,
}

impl Reference {
    fn new(items: &[&Item], cx: &SlideCx<'_>) -> Self {
        let zone = body_zone(cx.page, true);
        let grid = Grid { columns: 12, rows: 6, gap: None };
        let gap = grid_gap(&grid, cx.page);
        let fit = Fit::new(items.iter().map(|i| &i.frame), cx.scale, &zone);
        Self { zone, grid, gap, fit }
    }

    fn cell(&self, f: &crate::source::Frame) -> Cell {
        snap(&self.zone, &self.grid, self.gap, to_zone(f, &self.fit, &self.zone))
    }

    /// Une piste de la grille en pt source.
    fn track(&self) -> (f64, f64) {
        track(&self.zone, &self.grid, self.gap, self.fit.scale)
    }
}

/// Largeur d'une colonne et hauteur d'une ligne de `grid` dans `zone`, ramenées en pt source par `scale`.
fn track(zone: &Zone, grid: &Grid, gap: f64, scale: f64) -> (f64, f64) {
    let (w, h) = track_size(zone, grid, gap);
    if scale > 0.0 { (w / scale, h / scale) } else { (0.0, 0.0) }
}

/// Les blocs d'un flux : les unités en ordre de lecture, la rangée de colonnes
/// (s'il faut une pile) à la place de sa première unité ; jamais une forme sans texte.
fn flow_order<'u, 'a>(units: &'u [Unit<'a>], shape: &Shape, stack: bool) -> Vec<Vec<&'u Unit<'a>>> {
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
fn snap_all(elements: &mut [Element], zone: &Zone, grid: &Grid, gap: f64) {
    let cells: Vec<Option<Cell>> = elements.iter().map(|e| e.frame.map(|f| snap(zone, grid, gap, f))).collect();
    let solid: Vec<(usize, Cell)> = cells
        .iter()
        .enumerate()
        .filter(|(k, _)| elements[*k].kind != "shape")
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

/// Les titres posés en paramètres de la slide (ou du deck pour la première
/// couverture) ; ceux que le widget ne déclare pas restent des contenus.
struct Titles<'a> {
    ops: Vec<Op>,
    extra: Vec<&'a Item>,
}

fn place_titles<'a>(
    heads: &Headings<'a>,
    first_cover: bool,
    schema: Option<&WidgetSchema>,
    slide: &mut Slide,
    losses: &mut Vec<Loss>,
) -> Titles<'a> {
    let mut out = Titles { ops: Vec::new(), extra: Vec::new() };
    for (item, name) in [(heads.kicker, "kicker"), (heads.title, "title"), (heads.subtitle, "subtitle")] {
        let Some(item) = item else { continue };
        let Some(tb) = item.text() else { continue };
        let text = plain_of(tb, &item.id, losses);
        if text.is_empty() {
            continue;
        }
        if first_cover && name != "kicker" {
            out.ops.push(Op::SetDeckParam { key: name.to_owned(), value: Value::String(text.clone()) });
        }
        if holds(schema, name) {
            slide.params.set(name, ParamValue::Text(text));
        } else if !(first_cover && binds(schema, name)) {
            out.extra.push(item);
        }
    }
    out
}

/// Met les textes de `unit` (groupes compris) dans `moved`, un paragraphe par
/// boîte, avec la perte `MovedToNotes` ; ses items sans texte n'ont pas de
/// place (`NoSlot`).
fn move_to_notes(unit: &Unit<'_>, moved: &mut Vec<String>, losses: &mut Vec<Loss>) {
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

/// La slide Oku de `src`, et ce qu'en dit le rapport.
///
/// `deck_titled` dit si une couverture a déjà rempli les paramètres du deck ;
/// la première couverture le passe à `true`. `None` seulement si `profiles`
/// est vide.
pub(crate) fn project_slide<'a>(
    src: &'a SourceSlide,
    profiles: &[Profile<'_>],
    deck_titled: &mut bool,
    cx: &SlideCx<'_>,
) -> Option<Projected<'a>> {
    let mut kept: Vec<&Item> = src.items.iter().filter(|i| !cx.decor.skips(i)).collect();
    let dropped = kept.len().saturating_sub(MAX_SLIDE_ITEMS);
    kept.truncate(MAX_SLIDE_ITEMS);
    let heads = headings(&kept, cx.size);
    let title_bottom = heads.title.map_or(0.0, |t| t.frame.y + t.frame.h);
    let reference = Reference::new(&content_items(&kept, &heads), cx);
    let content: Vec<&Item> = content_items(&kept, &heads)
        .into_iter()
        .filter(|i| !is_slide_decor(i, cx.size, title_bottom, reference.track()))
        .collect();
    let mut found = units(&content, |f| reference.cell(f));
    let mut shape = shape(src, cx.layouts, &heads, &found);
    let cover = is_cover(&shape, &heads);
    let first_cover = cover && !*deck_titled;
    // La couverture du deck reste une couverture si ses autres contenus sont
    // des textes (ils iront dans les notes) ; une image, un tableau ou un
    // graphique demandent un modèle qui les tienne.
    let as_title = first_cover
        && shape.content.iter().all(|k| matches!(k, ContentKind::Text | ContentKind::Card | ContentKind::Shape));
    if as_title {
        shape.layout_kind = Some(LayoutKind::Title);
    } else if cover && !first_cover {
        shape.layout_kind = Some(LayoutKind::SectionHeader);
    }
    let profile = if as_title { choose(&shape, profiles).cloned() } else { pick(&shape, profiles) }?;
    let template = &profile.template.slide;
    let schema = cx.registry.get(&template.kind);
    let id = format!("s{}", src.number);
    let mut losses: Vec<Loss> = cx.decor.losses(src.number).to_vec();
    if src.hidden {
        losses.push(Loss::slide(LossKind::HiddenSlide));
    }
    if dropped > 0 {
        losses.push(Loss::slide(LossKind::ItemsDropped { dropped: u32::try_from(dropped).unwrap_or(u32::MAX) }));
    }
    for item in [heads.kicker, heads.title, heads.subtitle].into_iter().flatten() {
        effect_losses(item, cx.effects, &mut losses);
    }
    let mut slide = Slide::new(id.clone(), &template.kind);
    let titles = place_titles(&heads, first_cover, schema, &mut slide, &mut losses);
    let mut ops = titles.ops;
    if first_cover {
        *deck_titled = true;
    }
    if !titles.extra.is_empty() {
        // Un titre sans paramètre devient un contenu : les unités sont refaites avec lui.
        let mut all = content.clone();
        all.extend(titles.extra.iter().copied());
        let rank: BTreeMap<&str, usize> = kept.iter().enumerate().map(|(k, i)| (i.id.as_str(), k)).collect();
        all.sort_by_key(|i| rank.get(i.id.as_str()).copied());
        found = units(&all, |f| reference.cell(f));
        shape = Shape { layout_kind: shape.layout_kind, ..super::template::shape(src, cx.layouts, &heads, &found) };
    }
    let (flow_slot, free_slot) = (slot(schema, SlotMode::Flow), slot(schema, SlotMode::Free));
    let flowable = !matches!(profile.layout, ContentLayout::Free | ContentLayout::Grid)
        && (found.len() <= 1 || shape.columns >= 2);
    let placement = match (flow_slot, free_slot) {
        (Some(_), None) => Placement::Flow,
        (Some(_), Some(_)) if flowable => Placement::Flow,
        (_, Some(_)) => Placement::Grid,
        (None, None) => Placement::Flow,
    };
    let stack = shape.columns >= 2 && shape.row.len() < found.len();
    if placement == Placement::Flow {
        if stack {
            slide.params.set("layout", ParamValue::Text("default".to_owned()));
        } else if let Some(layout) = template.params.get("layout") {
            slide.params.set("layout", layout.clone());
        }
    } else if let Some(grid) = template.params.get("grid") {
        slide.params.set("grid", grid.clone());
    }
    let free = template.params.get("grid").is_none() && profile.layout == ContentLayout::Free;
    let grid = slide.resolved_grid(cx.registry);
    let zone = body_zone(cx.page, slide.params.text("title").is_some());
    let gap = grid_gap(&grid, cx.page);
    let fit = Fit::new(found.iter().map(|u| u.frame()).collect::<Vec<_>>().iter(), cx.scale, &zone);
    let content_cx = ContentCx { track: track(&zone, &grid, gap, fit.scale), ..cx.content };
    let mut elements = Vec::new();
    let mut fallbacks = Vec::new();
    let mut origins = Vec::new();
    let mut kinds: Vec<String> = Vec::new();
    let blocks: Vec<Vec<&Unit<'a>>> = match placement {
        Placement::Flow => flow_order(&found, &shape, stack),
        // Le retrofit ne place jamais en cadres libres (hi-fi) : comme la grille.
        Placement::Grid | Placement::Free => found.iter().map(|u| vec![u]).collect(),
    };
    let target = match placement {
        Placement::Flow => flow_slot,
        Placement::Grid | Placement::Free => free_slot,
    };
    // Textes d'une couverture sans slot : dans les notes, en ordre de lecture.
    let mut moved: Vec<String> = Vec::new();
    for (k, block) in (1..).zip(&blocks) {
        let block_id = format!("{id}-{k}");
        let mut made = Vec::new();
        for (j, unit) in (1..).zip(block) {
            for item in unit.items() {
                effect_losses(item, cx.effects, &mut losses);
            }
            if target.is_none() && first_cover {
                move_to_notes(unit, &mut moved, &mut losses);
                continue;
            }
            let unit_id = if block.len() > 1 { format!("{block_id}-{j}") } else { block_id.clone() };
            origins.extend(unit.items().into_iter().map(|i| (i.id.clone(), unit_id.clone())));
            let c = element(unit, unit_id, &content_cx);
            for l in c.losses {
                note(&mut losses, l);
            }
            if let Some(kind) = c.fallback {
                if !kinds.contains(&kind) {
                    kinds.push(kind.clone());
                }
                fallbacks.push((unit.lead(), kind));
            }
            let Some(mut e) = c.element else { continue };
            if placement == Placement::Grid {
                e.frame = Some(to_zone(&unit.frame(), &fit, &zone));
            }
            made.push(e);
        }
        if block.len() > 1 && !made.is_empty() {
            let mut row = Element::new(block_id, "stack");
            row.params.set("layout", ParamValue::Text("columns".to_owned()));
            row.params.set("children", ParamValue::Slot(made));
            elements.push(row);
        } else {
            elements.extend(made);
        }
    }
    if placement == Placement::Grid && !free {
        snap_all(&mut elements, &zone, &grid, gap);
    }
    let mut assets = BTreeSet::new();
    collect_assets(&elements, &mut assets);
    match target {
        Some(name) if !elements.is_empty() => {
            slide.params.set(name, ParamValue::Slot(elements));
        }
        None if !elements.is_empty() => {
            for item in found.iter().flat_map(Unit::items) {
                note(&mut losses, Loss::on(&item.id, LossKind::NoSlot));
            }
        }
        _ => {}
    }
    ops.push(Op::AddSlide { index: None, defaults: false, slide });
    if !src.notes.trim().is_empty() {
        moved.push(src.notes.clone());
    }
    if !moved.is_empty() {
        ops.push(Op::SetNotes { slide: id.clone(), notes: moved.join("\n\n") });
    }
    let report = SlideReport {
        source: src.number,
        slide: id,
        template: profile.template.id.clone(),
        placement,
        losses,
        fallbacks: kinds,
        score: None,
    };
    Some(Projected { ops, report, assets, fallbacks, origins })
}

#[cfg(test)]
#[path = "slide_tests.rs"]
mod slide_tests;
