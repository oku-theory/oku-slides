//! Une slide source → les ops d'une slide Oku.
//!
//! Les items omis par règle ([`Decor`]) sont écartés ; les titres viennent
//! des placeholders, sinon de la taille et de la position des textes
//! ([`headings`]) ; le reste se regroupe en unités ([`units`] : cartes,
//! textes fusionnés), sans les fonds (une forme sans texte d'au moins la
//! moitié de la page), les formes sans texte des bandes d'en-tête et de pied,
//! ni les filets posés au-dessus du bas du titre. La forme de la slide choisit
//! un profil de layout ([`choose`]) ; la slide Oku prend son layout
//! (`<namespace>/<layout>`) et ses paramètres (agencement `layout`, grille
//! `grid`), jamais de texte d'exemple.
//! Titre, surtitre et sous-titre vont dans les paramètres que le widget
//! déclare ; ceux qu'il ne déclare pas deviennent des contenus.
//!
//! Placement (décision 6) : une slide « titre + un corps » (au plus une
//! unité, ou une rangée de colonnes, seule ou entre des unités pleine
//! largeur) sur un profil à blocs va dans le slot `flow`, en ordre de
//! lecture ; une rangée entourée d'autres unités devient une pile `stack` en
//! colonnes, et la slide garde l'agencement `default`. Une forme sans texte
//! n'entre jamais dans un flux. Sinon les unités vont dans le slot libre, dans
//! l'ordre de dessin, leur boîte englobante ajustée dans la zone de corps
//! ([`Fit`]) : posées en cadre (`frame`) sur un profil libre, sans grille ;
//! sur un profil à grille, accrochées à la cellule la plus proche ([`snap`])
//! si aucun contenu n'y recouvre un autre, sinon laissées en cadres (la
//! grille du profil ne sait pas tenir l'arrangement).
//!
//! Slides de titre (décision 7) : la première couverture remplit les
//! paramètres du deck (`title`, `subtitle`) et prend un profil de couverture
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

use oku_slides_core::{
    Arrangement, Cell, Element, Grid, Op, ParamType, ParamValue, Slide, SlotMode, WidgetRegistry, WidgetSchema,
};
use serde_json::Value;

use super::cluster::{Unit, is_bare_shape, units};
use super::content::{ContentCx, effect_losses, is_thin, plain_of};
use super::decor::{Decor, in_band, is_backdrop};
use super::heading::{Headings, headings};
use super::measure::{Measure, TextTokens, resize, title_band};
use super::place::{Arrange, arrange, aspects, collect_assets, track};
use super::profile::{ContentKind, Profile, Shape, choose, content_items, shape, slide_params};
use super::text::note;
use super::zone::{Fit, PageTokens, Zone, body_zone, grid_gap, snap, to_zone};
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

/// Le profil de `shape` : parmi ceux qui ont un slot si la slide a du contenu.
fn pick<'p>(shape: &Shape, profiles: &[Profile<'p>]) -> Option<Profile<'p>> {
    if !shape.content.is_empty() {
        let roomy: Vec<Profile<'p>> = profiles.iter().filter(|p| p.has_slot).cloned().collect();
        if let Some(p) = choose(shape, &roomy) {
            return Some(p.clone());
        }
    }
    choose(shape, profiles).cloned()
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

/// La grille de référence d'une slide avant le choix du profil : la grille
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

/// La place d'un flux : la zone de corps sous le titre et le surtitre
/// réellement posés (lignes estimées, [`title_band`]).
struct Room<'t> {
    tokens: &'t TextTokens,
    width: f64,
    height: f64,
    layout: String,
}

impl<'t> Room<'t> {
    fn of(page: &'t PageTokens, slide: &Slide) -> Self {
        let zone = body_zone(page, false);
        let (title, kicker) = (slide.params.text("title"), slide.params.text("kicker"));
        let band = title_band(&page.text, title, kicker, zone.w, page.title_band - page.text.title);
        Self {
            tokens: &page.text,
            width: zone.w,
            height: zone.h - band,
            layout: slide.params.text("layout").unwrap_or("default").to_owned(),
        }
    }

    /// La hauteur que demandent les blocs `blocks`.
    fn need(&self, m: &Measure<'_>, blocks: &[Element]) -> f64 {
        m.flow(blocks, &self.layout, self.width, self.height)
    }

