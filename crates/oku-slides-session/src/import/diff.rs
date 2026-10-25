//! Le diff d'une slide importée (`import_diff`).
//!
//! Sa page de référence et sa page Oku rendue maintenant, la note qui en sort et l'essai qu'il
//! compte.
//!
//! Sans région : les deux pages entières (la page source, [`super::REFERENCE_WIDTH`]
//! pixels de large), la slide notée de nouveau à ce que coûte sa note
//! ([`super::tune`] : un essai de la slide si son raster a changé depuis sa
//! note précédente, par ce diff ou par `import_report`, un essai global si
//! c'est l'état du deck entier qui a changé). Avec une région (un item source
//! de la slide) : les deux rognures de son cadre, à [`MARGIN`] pixels de marge
//! (bornée à la page), agrandies [`ZOOM`] fois ; rien n'est noté ni compté.
//! Les deux sont refusés pour une slide déjà notée sans essai restant :
//! `done`, `witness`, `unscored` (`Untunable`), `exhausted` ou `stalled`
//! (`TriesUsed`) ; la première note d'une slide, sa base, est toujours permise.
//!
//! La référence doit être faite (le travail de [`crate::Session::import_reference_job`],
//! lancé par l'appelant hors du verrou) : sinon `ReferencePending`. Le deck
//! est rendu s'il ne l'est pas encore ; un essai n'est compté qu'une fois les
//! deux images faites, et la rognure d'une région part de ses bords signés
//! (un cadre qui commence avant la page ne l'élargit pas).
//!
//! Les cadres sont en pt de la page Oku : en hi-fi, la zone de corps est la
//! page (marges nulles), ce sont donc les cadres des éléments pour `apply`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use oku_slides_core::Frame;
use oku_slides_import::metrics::{RegionScore, SlideScore};
use oku_slides_import::report::ReferenceCause;
use oku_slides_reference::Raster;

use super::queue::{LISTED, compare_failed, element_ids, refused};
use super::state::SlideRegion;
use super::tune::{Charge, SlideState, excess};
use super::tune_error::{
    HifiOnlySnafu, NoReferenceSnafu, ReferencePendingSnafu, TriesUsedSnafu, UnknownRegionSnafu, UnknownSlideSnafu,
    UntunableSnafu,
};
use super::{ImportMode, ImportState};
use crate::{Result, Session};

/// Agrandissement des rognures d'une région.
pub const ZOOM: u32 = 2;

/// Marge autour d'une région rognée, en pixels de la page source.
///
/// Elle se mesure avant agrandissement.
pub const MARGIN: u32 = 16;

/// Le diff d'une slide : deux images PNG, puis le JSON qui les accompagne.
#[derive(Debug)]
pub struct ImportDiff {
    /// L'image de référence : la page entière, ou la rognure de la région.
    pub reference: Vec<u8>,
    /// L'image Oku, rendue maintenant, au même cadre.
    pub oku: Vec<u8>,
    /// La note et l'état de la slide, ou la région montrée.
    pub json: Value,
}

impl Session {
    /// Compares slide `slide`, or its region `region` (a source item), to its reference.
    ///
    /// The reference must be made ([`Self::import_reference_job`]); rejected
    /// outside hi-fi, for an unknown slide or region, and for a slide that no
    /// longer has an attempt (zoom included).
    ///
    /// # Errors
    ///
    /// Fails without an import (`NoImport`), outside hi-fi (`HifiOnly`), without a reference
    /// (`ReferencePending`, `NoReference`), for an unknown slide or region, or without an attempt.
    pub fn import_diff(&mut self, slide: &str, region: Option<&str>) -> Result<ImportDiff> {
        let state = self.tuned_import()?;
        if state.mode() != ImportMode::Hifi {
            return Err(refused(HifiOnlySnafu.build()));
        }
        state.settle();
        match state.reference_status() {
            None => return Err(refused(ReferencePendingSnafu.build())),
            Some(Err(cause)) => return Err(refused(NoReferenceSnafu { cause }.build())),
            Some(Ok(_)) => {}
        }
        let unknown = || refused(UnknownSlideSnafu { slide }.build());
        if !state.report().slides.iter().any(|r| r.slide == slide) {
            return Err(unknown());
        }
        let current = state.slide_state(slide);
        let tuning = state.tuning(slide);
        // Une slide jamais notée par un outil n'a pas encore d'état : sa première note est sa base, gratuite.
        let based = tuning.is_some_and(|t| t.seen.is_some());
        if based && current.tries_left(tuning, state.limits()) == 0 {
            return Err(refused(match current {
                SlideState::Exhausted | SlideState::Stalled | SlideState::Pending => {
                    TriesUsedSnafu { slide, tries: tuning.map_or(0, |t| t.tries) }.build()
                }
                SlideState::Done | SlideState::Witness | SlideState::Unscored => {
                    UntunableSnafu { slide, state: current }.build()
                }
            }));
        }
        let ppp = state.pixel_per_pt();
        self.ensure_rendered()?;
        // Une slide supprimée depuis l'import n'a plus de page.
        let page = self.page_of(slide).and_then(|p| p.checked_sub(1)).ok_or_else(unknown)?;
        let (png, hash) = self.with_compiled(|c| Ok((c.png(page, ppp)?, c.page_hash(page))))?;
        let oku = Raster::from_png(&png).map_err(|e| compare_failed((ReferenceCause::Failed, e.to_string())))?;
        let keys = (self.deck_key()?, self.slide_key(slide)?);
        let state = self.tuned_import()?;
        match region {
            None => whole(state, slide, &oku, hash, keys),
            Some(item) => zoom(state, slide, item, &oku),
        }
    }
}

