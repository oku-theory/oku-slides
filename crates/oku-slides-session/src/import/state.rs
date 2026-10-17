//! L'état d'un import, gardé en mémoire tant que vit l'onglet du deck importé.
//!
//! Le rapport (scores courants), la source lue de la présentation (ses
//! régions), ce que l'agent corrige pour chaque item (éléments ou
//! paramètre), le témoin choisi à l'import et la place de son rendu de
//! référence, et, pour l'ajustement par l'agent, ses bornes et ses essais
//! ([`super::Tuner`] : par slide et pour le deck entier). Les notes, elles,
//! sont dans [`super::score`].
//!
//! Le témoin ne rend la présentation qu'une fois, au premier besoin, hors du
//! verrou de la session ([`ImportState::reference_job`], [`ReferenceJob`]) :
//! l'app n'attend pas le témoin pour ouvrir l'onglet ; la CLI le demande
//! aussitôt. Ses pages (PNG à [`REFERENCE_WIDTH`](super::REFERENCE_WIDTH) pixels de large) sont
//! gardées ; un échec aussi, et n'est pas retenté. Les octets du `.pptx` ne
//! servent qu'au témoin : jamais gardés en retrofit (ni sans témoin), ils
//! sont libérés dès que la référence est faite ([`ImportState::settle`]).
//! Les régions suspectes sont celles des limites du témoin choisi
//! (office2pdf, celui de l'app ; aucune sous LibreOffice).
//! En retrofit, l'état garde aussi le pack enfant, créé à la première
//! écriture d'un widget sur le pack cible ([`super::child_pack`]).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use oku_slides_core::Frame;
use oku_slides_i18n::{LiveLocalizer, Localizer};
use oku_slides_import::metrics::{self, Witness, WitnessLimit};
use oku_slides_import::plan::{GroupCandidate, ItemTarget, WidgetRequest};
use oku_slides_import::report::{ReferenceCause, ReferenceReport};
use oku_slides_import::source::{SourceDeck, SourceSlide};
use oku_slides_import::{CheckedPlan, ImportReport, hifi};
use oku_slides_reference::SOFFICE_WITNESS;
use oku_slides_render::Fonts;
use oku_slides_render_worker::ReferenceWitness;

use super::child_pack::ChildPack;
use super::geometry::Geometry;
use super::reference::{Failure, Reference, ReferenceJob, ReferenceSlot, limits_of, log_failure};
use super::tune::{TuneLimits, Tuner, Tuning};
use super::{ImportMode, Input};

/// Une région d'une slide importée.
///
/// Un item source et son cadre sur la page Oku.
#[derive(Debug, Clone, PartialEq)]
pub struct SlideRegion {
    /// Id de l'item source.
    pub item: String,
    /// Ce que l'agent corrige pour l'item, s'il est connu.
    ///
    /// Ses éléments, ou le paramètre qu'il remplit.
    pub target: Option<ItemTarget>,
    /// Le cadre de l'item, en pt de la page Oku.
    pub frame: Frame,
    /// La limite du témoin sur cet item : son écart est surtout celui du témoin.
    pub suspect: Option<WitnessLimit>,
}

/// L'état d'un import, gardé par l'onglet du deck importé.
#[derive(Debug)]
pub struct ImportState {
    pub(super) mode: ImportMode,
    /// Le plan appliqué, sans ses ops : rapport, demandes de widget, cibles des items.
    pub(super) plan: CheckedPlan,
    /// La source lue de la présentation.
    pub(super) source: SourceDeck,
    pub(super) fonts: Fonts,
    pub(super) threshold: f64,
    /// Le témoin choisi à l'import ; `None` : pas de référence (retrofit, `--reference none`).
    pub(super) witness: Option<ReferenceWitness>,
    /// Les octets du `.pptx`, pour le témoin seulement.
    ///
    /// `None` sans témoin ou une fois la référence faite.
    pub(super) bytes: Option<Arc<[u8]>>,
    /// Le rendu de référence, rempli une fois par le premier travail qui le fait.
    pub(super) reference: ReferenceSlot,
    pub(super) tuner: Tuner,
    /// L'empreinte Typst de la page Oku à la dernière note de chaque slide.
    ///
    /// Un filtre avant de rendre le raster.
    pub(super) scored: BTreeMap<String, u128>,
    /// La langue du journal : un échec du témoin y dit sa cause.
    pub(super) log: LiveLocalizer,
    /// Retrofit : le pack enfant, créé au premier widget écrit sur le pack cible.
    child: Option<ChildPack>,
}

