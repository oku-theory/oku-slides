//! Le témoin d'un import hi-fi : le rendu de référence de la présentation.
//!
//! Le témoin (`office2pdf`, celui de l'app ; LibreOffice pour la CLI
//! `--reference soffice`, cherché parmi `[import] soffice` par [`soffice`], qui
//! refuse un script sur macOS)
//! rend le `.pptx` dans un processus de rendu à lui ([`witness_worker`], borné
//! par `[import] reference_timeout_ms` ; LibreOffice par `soffice_timeout_ms`,
//! le processus de rendu attendant [`SOFFICE_GRACE_MS`] de plus), par tranches
//! d'au plus [`SPAN`] slides source : sa mémoire ne croît pas avec le deck.
//! Ses pages font [`REFERENCE_WIDTH`] pixels de large. Seules les limites
//! d'office2pdf sont connues : sous LibreOffice, aucune région n'est suspecte.
//! Un échec n'est jamais une erreur de l'import : sa cause ([`ReferenceCause`])
//! va au rapport.
//!
//! [`ReferenceJob`] détache ce rendu de l'état de l'import : l'app comme la
//! CLI le font hors du verrou de la session, avec le témoin choisi à l'import.
//! Tous les travaux d'un import partagent sa place : le premier rend, les
//! autres attendent son résultat (un seul rendu du témoin, même sous des
//! appels concurrents), et un travail ne remplit jamais que la place de
//! l'import qui l'a donné.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use oku_slides_i18n::{LiveLocalizer, Localize};
use oku_slides_import::metrics::Witness;
use oku_slides_import::report::ReferenceCause;
use oku_slides_platform::Platform;
use oku_slides_platform::config::RenderSettings;
use oku_slides_reference::{ReferenceError, SlideSpan, find_soffice};
use oku_slides_render::{Fonts, RenderError, RenderErrorKind};
use oku_slides_render_worker::{ReferenceWitness, RenderWorker};

/// Largeur, en pixels, de la page source comparée (spec : 960 px).
pub const REFERENCE_WIDTH: u32 = 960;

/// Slides rendues par le témoin en une requête, au plus.
pub const SPAN: usize = 20;

/// Délai du processus de rendu au-delà de celui de LibreOffice.
///
/// Le temps d'écrire ses fichiers et de lire son PDF : LibreOffice est tué
/// par son propre délai d'abord, jamais orphelin d'un processus de rendu tué.
const SOFFICE_GRACE_MS: u64 = 10_000;

/// Les pages d'une tranche rendues par le témoin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessPages {
    /// Nom et version du témoin (`office2pdf 0.8.1`).
    pub witness: String,
    /// Pages que le témoin a rendues pour la tranche.
    pub count: usize,
    /// Une page PNG par slide de la tranche.
    ///
    /// Vide si `count` n'est pas le nombre de slides.
    pub pages: Vec<Vec<u8>>,
}

/// Ce qui rend les pages de référence d'une présentation.
///
/// Implement this trait to give [`ReferenceJob::run_with`] another witness:
/// the real witness runs in its own render worker ([`ReferenceJob::run`]), tests use a fake.
pub trait WitnessRenderer: Send + Sync {
    /// Le nom du témoin, tant que sa version n'est pas connue.
    fn name(&self) -> &str;

    /// Le témoin dont `metrics` connaît les limites : ses régions suspectes.
    fn limits(&self) -> Witness;

    /// Les pages de la tranche `span` de `pptx`, à `width` pixels de large.
    ///
    /// Les polices `fonts` passent en priorité.
    fn render(&self, pptx: &[u8], fonts: &Fonts, span: SlideSpan, width: u32) -> Result<WitnessPages, RenderError>;
}

/// Un témoin et le processus de rendu à lui qui le fait tourner.
#[derive(Debug, Clone)]
pub(crate) struct WitnessWorker {
    witness: ReferenceWitness,
    worker: RenderWorker,
}

impl WitnessRenderer for WitnessWorker {
    fn name(&self) -> &str {
        self.witness.name()
    }

    fn limits(&self) -> Witness {
        limits_of(&self.witness)
    }

    fn render(&self, pptx: &[u8], fonts: &Fonts, span: SlideSpan, width: u32) -> Result<WitnessPages, RenderError> {
        let reference = self.worker.reference(&self.witness, pptx, fonts, Some(span))?;
        let count = reference.page_count();
        let pages = if count == span_len(span) {
            (0..count).map(|i| reference.png(i, width)).collect::<Result<_, _>>()?
        } else {
            Vec::new()
        };
        Ok(WitnessPages { witness: reference.witness().to_owned(), count, pages })
    }
}

