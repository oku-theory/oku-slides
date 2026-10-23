//! Projecteur hi-fi : la source reproduite fidèlement, avec un pack de layouts
//! et des jetons générés pour elle, sur le thème Neutre.
//!
//! [`project`] enchaîne : la page ([`page`]), les polices ([`crate::fonts::resolve`] :
//! familles de la source et du thème), les jetons générés ([`theme`], ceux du
//! master de la première slide), le pack généré ([`pack`] : décor des masters,
//! un widget par layout utilisé, schémas déclarés), puis chaque slide
//! ([`slide`]). Le plan porte le pack (à ajouter au bundle), les
//! schémas déclarés (que la sonde du rendu doit retrouver), les polices à
//! embarquer, les assets, et des ops qui épinglent d'abord `base` (widgets) et
//! `essentiel` (layouts, pour le repli `essentiel/blank`), désignent le thème
//! (`set_theme`, que la session complète), posent les jetons (`set_tokens`) et
//! ajoutent le pack généré (`add_pack` du pack de layouts `deck-layouts`), posent la langue du texte (`lang`,
//! [`SourceDeck::language`]), puis ajoutent les slides (`s<numéro>`, éléments
//! `s<numéro>-<rang>`) et leurs notes. Les valeurs d'exemple d'un widget sont
//! les textes de la première slide de son layout.
//!
//! Pertes : celles du pack (fond, décor) vont à la première slide de leur
//! layout ou de leur master, `PageFitted` à la première slide, `LayoutDropped`
//! (layout sans widget, voir [`PackLimits`]) à la première slide du layout. La couverture
//! se mesure comme au retrofit, sans rien omettre : le décor des masters et
//! des layouts n'est pas dans les items des slides. Aucun aléa, aucune
//! horloge : ids et date de création viennent de la cible.

mod color;
mod element;
mod element_text;
mod geometry;
mod groups;
mod pack;
mod page;
mod slide;
mod table;
mod theme;
mod typst;
mod widget;

use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::{Deck, Namespace, Op, PackKind, PackPin, Slide, WidgetRegistry, apply_all, kinds};
use serde_json::Value;
use uuid::Uuid;

use self::element::{ElementCtx, effect_items};
pub use self::groups::GROUP_MIN_SLIDES;
pub use self::pack::PackLimits;
use self::pack::{GeneratedPack, generate_pack, generated_registry, pack_manifest};
use self::page::Page;
use self::slide::{Projected, SlideCx, project_slide};
use self::table::{TableLook, first_table};
use self::theme::{masterless_tokens, theme_colors, theme_master, theme_tokens};
use self::typst::GenCtx;
use crate::coverage::coverage;
use crate::fonts::{FontCatalog, resolve, sought};
use crate::plan::ImportPlan;
use crate::report::{ImportReport, Loss, LossKind, Mode, SlideReport};
use crate::retrofit::text::note;
use crate::source::{SourceDeck, ThemeFonts};

/// La cible d'un import hi-fi : les packs intégrés, leurs épinglages, les polices disponibles,
/// le thème à épingler et l'identité du pack généré.
#[derive(Debug, Clone)]
pub struct HifiTarget<'a> {
    /// Le registre des packs intégrés `base` et `essentiel` : le pack généré s'y ajoute.
    pub registry: &'a WidgetRegistry,
    /// L'épinglage de `base` que le deck importé prend ; la session le remplit.
    pub base: PackPin,
    /// L'épinglage d'`essentiel` que le deck importé prend ; la session le remplit.
    pub essentiel: PackPin,
    /// Les polices de l'app et du système.
    pub fonts: &'a FontCatalog,
    /// Id du pack généré.
    pub pack_id: Uuid,
    /// Le thème que le deck importé épingle (Neutre) ; la session en remplit la fermeture.
    pub theme: Namespace,
    /// Nom du pack généré (celui de la présentation).
    pub name: String,
    /// Création, au format des manifestes (`AAAA-MM-JJTHH:MM:SSZ`).
    pub created: String,
    /// Les bornes de la sonde des packs : le pack généré s'y tient.
    pub limits: PackLimits,
}