    /// La plus grande taille du thème, plus petite que le corps, à laquelle `blocks` tiennent.
    fn smaller(&self, m: &Measure<'_>, blocks: &[Element]) -> Option<&'static str> {
        self.tokens.smaller.iter().filter(|(_, size)| *size < self.tokens.body).find_map(|(name, _)| {
            let mut resized = blocks.to_vec();
            resize(&mut resized, name);
            (self.need(m, &resized) <= self.height).then_some(*name)
        })
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
    // graphique demandent un profil qui les tienne.
    let as_title = first_cover
        && shape.content.iter().all(|k| matches!(k, ContentKind::Text | ContentKind::Card | ContentKind::Shape));
    if as_title {
        shape.layout_kind = Some(LayoutKind::Title);
    } else if cover && !first_cover {
        shape.layout_kind = Some(LayoutKind::SectionHeader);
    }
    let profile = if as_title { choose(&shape, profiles).cloned() } else { pick(&shape, profiles) }?;
    let kind = profile.kind();
    let hints = &profile.source.1;
    let schema = cx.registry.get(&kind);
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
    let mut slide = Slide::new(id.clone(), &kind);
    slide.params = slide_params(hints);
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
        shape = Shape { layout_kind: shape.layout_kind, ..super::profile::shape(src, cx.layouts, &heads, &found) };
    }
    let (flow_slot, free_slot) = (slot(schema, SlotMode::Flow), slot(schema, SlotMode::Free));
    let flowable = !matches!(profile.arrangement, Arrangement::Free | Arrangement::Grid)
        && (found.len() <= 1 || shape.columns >= 2);
    let mut placement = match (flow_slot, free_slot) {
        (Some(_), None) => Placement::Flow,
        (Some(_), Some(_)) if flowable => Placement::Flow,
        (_, Some(_)) => Placement::Grid,
        (None, None) => Placement::Flow,
    };
    let stack = shape.columns >= 2 && shape.row.len() < found.len();
    // La slide porte déjà les paramètres du profil : en grille, son agencement tombe.
    let grid_slide = |mut s: Slide| {
        s.params.remove("layout");
        s
    };
    if placement == Placement::Flow {
        if stack {
            slide.params.set("layout", ParamValue::Text("default".to_owned()));
        }
    } else {
        slide = grid_slide(slide);
    }
    let target_of = |p: Placement| match p {
        Placement::Flow => flow_slot,
        Placement::Grid | Placement::Free => free_slot,
    };
    let aspects = aspects(&kept);
    let a = Arrange {
        id: &id,
        units: &found,
        shape: &shape,
        stack,
        to_notes: target_of(placement).is_none() && first_cover,
        free: !hints.params.contains_key("grid") && profile.arrangement == Arrangement::Free,
        scale: cx.scale,
        page: cx.page,
        registry: cx.registry,
        content: cx.content,
        effects: cx.effects,
        measure: Measure { tokens: &cx.page.text, aspects: &aspects },
    };
    let mut built = arrange(placement, &slide, &a);
    if placement == Placement::Flow
        && let Some(name) = target_of(placement)
        && !built.elements.is_empty()
    {
        // t-816 : un flux plus haut que sa zone passe en grille si elle tient,
        // sinon prend une taille plus petite du thème, sinon déborde, dit au rapport.
        let room = Room::of(cx.page, &slide);
        if room.need(&a.measure, &built.elements) > room.height {
            let grid = free_slot.map(|_| grid_slide(slide.clone())).map(|s| {
                let b = arrange(Placement::Grid, &s, &a);
                (s, b)
            });
            match grid.filter(|(_, b)| b.fits && !b.elements.is_empty()) {
                Some((s, b)) => {
                    (slide, built, placement) = (s, b, Placement::Grid);
                }
                None => match room.smaller(&a.measure, &built.elements) {
                    Some(size) => resize(&mut built.elements, size),
                    None => note(&mut losses, Loss::slide(LossKind::Overflow { element: format!("{id}/{name}") })),
                },
            }
        }
    }
    for l in built.losses {
        note(&mut losses, l);
    }
    let mut moved = built.moved;
    let elements = built.elements;
    let mut assets = BTreeSet::new();
    collect_assets(&elements, &mut assets);
    match target_of(placement) {
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
        profile: profile.name(),
        placement,
        losses,
        fallbacks: built.kinds,
        score: None,
    };
    Some(Projected { ops, report, assets, fallbacks: built.fallbacks, origins: built.origins })
}

#[cfg(test)]
#[path = "slide_tests.rs"]
mod slide_tests;
