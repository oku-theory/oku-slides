//! Theme picker thumbnails, rendered with no session lock held.
//!
//! [`Session::theme_job`] copies, under the lock, what the thumbnails need (the whole deck, each
//! theme's layer), shares the assets and lends the preview render slot. [`ThemeJob::run`] renders with
//! no lock; [`Session::restore_preview_slot`] takes the slot back. Each card renders the whole
//! deck and shows the page of its slide: page number, outline and links are the deck's own.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

use oku_slides_core::{Deck, Namespace};
use oku_slides_render_worker::RenderSlot;
use serde_json::{Map, Value};
use snafu::ResultExt;

use crate::agent::clip;
use crate::deck_theme::render_themed;
use crate::error::RenderSnafu;
use crate::theme_picker::{Bounds, NAME_CHARS, THEMES_RENDERED, ThemeCard, ThemeChoices, swatches};
use crate::{Result, Session, new_renderer};

/// A theme the picker offers: its namespace, name and the deck's layer under it.
#[derive(Debug)]
struct Offered {
    namespace: Namespace,
    name: String,
    /// `None`: the theme or a parent went missing, the card stays without thumbnail or swatches.
    layer: Option<Map<String, Value>>,
}

/// The picker's thumbnails to render, owned: no lock is held while they render.
#[derive(Debug)]
pub struct ThemeJob {
    deck: Deck,
    /// The slide every card shows; empty for an empty deck.
    shown: String,
    active: Namespace,
    offered: Vec<Offered>,
    /// The session's assets, shared: taking them copies no byte.
    assets: Arc<BTreeMap<String, Vec<u8>>>,
    /// The session's preview slot, lent; `None` for an empty deck (nothing to render).
    slot: Option<PreviewSlot>,
    bounds: Bounds,
}

/// The session's preview render slot, lent to a [`ThemeJob`] and handed back after it.
#[derive(Debug)]
pub struct PreviewSlot {
    slot: RenderSlot,
    /// The session's preview epoch when lent: a later reset makes the slot stale.
    epoch: u64,
}

#[cfg(test)]
impl ThemeJob {
    /// The assets the job renders with, to check they are the session's own.
    pub(crate) fn assets(&self) -> &Arc<BTreeMap<String, Vec<u8>>> {
        &self.assets
    }
}

impl ThemeJob {
    /// Renders the thumbnails within the bounds, then returns the cards and the lent slot.
    ///
    /// The active theme first, then the others in library order. A failed or oversized thumbnail
    /// is logged and its card left without one.
    ///
    /// # Blocking
    ///
    /// Renders the whole deck once per theme in the worker. The budget is checked between
    /// renders, so one full render may overrun it, and the active theme always renders. The
    /// session lock is free meanwhile, but the worker is not: an edit that renders waits for
    /// the thumbnail in progress.
    #[must_use]
    pub fn run(mut self) -> (ThemeChoices, Option<PreviewSlot>) {
        let scene = Scene { deck: &self.deck, shown: &self.shown, assets: &self.assets };
        let svgs = match self.slot.as_mut() {
            Some(lent) => thumbnails(&mut lent.slot, &scene, &self.active, &self.offered, self.bounds),
            None => HashMap::new(),
        };
        let cards = self
            .offered
            .into_iter()
            .map(|o| ThemeCard {
                name: clip(&o.name, NAME_CHARS),
                slide: self.shown.clone(),
                swatches: o.layer.as_ref().map(swatches).unwrap_or_default(),
                svg: svgs.get(&o.namespace).cloned(),
                namespace: o.namespace,
            })
            .collect();
        (ThemeChoices { cards, active: self.active }, self.slot)
    }
}

/// What every thumbnail renders: the whole deck, the slide shown and the assets.
struct Scene<'a> {
    deck: &'a Deck,
    shown: &'a str,
    assets: &'a BTreeMap<String, Vec<u8>>,
}