impl ImportState {
    /// L'état d'un import tout juste appliqué ; `fonts` : celles du rendu, données au témoin.
    ///
    /// Le témoin est celui de l'app (office2pdf) en hi-fi, aucun en retrofit ;
    /// `threshold` : le seuil et les bornes de l'ajustement ; `log` : la langue
    /// courante, celle des échecs journalisés ; `child` : le pack enfant du retrofit.
    pub(crate) fn new(
        (mode, child): (ImportMode, Option<ChildPack>),
        plan: CheckedPlan,
        input: Input,
        fonts: Fonts,
        (threshold, limits): (f64, TuneLimits),
        log: LiveLocalizer,
    ) -> Self {
        let Input { bytes, source, .. } = input;
        let witness = (mode == ImportMode::Hifi).then_some(ReferenceWitness::Office2Pdf);
        Self {
            mode,
            plan,
            source,
            fonts,
            threshold,
            bytes: witness.is_some().then_some(bytes),
            witness,
            reference: ReferenceSlot::default(),
            tuner: Tuner::new(limits),
            scored: BTreeMap::new(),
            log,
            child,
        }
    }

    /// Retrofit : le pack enfant, créé au premier widget écrit sur le pack cible.
    pub(crate) fn child(&self) -> Option<&ChildPack> {
        self.child.as_ref()
    }

    /// Journalise un échec du témoin ou d'une note : sa cause localisée, son détail.
    pub(super) fn warn(&self, failure: &Failure, slide: Option<&str>) {
        log_failure(&self.log, failure, slide);
    }

    /// Le mode de l'import.
    pub fn mode(&self) -> ImportMode {
        self.mode
    }

    /// Le rapport, avec les scores courants.
    pub fn report(&self) -> &ImportReport {
        &self.plan.report
    }

    /// Le rapport, pour y relever les débordements d'un rendu.
    pub fn report_mut(&mut self) -> &mut ImportReport {
        self.plan.report_mut()
    }

    /// Le rapport écrit par `--report` : messages localisés et demandes de widget.
    pub fn report_json(&self, l: &Localizer) -> Value {
        self.plan.report_json(l)
    }

    /// Les demandes de widget du plan.
    pub fn widget_requests(&self) -> &[WidgetRequest] {
        &self.plan.widget_requests
    }

    /// Hi-fi : les groupes source répétés, candidats à un widget d'élément.
    pub fn groups(&self) -> &[GroupCandidate] {
        &self.plan.groups
    }

    /// Hi-fi : ce que l'agent corrige pour l'item source `item`, s'il est projeté.
    pub fn target(&self, item: &str) -> Option<&ItemTarget> {
        self.plan.targets.get(item)
    }

    /// Le seuil du score de la pire région (`[import] score_threshold`).
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Les bornes de l'ajustement par l'agent (`[import] tune_iterations`, `tune_stall`, `tune_global`).
    pub fn limits(&self) -> TuneLimits {
        self.tuner.limits()
    }

    /// Les polices du rendu Oku, aussi données au témoin.
    ///
    /// Celles de l'app et celles du deck.
    pub fn fonts(&self) -> &Fonts {
        &self.fonts
    }

    /// La source lue de la présentation.
    pub fn source(&self) -> &SourceDeck {
        &self.source
    }

    /// Les essais d'ajustement de `slide`, si un outil l'a notée.
    pub fn tuning(&self, slide: &str) -> Option<&Tuning> {
        self.tuner.tuning(slide)
    }

    /// Les essais d'ajustement de toutes les slides et le budget global.
    pub fn tuner(&self) -> &Tuner {
        &self.tuner
    }

    /// Les régions de la slide Oku `slide` : ses items source, dans l'ordre du dessin.
    ///
    /// Chaque cadre est en pt de la page Oku ; vide pour une slide inconnue.
    /// Les suspectes sont celles du témoin qui a rendu la référence, sinon du témoin choisi.
    pub fn regions(&self, slide: &str) -> Vec<SlideRegion> {
        let Some(src) = self.source_slide(slide) else { return Vec::new() };
        let rect = hifi::source_rect(&self.source);
        let scale = rect.w / self.source.size.w;
        metrics::regions(src, scale, self.witness_limits())
            .into_iter()
            .map(|r| SlideRegion {
                target: self.plan.targets.get(&r.item).cloned(),
                frame: Frame { x: rect.x + r.x, y: rect.y + r.y, w: r.w, h: r.h },
                suspect: r.suspect,
                item: r.item,
            })
            .collect()
    }