/// Le plan qui construit le deck Oku fidèle de `deck`, sur un thème et un
/// pack générés pour lui.
///
/// La couverture (`report.coverage`) est mesurée sur le deck que donnent les
/// ops appliquées à un deck vide avec le registre déclaré ; `None` seulement
/// si le cœur refusait le plan (un bogue).
pub fn project(deck: &SourceDeck, target: &HifiTarget<'_>) -> ImportPlan {
    let page = Page::new(deck.size);
    let master = theme_master(deck);
    let theme_fonts = master.and_then(|m| deck.themes.get(m.theme)).map(|t| &t.fonts);
    let mut fonts = resolve(&deck.fonts, &theme_families(deck), target.fonts);
    fonts.settle_fallback(theme_fonts.map(|f| f.minor.as_str()), target.fonts);
    let colors = master.map(|m| theme_colors(deck, m)).unwrap_or_default();
    let lang = deck.language();
    let base = GenCtx {
        page: &page,
        colors: &colors,
        fonts: &fonts,
        assets: &deck.assets,
        theme_fonts,
        lang: lang.as_deref(),
    };
    let pack = generate_pack(deck, &base, target.limits);
    let registry = generated_registry(target.registry, &pack).unwrap_or_else(|e| {
        debug_assert!(false, "schémas déclarés refusés, un bogue du générateur : {e:?}");
        target.registry.clone()
    });
    let effects = effect_items(&deck.warnings);
    let table = first_table(deck).map(|t| TableLook::of(t, &colors, &page));
    let cx = SlideCx {
        deck,
        pack: &pack,
        registry: &registry,
        elements: ElementCtx { base, effects: &effects, table: table.as_ref() },
    };

    let mut report = ImportReport::new(Mode::Hifi);
    report.warnings = deck.warnings.clone();
    report.fonts = fonts.choices.clone();
    let mut plan = ImportPlan::new(report);
    let mut origins = BTreeMap::new();
    let mut assets = pack.assets.clone();
    let mut examples = BTreeMap::new();
    let mut slides = Vec::new();
    let mut dropped: BTreeSet<&str> = pack.dropped.iter().map(String::as_str).collect();
    for src in &deck.slides {
        let Projected { slide, mut report, assets: drawn, origins: o, targets: t } = project_slide(src, &cx);
        // Un layout sans widget est dit une fois, à sa première slide.
        if let Some(layout) = src.layout.as_deref().and_then(|l| dropped.take(l))
            && let Some(l) = deck.layouts.iter().find(|x| x.id == layout)
        {
            report.losses.insert(0, Loss::slide(LossKind::LayoutDropped { layout: l.name.clone() }));
        }
        if let Some(layout) = &src.layout
            && let Some(w) = pack.widget(layout)
        {
            examples.entry(w.layout.clone()).or_insert_with(|| slide.clone());
        }
        assets.extend(drawn);
        origins.extend(o);
        plan.targets.extend(t);
        plan.report.slides.push(report);
        slides.push(slide);
    }
    attach_pack_losses(deck, &pack, &mut plan.report.slides);
    if let (Some(loss), Some(first)) = (page.loss(), plan.report.slides.first_mut()) {
        first.losses.insert(0, Loss::slide(loss));
    }

    let tokens = master.map_or_else(|| masterless_tokens(&fonts), |m| theme_tokens(deck, m, &fonts, &page));
    let manifest = pack_manifest(&pack, target.pack_id, &target.name, &target.created, &examples);
    let pack_files = manifest.to_files(PackKind::Layouts, pack.files.clone());
    let Ok(pack_files) = pack_files else {
        debug_assert!(false, "un manifeste se sérialise toujours");
        return plan;
    };
    // `base` d'abord : le pack généré le requiert, et les éléments sont des widgets de `base` ;
    // `essentiel` ensuite, pour les slides de repli `essentiel/blank`.
    plan.ops.push(Op::AddPack {
        kind: PackKind::Widgets,
        namespace: kinds::base::namespace(),
        pin: target.base.clone(),
    });
    plan.ops.push(Op::AddPack {
        kind: PackKind::Layouts,
        namespace: kinds::essentiel::namespace(),
        pin: target.essentiel.clone(),
    });
    // La fermeture des thèmes (`themes`) est celle de la session, qui lit les manifestes.
    plan.ops.push(Op::SetTheme { theme: Some(target.theme.clone()), themes: BTreeMap::new() });
    plan.ops.push(Op::SetTokens { tokens: Value::Object(tokens) });
    plan.ops.push(Op::AddPack {
        kind: PackKind::Layouts,
        namespace: Namespace::deck_layouts(),
        pin: PackPin { id: target.pack_id, version: pack_files.version(), requires: manifest.requires.clone() },
    });
    if let Some(lang) = &lang {
        plan.ops.push(Op::SetDeckParam { key: "lang".to_owned(), value: Value::String(lang.clone()) });
    }
    for slide in slides {
        let notes = (!slide.notes.is_empty()).then(|| (slide.id.clone(), slide.notes.clone()));
        plan.ops.push(Op::AddSlide { index: None, defaults: false, slide: Slide { notes: String::new(), ..slide } });
        if let Some((slide, notes)) = notes {
            plan.ops.push(Op::SetNotes { slide, notes });
        }
    }
    plan.assets = assets.into_iter().filter_map(|id| Some((id.clone(), deck.assets.get(&id)?.clone()))).collect();
    plan.pack = Some(pack_files);
    plan.schemas = pack.schemas().cloned().collect();
    plan.fonts = fonts.files;

    // La couverture ne dépend pas du thème : sans la fermeture que la session remplit, le cœur
    // refuserait `set_theme`, que le deck de mesure prend donc sans thème.
    let measured = plan
        .ops
        .iter()
        .map(|op| match op {
            Op::SetTheme { .. } => Op::SetTheme { theme: None, themes: BTreeMap::new() },
            op => op.clone(),
        })
        .collect();
    let mut scratch = Deck::empty();
    plan.report.coverage = match apply_all(&mut scratch, measured, &registry) {
        Ok(()) => Some(coverage(deck, &scratch, &registry, &origins, None, &|_| false)),
        Err(e) => {
            debug_assert!(false, "le cœur refuse le plan, valide par construction : {e:?}");
            None
        }
    };
    plan.origins = origins;
    plan.groups = groups::repeated(deck);
    plan
}

