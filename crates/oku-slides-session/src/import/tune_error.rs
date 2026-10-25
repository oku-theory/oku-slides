//! Les refus des outils d'ajustement d'un import (`import_report`,
//! `import_diff`), dits à l'agent (`session-import-tune-*`).

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::report::ReferenceCause;
use snafu::{Backtrace, Snafu};

use super::tune::SlideState;

/// Un appel d'ajustement refusé ; l'import et le deck sont intacts.
#[derive(Debug, Snafu)]
pub struct TuneError(TuneInner);

/// Nature d'une [`TuneError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TuneErrorKind {
    /// L'onglet ne montre pas un deck importé (fermé, repris, ou un autre document).
    NoImport,
    /// `import_diff` sur un import retrofit : il n'a pas de référence.
    HifiOnly,
    /// Le rendu de référence manque.
    NoReference,
    /// Le rendu de référence n'est pas encore fait.
    ///
    /// L'appelant n'a pas lancé son travail, ou il tourne encore.
    ReferencePending,
    /// Pas une slide de l'import (inconnue, ou supprimée depuis).
    UnknownSlide,
    /// Pas une région de la slide.
    UnknownRegion,
    /// La slide n'a plus d'essai : tous faits, ou arrêtée sans gain.
    TriesUsed,
    /// La slide n'est pas à ajuster : `done`, `witness` ou `unscored`, elle n'a aucun essai.
    Untunable,
    /// La comparaison de la slide à sa référence a échoué.
    Compare,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum TuneInner {
    #[snafu(display("no import in this tab"))]
    NoImport { backtrace: Backtrace },
    #[snafu(display("a retrofit import has no reference to compare with"))]
    HifiOnly { backtrace: Backtrace },
    #[snafu(display("no reference rendering: {cause}"))]
    NoReference { cause: ReferenceCause, backtrace: Backtrace },
    #[snafu(display("the reference rendering is not done yet"))]
    ReferencePending { backtrace: Backtrace },
    #[snafu(display("{slide} is not a slide of the import"))]
    UnknownSlide { slide: String, backtrace: Backtrace },
    #[snafu(display("{region} is not a region of slide {slide}"))]
    UnknownRegion { slide: String, region: String, backtrace: Backtrace },
    #[snafu(display("no try left for slide {slide} ({tries} used)"))]
    TriesUsed { slide: String, tries: u32, backtrace: Backtrace },
    #[snafu(display("slide {slide} is {} and has no try", state.name()))]
    Untunable { slide: String, state: SlideState, backtrace: Backtrace },
    #[snafu(display("comparison failed: {detail}"))]
    Compare { detail: String, backtrace: Backtrace },
}

impl TuneError {
    /// Nature du refus.
    pub fn kind(&self) -> TuneErrorKind {
        use TuneInner::{
            Compare, HifiOnly, NoImport, NoReference, ReferencePending, TriesUsed, UnknownRegion, UnknownSlide,
            Untunable,
        };
        match self.0 {
            NoImport { .. } => TuneErrorKind::NoImport,
            HifiOnly { .. } => TuneErrorKind::HifiOnly,
            NoReference { .. } => TuneErrorKind::NoReference,
            ReferencePending { .. } => TuneErrorKind::ReferencePending,
            UnknownSlide { .. } => TuneErrorKind::UnknownSlide,
            UnknownRegion { .. } => TuneErrorKind::UnknownRegion,
            TriesUsed { .. } => TuneErrorKind::TriesUsed,
            Untunable { .. } => TuneErrorKind::Untunable,
            Compare { .. } => TuneErrorKind::Compare,
        }
    }
}

impl Localize for TuneError {
    fn localize(&self, l: &Localizer) -> String {
        use TuneInner::{
            Compare, HifiOnly, NoImport, NoReference, ReferencePending, TriesUsed, UnknownRegion, UnknownSlide,
            Untunable,
        };
        match &self.0 {
            NoImport { .. } => l.tr("session-import-tune-none"),
            HifiOnly { .. } => l.tr("session-import-tune-hifi-only"),
            NoReference { cause, .. } => {
                l.tr_with("session-import-tune-no-reference", &[("reason", cause.localize(l).into())])
            }
            ReferencePending { .. } => l.tr("session-import-tune-reference-pending"),
            UnknownSlide { slide, .. } => {
                l.tr_with("session-import-tune-unknown-slide", &[("slide", slide.as_str().into())])
            }
            UnknownRegion { slide, region, .. } => l.tr_with(
                "session-import-tune-unknown-region",
                &[("slide", slide.as_str().into()), ("region", region.as_str().into())],
            ),
            TriesUsed { slide, tries, .. } => l.tr_with(
                "session-import-tune-tries-used",
                &[("slide", slide.as_str().into()), ("tries", (*tries).into())],
            ),
            Untunable { slide, state, .. } => l.tr_with(
                "session-import-tune-untunable",
                &[("slide", slide.as_str().into()), ("state", state.name().into())],
            ),
            Compare { .. } => l.tr("session-import-tune-compare"),
        }
    }
}

#[cfg(test)]
#[path = "tune_error_tests.rs"]
mod tune_error_tests;