    /// Pixels par pt du rendu Oku d'une page.
    ///
    /// La page source y fait [`REFERENCE_WIDTH`](super::REFERENCE_WIDTH) pixels.
    pub fn pixel_per_pt(&self) -> f32 {
        Geometry::of(&self.source).ppp as f32
    }

    /// Choisit le témoin de la référence ; `None` : pas de référence.
    ///
    /// À faire avant le premier travail (la CLI : `--reference`) ; sans effet
    /// sur une référence déjà faite. Sans témoin, les octets du `.pptx` sont libérés.
    pub fn set_witness(&mut self, witness: Option<ReferenceWitness>) {
        if witness.is_none() {
            self.bytes = None;
        }
        self.witness = witness;
    }

    /// Le témoin LibreOffice manque ; `detail` dit pourquoi au journal (la CLI : son option).
    ///
    /// Gardé comme un échec du témoin, cause `missing` : aucune slide n'a de
    /// score, aucune région n'est suspecte, et le rapport dit pourquoi. Sans
    /// effet sur une référence déjà faite.
    pub fn witness_missing(&mut self, detail: String) {
        self.witness_unavailable((ReferenceCause::Missing, detail));
    }

    /// Le témoin LibreOffice trouvé est refusé ; `detail` dit pourquoi (un script, sur macOS).
    ///
    /// Comme [`Self::witness_missing`], cause `failed` (t-912).
    pub fn witness_refused(&mut self, detail: String) {
        self.witness_unavailable((ReferenceCause::Failed, detail));
    }

    /// Le témoin LibreOffice manque au rapport pour la cause de `failure`.
    fn witness_unavailable(&mut self, failure: Failure) {
        self.warn(&failure, None);
        let missing = Reference { witness: SOFFICE_WITNESS.to_owned(), limits: Witness::Other, pages: Err(failure.0) };
        if self.reference.set(missing).is_err() {
            tracing::debug!("référence déjà faite : le témoin manquant n'y change rien");
        }
        self.set_witness(None);
        self.settle();
    }

    /// Le rendu de référence à faire hors de l'état, s'il reste à faire.
    ///
    /// `None` sans témoin choisi. Tant que la référence manque, chaque appel
    /// rend un travail sur la même place : le premier qui tourne rend, les
    /// autres attendent son résultat.
    pub fn reference_job(&self) -> Option<ReferenceJob> {
        if self.reference.get().is_some() {
            return None;
        }
        let (witness, bytes) = (self.witness.clone()?, self.bytes.clone()?);
        let slides = self.plan.report.slides.iter().map(|r| (r.slide.clone(), r.source)).collect();
        let width = Geometry::of(&self.source).width;
        let (slot, log, fonts) = (self.reference.clone(), self.log.clone(), self.fonts.clone());
        Some(ReferenceJob { bytes, fonts, slides, width, witness, slot, log })
    }

    /// Le nom du témoin, ou la cause de son échec.
    ///
    /// `None` tant que la référence n'est pas faite.
    pub fn reference_status(&self) -> Option<Result<&str, ReferenceCause>> {
        let reference = self.reference.get()?;
        Some(reference.pages.as_ref().map(|_| reference.witness.as_str()).map_err(Clone::clone))
    }

    /// La page PNG du témoin pour la slide Oku `slide`.
    pub fn reference_page(&self, slide: &str) -> Option<&[u8]> {
        let pages = self.reference.get()?.pages.as_ref().ok()?;
        pages.get(slide).map(Vec::as_slice)
    }

    /// Les limites du témoin qui a rendu la référence, sinon celles du témoin choisi.
    pub(super) fn witness_limits(&self) -> Witness {
        match (self.reference.get(), &self.witness) {
            (Some(reference), _) => reference.limits,
            (None, Some(witness)) => limits_of(witness),
            (None, None) => Witness::Other,
        }
    }

    /// Une référence faite.
    ///
    /// Libère les octets du `.pptx` et pose au rapport le témoin, le seuil et la cause d'une référence
    /// manquante.
    ///
    /// Sans effet tant que la référence n'est pas faite.
    pub(crate) fn settle(&mut self) {
        let Some(reference) = self.reference.get() else { return };
        self.bytes = None;
        let witness = reference.witness.clone();
        let unavailable = reference.pages.as_ref().err().cloned();
        let threshold = self.threshold;
        self.plan.report_mut().reference = Some(ReferenceReport { witness, threshold, unavailable });
    }

    /// La slide source de la slide Oku `slide`.
    pub(super) fn source_slide(&self, slide: &str) -> Option<&SourceSlide> {
        let number = self.plan.report.slides.iter().find(|r| r.slide == slide)?.source;
        self.source.slides.iter().find(|s| s.number == number)
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