/// La page source de `deck` sur la page Oku (pt page) : le rendu Oku d'une
/// slide, rogné à ce cadre, se compare au rendu de référence de la source.
pub fn source_rect(deck: &SourceDeck) -> oku_slides_core::Frame {
    Page::new(deck.size).source_rect()
}

/// Les familles que [`project`] cherche au catalogue des polices pour
/// `deck` (voir [`sought`]) : l'appelant n'y ajoute que celles du système
/// qui en sont.
pub fn sought_fonts(deck: &SourceDeck) -> BTreeSet<String> {
    sought(&deck.fonts, &theme_families(deck))
}

/// Les polices des titres et du corps du thème du master de la première slide,
/// latines puis des autres écritures.
fn theme_families<'a>(deck: &'a SourceDeck) -> Vec<&'a str> {
    let fonts = theme_master(deck).and_then(|m| deck.themes.get(m.theme)).map(|t| &t.fonts);
    let all = |f: &'a ThemeFonts| {
        let scripts = [&f.major_ea, &f.minor_ea, &f.major_cs, &f.minor_cs].into_iter().flatten().map(String::as_str);
        [f.major.as_str(), f.minor.as_str()].into_iter().chain(scripts).collect()
    };
    fonts.map(all).unwrap_or_default()
}

/// Attache chaque perte du pack à la première slide de son layout, sinon de
/// son master (le propriétaire est le préfixe de l'id d'item : `l3/7`, `m1`),
/// sinon à la première slide.
fn attach_pack_losses(deck: &SourceDeck, pack: &GeneratedPack, slides: &mut [SlideReport]) {
    for loss in &pack.losses {
        let owner = loss.item.as_deref().and_then(|i| i.split('/').next()).unwrap_or_default();
        let layouts: BTreeSet<&str> =
            deck.layouts.iter().filter(|l| l.id == owner || l.master == owner).map(|l| l.id.as_str()).collect();
        let at = deck.slides.iter().position(|s| s.layout.as_deref().is_some_and(|l| layouts.contains(l)));
        let Some(report) = slides.get_mut(at.unwrap_or(0)) else { return };
        note(&mut report.losses, loss.clone());
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