/// Le témoin `witness` dans un processus de rendu à lui.
///
/// Distinct de celui du rendu Oku : le témoin n'ajoute pas sa mémoire à celle du rendu Oku ; son délai est
/// `[import] reference_timeout_ms`, ou celui de LibreOffice et
/// [`SOFFICE_GRACE_MS`] de plus.
/// Sous LibreOffice, le bac à sable du processus de rendu laisse lancer son binaire, et lui seul (t-912).
pub(crate) fn witness_worker(platform: &Platform, witness: ReferenceWitness) -> WitnessWorker {
    let mut worker = RenderWorker::new(platform.render_launch.clone(), worker_settings(platform, &witness));
    if let ReferenceWitness::Soffice { program, .. } = &witness {
        worker = worker.with_soffice(program.clone());
    }
    WitnessWorker { witness, worker }
}

/// Le témoin LibreOffice, le premier programme de `[import] soffice` qui existe.
///
/// # Errors
///
/// Le programme trouvé est refusé : un script, sur macOS (le bac à sable ne lance que le binaire).
pub fn soffice(platform: &Platform) -> Result<Option<ReferenceWitness>, ReferenceError> {
    let import = &platform.settings.import;
    let path = std::env::var_os("PATH");
    let found = find_soffice(&platform.storage, &import.soffice, path.as_deref())?;
    Ok(found.map(|program| ReferenceWitness::Soffice { program, timeout_ms: import.soffice_timeout_ms }))
}

/// Les bornes du processus de rendu de `witness`.
fn worker_settings(platform: &Platform, witness: &ReferenceWitness) -> RenderSettings {
    let timeout_ms = match witness {
        ReferenceWitness::Office2Pdf => platform.settings.import.reference_timeout_ms,
        ReferenceWitness::Soffice { timeout_ms, .. } => timeout_ms.saturating_add(SOFFICE_GRACE_MS),
    };
    RenderSettings { timeout_ms, ..platform.settings.render.clone() }
}

/// Le témoin dont `metrics` connaît les limites.
///
/// Office2pdf seul (LibreOffice ne marque aucune région suspecte).
pub(crate) fn limits_of(witness: &ReferenceWitness) -> Witness {
    match witness {
        ReferenceWitness::Office2Pdf => Witness::Office2Pdf,
        ReferenceWitness::Soffice { .. } => Witness::Other,
    }
}

/// Le rendu de référence gardé, échec compris.
#[derive(Debug)]
pub(crate) struct Reference {
    /// Nom et version du témoin ; son nom seul s'il a échoué.
    pub(crate) witness: String,
    /// Ses limites connues : les régions suspectes.
    pub(crate) limits: Witness,
    /// Page PNG du témoin, par id de slide Oku.
    ///
    /// Ou la cause de son échec.
    pub(crate) pages: Result<BTreeMap<String, Vec<u8>>, ReferenceCause>,
}

/// La place du rendu de référence d'un import : vide, puis remplie une fois.
pub(crate) type ReferenceSlot = Arc<OnceLock<Reference>>;

/// Le rendu de référence d'un import, fait hors de son état et du verrou.
///
/// Obtenu par [`crate::Session::import_reference_job`] ; se clone sans copier la présentation.
#[derive(Debug, Clone)]
pub struct ReferenceJob {
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) fonts: Fonts,
    /// Id de slide Oku et numéro source, dans l'ordre du rapport.
    pub(crate) slides: Vec<(String, u32)>,
    /// Largeur des pages, en pixels.
    pub(crate) width: u32,
    /// Le témoin choisi à l'import.
    pub(crate) witness: ReferenceWitness,
    /// La place de l'import qui a donné ce travail : la seule qu'il remplit.
    pub(crate) slot: ReferenceSlot,
    /// La langue du journal, où un échec du témoin dit sa cause.
    pub(crate) log: LiveLocalizer,
}

impl ReferenceJob {
    /// Rend les pages par le témoin de l'import, dans son propre processus.
    ///
    /// Bloquant : une seule fois par import, les appels concurrents attendant
    /// le premier ; sans effet si le rendu est déjà fait.
    pub fn run(&self, platform: &Platform) {
        if self.slot.get().is_none() {
            self.run_with(&witness_worker(platform, self.witness.clone()));
        }
    }

