//! L'écart visuel d'un import hi-fi (`--reference auto`, le défaut en hi-fi).
//!
//! Le témoin rend la présentation dans un processus de rendu à lui
//! ([`RenderWorker::reference`], borné par `[import] reference_timeout_ms`),
//! par tranches de [`SPAN`] slides : sa mémoire ne croît pas avec le deck.
//! Chaque slide Oku, rognée à la page source ([`hifi::source_rect`]), est
//! comparée à sa page de référence à [`REFERENCE_WIDTH`] pixels de large
//! ([`metrics::score`], régions des items source) ; les slides se répondent
//! par leur numéro source. Une référence en échec, d'un autre nombre de pages
//! ou une slide qu'on ne peut noter n'est jamais une erreur : le rapport dit
//! pourquoi (`ReferenceCause`), le journal dit le détail, et aucune slide ne
//! garde de score.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::metrics::{self, Image};
use oku_slides_import::report::{ReferenceCause, ReferenceReport};
use oku_slides_import::source::SourceDeck;
use oku_slides_import::{ImportReport, hifi};
use oku_slides_reference::{Raster, SlideSpan, WITNESS};
use oku_slides_render::{Fonts, RenderError, RenderErrorKind};
use oku_slides_render_worker::RenderWorker;

use crate::pack_render::Rendered;

/// Largeur, en pixels, de la page source comparée (spec : 960 px).
pub(crate) const REFERENCE_WIDTH: f64 = 960.0;

/// Slides rendues par le témoin en une requête, au plus.
pub(crate) const SPAN: usize = 20;

/// Ce qu'il faut pour mesurer l'écart d'un import hi-fi.
pub(crate) struct Scoring<'a> {
    /// Les octets du `.pptx`.
    pub(crate) pptx: &'a [u8],
    /// Sa source lue.
    pub(crate) source: &'a SourceDeck,
    /// Les polices du rendu Oku (celles de l'app et celles du deck), données au témoin.
    pub(crate) fonts: &'a Fonts,
    /// Le processus de rendu du témoin, distinct de celui du rendu Oku.
    pub(crate) worker: &'a RenderWorker,
    /// Le seuil du rapport (`[import] score_threshold`).
    pub(crate) threshold: f64,
    /// La langue du journal.
    pub(crate) l: &'a Localizer,
}

/// Un échec : sa cause au rapport et son détail au journal.
type Failure = (ReferenceCause, String);

/// Pose dans `report` la référence et l'écart de chaque slide de `doc`.
pub(crate) fn score(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport) {
    let (witness, unavailable) = match scores(s, doc, report) {
        Ok(witness) => (witness, None),
        Err((cause, detail)) => {
            tracing::warn!(error = %detail, "{}", cause.localize(s.l));
            for slide in &mut report.slides {
                slide.score = None;
            }
            (WITNESS.to_owned(), Some(cause))
        }
    };
    report.reference = Some(ReferenceReport { witness, threshold: s.threshold, unavailable });
}

/// Les tranches de numéros source à rendre : [`SPAN`] slides du rapport au
/// plus chacune, de la première à la dernière (les slides que l'import a
/// omises entre deux comptent dans la tranche, le témoin les rend aussi).
pub(crate) fn spans(numbers: &[u32], size: usize) -> Vec<SlideSpan> {
    numbers.chunks(size.max(1)).filter_map(|c| Some(SlideSpan { first: *c.first()?, last: *c.last()? })).collect()
}

/// L'écart de chaque slide, tranche par tranche ; le nom du témoin.
fn scores(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport) -> Result<String, Failure> {
    let rect = hifi::source_rect(s.source);
    // Pixels par pt de la page Oku, pour que la page source y fasse REFERENCE_WIDTH pixels.
    let ppp = REFERENCE_WIDTH / rect.w;
    let px = |v: f64| (v * ppp).round().max(0.0) as u32;
    let width = px(rect.w);
    let scale = f64::from(width) / s.source.size.w;
    let numbers: Vec<u32> = report.slides.iter().map(|r| r.source).collect();
    let mut witness = WITNESS.to_owned();
    for span in spans(&numbers, SPAN) {
        let reference = s.worker.reference(s.pptx, s.fonts, Some(span)).map_err(|e| render(&e, "témoin"))?;
        let expected = (span.last - span.first + 1) as usize;
        if reference.page_count() != expected {
            let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
            let cause = ReferenceCause::Pages { reference: count(reference.page_count()), deck: count(expected) };
            return Err((cause, format!("slides {}..={}", span.first, span.last)));
        }
        witness = reference.witness().to_owned();
        for slide in report.slides.iter_mut().filter(|r| (span.first..=span.last).contains(&r.source)) {
            let failed = |what: &str| (ReferenceCause::Failed, format!("slide {} : {what}", slide.source));
            let src = s.source.slides.iter().find(|x| x.number == slide.source).ok_or_else(|| failed("source"))?;
            let page = doc.layout().page_of(&slide.slide).and_then(|p| p.checked_sub(1));
            let page = page.ok_or_else(|| failed("page Oku absente du rendu"))?;
            let oku = doc.png(page, ppp as f32).map_err(|e| render(&e, "page Oku"))?;
            let oku = Raster::from_png(&oku).map_err(|e| (ReferenceCause::Failed, format!("page Oku : {e}")))?;
            let at = (slide.source - span.first) as usize;
            let refr = reference.png(at, width).map_err(|e| render(&e, "page du témoin"))?;
            let refr =
                Raster::from_png(&refr).map_err(|e| (ReferenceCause::Failed, format!("page du témoin : {e}")))?;
            // Rognées à leur taille commune : un arrondi d'un pixel ne prive pas la slide de son score.
            let w = width.min(refr.width()).min(oku.width().saturating_sub(px(rect.x)));
            let h = px(rect.h).min(refr.height()).min(oku.height().saturating_sub(px(rect.y)));
            let (oku, refr) = (oku.crop(px(rect.x), px(rect.y), w, h), refr.crop(0, 0, w, h));
            let scored = metrics::score(image(&refr), image(&oku), &metrics::regions(src, scale));
            slide.score = Some(scored.ok_or_else(|| failed("images vides"))?);
        }
    }
    Ok(witness)
}

/// La vue de `r` que lit [`metrics::score`].
fn image(r: &Raster) -> Image<'_> {
    Image { width: r.width(), height: r.height(), rgba: r.rgba() }
}

/// Un échec du processus de rendu, `what` en disant l'objet.
fn render(e: &RenderError, what: &str) -> Failure {
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
#[path = "import_reference_tests.rs"]
mod import_reference_tests;