/// Le diff entier.
///
/// La slide notée de nouveau à ce que coûte sa note, les deux pages.
///
/// `page` : l'empreinte Typst de sa page ; `keys` : celles de l'état du deck et du modèle de la slide.
fn whole(
    state: &mut ImportState,
    slide: &str,
    oku: &Raster,
    page: Option<u128>,
    keys: (u128, u128),
) -> Result<ImportDiff> {
    // Les images d'abord : un essai n'est compté qu'une fois le diff sûr d'aboutir.
    let (refr, cropped) = state.pair(slide, oku).map_err(compare_failed)?;
    let (reference, oku_png) = (png(&refr)?, png(&cropped)?);
    let (charge, score) = state.observe(slide, oku, page, keys).map_err(compare_failed)?;
    let Some(score) = score else {
        // Une slide `pending` a un essai : la note n'est refusée qu'à une slide sans essai, vue plus haut.
        let tries = state.tuning(slide).map_or(0, |t| t.tries);
        return Err(refused(TriesUsedSnafu { slide, tries }.build()));
    };
    let (tuning, limits, threshold) = (state.tuning(slide), state.limits(), state.threshold());
    let current = state.slide_state(slide);
    let json = json!({
        "slide": slide,
        "score": score.score,
        "worst": score.worst(),
        "excess": excess(&score, threshold),
        "threshold": threshold,
        "state": current,
        "tries_left": current.tries_left(tuning, limits),
        "global_tries_left": state.tuner().global_tries_left(),
        "counted": charge == Charge::Slide,
        "charge": charge,
        "regions": ranked(state, slide, &score),
        "regions_more": score.regions.len().saturating_sub(LISTED),
        "suspect": score.regions.iter().filter(|r| r.suspect.is_some()).count(),
    });
    Ok(ImportDiff { reference, oku: oku_png, json })
}

/// Les [`LISTED`] premières régions classées de `score`, avec leur cadre et leur cible.
fn ranked(state: &ImportState, slide: &str, score: &SlideScore) -> Vec<Value> {
    let regions: BTreeMap<String, SlideRegion> =
        state.regions(slide).into_iter().map(|r| (r.item.clone(), r)).collect();
    score.regions.iter().take(LISTED).map(|r| region_json(state, regions.get(&r.item), Some(r), &r.item)).collect()
}

/// Une région.
///
/// Son item, sa note s'il y en a une, son cadre (pt de la page Oku) et ce qui la corrige.
fn region_json(state: &ImportState, region: Option<&SlideRegion>, score: Option<&RegionScore>, item: &str) -> Value {
    let mut v = json!({ "item": item });
    let Some(o) = v.as_object_mut() else { return v };
    if let Some(s) = score {
        o.insert("score".into(), json!(s.score));
        o.insert("shift".into(), json!(s.shift));
    }
    if let Some(limit) = score.and_then(|s| s.suspect).or(region.and_then(|r| r.suspect)) {
        o.insert("suspect".into(), json!(limit));
    }
    if let Some(r) = region {
        let round = |v: f64| (v * 100.0).round() / 100.0;
        let Frame { x, y, w, h } = r.frame;
        o.insert("frame".into(), json!({ "x": round(x), "y": round(y), "w": round(w), "h": round(h) }));
        match state.target(item) {
            Some(oku_slides_import::plan::ItemTarget::Param(param)) => {
                o.insert("param".into(), json!(param));
            }
            target => {
                o.insert("elements".into(), json!(element_ids(target)));
            }
        }
    }
    v
}

/// Le zoom.
///
/// Les deux rognures du cadre de la région `item`, agrandies ; rien n'est noté.
fn zoom(state: &ImportState, slide: &str, item: &str, oku: &Raster) -> Result<ImportDiff> {
    let unknown = || refused(UnknownRegionSnafu { slide, region: item }.build());
    let region = state.regions(slide).into_iter().find(|r| r.item == item).ok_or_else(unknown)?;
    let (refr, oku) = state.pair(slide, oku).map_err(compare_failed)?;
    let page_empty = || compare_failed((ReferenceCause::Failed, format!("{slide} : page vide")));
    let [x0, y0, w, h] = state.crop(region.frame, MARGIN, (refr.width(), refr.height())).ok_or_else(page_empty)?;
    let crop = |r: &Raster| r.crop(x0, y0, w, h).scaled(ZOOM);
    let image = |r: &Raster| crop(r).map_err(|e| compare_failed((ReferenceCause::Failed, e.to_string())));
    let score = state.report().slides.iter().find(|r| r.slide == slide).and_then(|r| r.score.as_ref());
    let scored = score.and_then(|s| s.regions.iter().find(|r| r.item == item));
    let json = json!({
        "slide": slide,
        "zoom": ZOOM,
        "crop": [x0, y0, w, h],
        "region": region_json(state, Some(&region), scored, item),
    });
    Ok(ImportDiff { reference: png(&image(&refr)?)?, oku: png(&image(&oku)?)?, json })
}

/// `r` en PNG ; un échec est un refus `Compare`.
fn png(r: &Raster) -> Result<Vec<u8>> {
    r.to_png().map_err(|e| compare_failed((ReferenceCause::Failed, e.to_string())))
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;