    /// Rend les pages manquantes par `renderer`, tranche par tranche.
    ///
    /// Comme [`Self::run`] : le premier appel rend, les autres attendent son
    /// résultat. Un échec est journalisé une fois, quand il survient.
    pub fn run_with(&self, renderer: &dyn WitnessRenderer) {
        self.slot.get_or_init(|| {
            let _span = tracing::info_span!("reference_job", slides = self.slides.len()).entered();
            let limits = renderer.limits();
            match self.pages(renderer) {
                Ok((witness, pages)) => Reference { witness, limits, pages: Ok(pages) },
                Err(failure) => {
                    log_failure(&self.log, &failure, None);
                    Reference { witness: renderer.name().to_owned(), limits, pages: Err(failure.0) }
                }
            }
        });
    }

    /// Le nom du témoin et ses pages par id de slide Oku.
    fn pages(&self, renderer: &dyn WitnessRenderer) -> Result<(String, BTreeMap<String, Vec<u8>>), Failure> {
        let numbers: Vec<u32> = self.slides.iter().map(|(_, n)| *n).collect();
        let mut name = renderer.name().to_owned();
        let mut pages = BTreeMap::new();
        for span in spans(&numbers, SPAN) {
            let rendered = renderer
                .render(&self.bytes, &self.fonts, span, self.width)
                .map_err(|e| render_failure(&e, "témoin"))?;
            let expected = span_len(span);
            if rendered.count != expected || rendered.pages.len() != expected {
                let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
                let cause = ReferenceCause::Pages { reference: count(rendered.count), deck: count(expected) };
                return Err((cause, format!("slides {}..={}", span.first, span.last)));
            }
            name = rendered.witness;
            let mut by_index = rendered.pages;
            for (slide, number) in self.slides.iter().filter(|(_, n)| (span.first..=span.last).contains(n)) {
                let at = (number - span.first) as usize;
                let page = by_index.get_mut(at).ok_or_else(|| (ReferenceCause::Failed, format!("page {at}")))?;
                pages.insert(slide.clone(), std::mem::take(page));
            }
        }
        Ok((name, pages))
    }
}

/// Journalise un échec du témoin ou d'une note.
///
/// Sa cause localisée dans la langue de `log`, son détail.
pub(crate) fn log_failure(log: &LiveLocalizer, (cause, detail): &Failure, slide: Option<&str>) {
    let message = cause.localize(&log.current());
    match slide {
        Some(slide) => tracing::warn!(error = %detail, slide, "{message}"),
        None => tracing::warn!(error = %detail, "{message}"),
    }
}

/// Nombre de slides de `span`, masquées comprises.
pub(crate) fn span_len(span: SlideSpan) -> usize {
    span.last.saturating_sub(span.first) as usize + 1
}

/// Les tranches de numéros source à rendre, chacune couvrant `size` slides source au plus.
///
/// De la première à la dernière : les slides que l'import a omises entre deux
/// comptent dans la tranche, le témoin les rend aussi ; une tranche est donc
/// coupée avant de couvrir plus de `size` numéros, pas seulement `size`
/// slides du rapport. `numbers` : croissants, sans doublon (l'ordre du rapport).
pub(crate) fn spans(numbers: &[u32], size: usize) -> Vec<SlideSpan> {
    debug_assert!(numbers.windows(2).all(|w| w[0] < w[1]), "numéros source non croissants : {numbers:?}");
    let size = u32::try_from(size.max(1)).unwrap_or(u32::MAX);
    let mut out: Vec<SlideSpan> = Vec::new();
    for &n in numbers {
        match out.last_mut() {
            Some(span) if n.saturating_sub(span.first) < size => span.last = n,
            _ => out.push(SlideSpan { first: n, last: n }),
        }
    }
    out
}

/// Un échec : sa cause au rapport et son détail au journal.
pub(crate) type Failure = (ReferenceCause, String);

/// Un échec du processus de rendu, `what` en disant l'objet.
pub(crate) fn render_failure(e: &RenderError, what: &str) -> Failure {
    (cause(e), format!("{what} : {e}"))
}

/// Pourquoi un échec du processus de rendu prive l'import de sa référence.
fn cause(e: &RenderError) -> ReferenceCause {
    match e.kind() {
        RenderErrorKind::Timeout => ReferenceCause::Timeout,
        RenderErrorKind::OutOfMemory => ReferenceCause::Memory,
        _ => ReferenceCause::Failed,
    }
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod reference_tests;
