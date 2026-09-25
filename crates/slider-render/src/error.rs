//! Erreurs du rendu : compilation Typst, export PDF, page absente.

use snafu::{Backtrace, Snafu};

/// Un rendu impossible.
#[derive(Debug, Snafu)]
pub struct RenderError(pub(crate) RenderInner);

/// Nature d'une [`RenderError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderErrorKind {
    /// Typst n'a pas pu compiler le deck (voir [`RenderError::diagnostics`]).
    Compile,
    /// L'export PDF a échoué.
    Pdf,
    NoPage,
    /// Le monde Typst n'a pas pu être préparé.
    Setup,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum RenderInner {
    #[snafu(display("Typst compilation failed:\n{}", diagnostics.join("\n")))]
    Compile { diagnostics: Vec<String>, backtrace: Backtrace },
    #[snafu(display("PDF export failed:\n{}", diagnostics.join("\n")))]
    Pdf { diagnostics: Vec<String>, backtrace: Backtrace },
    #[snafu(display("no such page: {index}"))]
    NoPage { index: usize, backtrace: Backtrace },
    #[snafu(display("invalid virtual path {path}"))]
    Path { path: String, source: typst::syntax::PathError, backtrace: Backtrace },
    #[snafu(display("the deck could not be written as JSON"))]
    DeckJson { source: serde_json::Error, backtrace: Backtrace },
}

impl RenderError {
    pub fn kind(&self) -> RenderErrorKind {
        match self.0 {
            RenderInner::Compile { .. } => RenderErrorKind::Compile,
            RenderInner::Pdf { .. } => RenderErrorKind::Pdf,
            RenderInner::NoPage { .. } => RenderErrorKind::NoPage,
            RenderInner::Path { .. } | RenderInner::DeckJson { .. } => RenderErrorKind::Setup,
        }
    }

    /// Messages de Typst (compilation ou export), un par problème.
    pub fn diagnostics(&self) -> &[String] {
        match &self.0 {
            RenderInner::Compile { diagnostics, .. } | RenderInner::Pdf { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }
}
