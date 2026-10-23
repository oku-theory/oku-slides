//! Projecteur retrofit : une [`SourceDeck`] projetée sur les profils des packs
//! de layouts choisis et un thème existant.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime, éléments libres placés sur la
//! grille d'une zone de corps approchée par ses jetons de page ([`PageTokens`]).
//! [`project`] rend un [`ImportPlan`] : une `AddSlide` par slide source (ids
//! `s<numéro>`, éléments `s<numéro>-<rang>`), les paramètres du deck tirés de
//! la première couverture et la langue du texte, les notes, les assets référencés, les demandes de
//! widget et le rapport, couverture du contenu comprise. Aucun aléa, aucune
//! horloge : même source, même plan.

pub(crate) mod chart;
mod cluster;
pub(crate) mod color;
pub(crate) mod content;
pub(crate) mod decor;
mod group;
mod heading;
mod measure;
mod place;
mod profile;
mod slide;
pub(crate) mod table;
pub(crate) mod text;
pub(crate) mod zone;

use std::collections::BTreeMap;

use oku_slides_core::{Deck, LayoutProfile, Namespace, Op, WidgetRegistry, apply_all};
use serde_json::Value;

pub use color::ColorTokens;
pub use measure::{CardTokens, TextTokens};
pub use zone::PageTokens;

use self::content::ContentCx;
use self::decor::Decor;
use self::slide::{SlideCx, project_slide};
use crate::coverage::coverage;
use crate::error::{ImportError, no_profiles};
use crate::pattern::signature;
use crate::plan::{ImportPlan, WidgetRequest};
use crate::report::{ImportReport, Mode};
use crate::source::{Item, ItemKind, SourceDeck};

/// La cible d'un retrofit : les profils des packs de layouts, le registre, les jetons du thème.
#[derive(Debug, Clone)]
pub struct RetrofitTarget<'a> {
    /// Les profils de chaque pack de layouts cible, sous son espace de noms :
    /// dans l'ordre des packs cibles, puis celui de leur manifeste.
    pub profiles: &'a [(Namespace, LayoutProfile)],
    /// Le registre des widgets (`base` et les packs de layouts cibles).
    pub registry: &'a WidgetRegistry,
    /// Les couleurs effectives du thème.
    pub colors: ColorTokens,
    /// Les jetons de page effectifs du thème.
    pub page: PageTokens,
}

/// Ajoute l'occurrence `item` (rendue par le repli `fallback` sur la slide
/// `slide`) à la demande de même signature, ou en crée une.
fn request(requests: &mut Vec<WidgetRequest>, item: &Item, fallback: &str, slide: &str) {
    let children = match &item.kind {
        ItemKind::Group { items, .. } => items.as_slice(),
        _ => std::slice::from_ref(item),
    };
    let sig = signature(children);
    match requests.iter_mut().find(|r| r.signature == sig) {
        Some(r) => {
            if r.slides.last().map(String::as_str) != Some(slide) {
                r.slides.push(slide.to_owned());
            }
        }
        None => requests.push(WidgetRequest {
            signature: sig,
            slides: vec![slide.to_owned()],
            sample: children.to_vec(),
            fallback: fallback.to_owned(),
        }),
    }
}

/// Le plan qui construit, sur la cible `target`, le deck Oku de `deck`.
///
/// Échelle : la page source est ramenée dans la page du thème en gardant ses
/// proportions (`min(largeur / w, hauteur / h)`, soit `1920 / w` pour une
/// source 16:9). Chaque slide prend le layout du profil le mieux noté
/// (`profile::choose`).
///
/// La couverture (`report.coverage`) est mesurée sur le deck que donnent les
/// ops appliquées à un deck vide sans thème ni pack embarqué : le thème ne
/// change pas le contenu, et le registre de la cible suffit à appliquer ;
/// toujours présente.
///
/// # Errors
///
/// [`ImportErrorKind::NoProfiles`](crate::ImportErrorKind::NoProfiles) si
/// les packs cibles ne proposent aucun profil ;
/// [`ImportErrorKind::PlanRefused`](crate::ImportErrorKind::PlanRefused) si
/// le cœur refuse le plan, valide par construction (un bogue).
pub fn project(deck: &SourceDeck, target: &RetrofitTarget<'_>) -> Result<ImportPlan, ImportError> {
    zone::check_defaults()?;
    if target.profiles.is_empty() {
        return Err(no_profiles());
    }
    let profiles = profile::profiles(target.profiles, target.registry);
    let scale = if deck.size.w > 0.0 && deck.size.h > 0.0 {
        (target.page.width / deck.size.w).min(target.page.height / deck.size.h)
    } else {
        1.0
    };
    let colors = target.colors.clone().with_accents(deck);
    let decor = Decor::of(deck);
    let effects = content::effect_items(&deck.warnings);
    let cx = SlideCx {
        registry: target.registry,
        page: &target.page,
        content: ContentCx { tokens: &colors, assets: &deck.assets, track: (0.0, 0.0) },
        layouts: &deck.layouts,
        size: deck.size,
        scale,
        decor: &decor,
        effects: &effects,
    };
    let mut report = ImportReport::new(Mode::Retrofit);
    report.warnings = deck.warnings.clone();
    let mut plan = ImportPlan::new(report);
    let mut origins = BTreeMap::new();
    // Le cœur ne porte la langue qu'au deck : celle de la plus grande part du texte.
    if let Some(lang) = deck.language() {
        plan.ops.push(Op::SetDeckParam { key: "lang".to_owned(), value: Value::String(lang) });
    }
    let mut deck_titled = false;
    let mut deck_slide = None;
    for src in &deck.slides {
        let was_titled = deck_titled;
        let Some(p) = project_slide(src, &profiles, &mut deck_titled, &cx) else { continue };
        if deck_titled && !was_titled {
            deck_slide = Some(src.number);
        }
        plan.ops.extend(p.ops);
        origins.extend(p.origins);
        for id in p.assets {
            if let Some(a) = deck.assets.get(&id) {
                plan.assets.insert(id, a.clone());
            }
        }
        for (item, fallback) in &p.fallbacks {
            request(&mut plan.widget_requests, item, fallback, &p.report.slide);
        }
        plan.report.slides.push(p.report);
    }
    let mut scratch = Deck::empty();
    apply_all(&mut scratch, plan.ops.clone(), target.registry).map_err(crate::error::plan_refused)?;
    plan.report.coverage = Some(coverage(deck, &scratch, target.registry, &origins, deck_slide, &|i| decor.skips(i)));
    plan.origins = origins;
    Ok(plan)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