/// The thumbnails by theme: the active first, then the others in order, within `bounds`.
fn thumbnails(
    slot: &mut RenderSlot,
    scene: &Scene<'_>,
    active: &Namespace,
    offered: &[Offered],
    bounds: Bounds,
) -> HashMap<Namespace, Arc<str>> {
    let mut order: Vec<&Offered> = offered.iter().filter(|o| o.namespace == *active).collect();
    order.extend(offered.iter().filter(|o| o.namespace != *active));
    order.truncate(THEMES_RENDERED);
    let deadline = Instant::now() + bounds.budget;
    let mut svgs = HashMap::new();
    for o in order {
        if o.namespace != *active && Instant::now() >= deadline {
            break;
        }
        let Some(layer) = &o.layer else { continue };
        match thumbnail_svg(slot, scene, layer) {
            Ok(Some(svg)) if svg.len() <= bounds.thumb_bytes => {
                svgs.insert(o.namespace.clone(), svg.into());
            }
            Ok(Some(svg)) => {
                tracing::warn!(theme = %o.namespace, bytes = svg.len(), "miniature du thème trop lourde : écartée");
            }
            Ok(None) => tracing::warn!(theme = %o.namespace, "miniature du thème : slide absente du rendu"),
            Err(e) => tracing::warn!(theme = %o.namespace, error = %e.one_line(), "miniature du thème impossible"),
        }
    }
    svgs
}

/// The SVG of the scene's slide, its deck rendered with `layer`; `None` if it has no page.
fn thumbnail_svg(slot: &mut RenderSlot, scene: &Scene<'_>, layer: &Map<String, Value>) -> Result<Option<String>> {
    let compiled = render_themed(slot, scene.deck, layer, scene.assets).context(RenderSnafu)?;
    let Some(page) = compiled.layout().page_of(scene.shown) else { return Ok(None) };
    Ok(Some(compiled.svg(page - 1).context(RenderSnafu)?))
}

impl Session {
    /// The picker's thumbnails on `slide`, rendered later with no lock held.
    ///
    /// `None` outside deck mode. `slide` absent or unknown: the first one. Shares the assets
    /// (the worker only receives those it lacks). Lends the preview slot (opened if needed): give
    /// it back with [`restore_preview_slot`](Self::restore_preview_slot) once the job has run.
    ///
    /// # Errors
    ///
    /// Fails if the active theme is missing or the preview slot cannot open.
    pub fn theme_job(&mut self, slide: Option<&str>) -> Result<Option<ThemeJob>> {
        self.theme_job_within(slide, Bounds::DEFAULT)
    }

    /// [`theme_job`](Self::theme_job) within these bounds.
    pub(crate) fn theme_job_within(&mut self, slide: Option<&str>, bounds: Bounds) -> Result<Option<ThemeJob>> {
        if !self.in_deck_mode() {
            return Ok(None);
        }
        let active = self.active_theme()?;
        let deck = self.deck().clone();
        let shown = slide.and_then(|id| deck.slide(id)).or_else(|| deck.slides.first()).map(|s| s.id.clone());
        let offered = self
            .theme_list
            .iter()
            .map(|t| Offered {
                namespace: t.namespace.clone(),
                name: t.name.clone(),
                layer: self.offered_layer(&t.namespace),
            })
            .collect();
        let slot = match shown {
            Some(_) => Some(self.lend_preview_slot()?),
            None => None,
        };
        let assets = Arc::clone(&self.doc.sources.assets);
        Ok(Some(ThemeJob { deck, shown: shown.unwrap_or_default(), active, offered, assets, slot, bounds }))
    }

    /// Takes back the slot a [`ThemeJob`] borrowed; dropped if stale or no longer needed.
    ///
    /// Stale: fonts or widgets changed since it was lent. Not needed: another preview opened one.
    pub fn restore_preview_slot(&mut self, lent: Option<PreviewSlot>) {
        if let Some(lent) = lent
            && lent.epoch == self.preview_epoch
            && self.variant_renderer.is_none()
        {
            self.variant_renderer = Some(lent.slot);
        }
    }

    /// The preview slot, taken out of the session (opened with the usable packs if absent).
    fn lend_preview_slot(&mut self) -> Result<PreviewSlot> {
        // First: a pack gone faulty elsewhere is set aside, which resets the slot.
        let set = self.usable_packs()?;
        let slot = match self.variant_renderer.take() {
            Some(slot) => slot,
            None => new_renderer(&self.worker, &self.render_fonts, &self.localizer, &set, &self.registry)?,
        };
        Ok(PreviewSlot { slot, epoch: self.preview_epoch })
    }

    /// Forgets the preview slot: one lent out now comes back stale.
    pub(crate) fn reset_preview_slot(&mut self) {
        self.variant_renderer = None;
        self.preview_epoch += 1;
    }
}

#[cfg(test)]
#[path = "theme_job_tests.rs"]
mod theme_job_tests;
