//! Erreurs du rendu : compilation Typst, export PDF, page absente.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

use crate::diagnostic::Diagnostic;

fn lines(diagnostics: &[Diagnostic]) -> String {
    diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
}

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
    /// L'export d'une image a échoué.
    Image,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum RenderInner {
    #[snafu(display("Typst compilation failed:\n{}", lines(diagnostics)))]
    Compile { diagnostics: Vec<Diagnostic>, backtrace: Backtrace },
    #[snafu(display("PDF export failed:\n{}", lines(diagnostics)))]
    Pdf { diagnostics: Vec<Diagnostic>, backtrace: Backtrace },
    #[snafu(display("no such page: {index}"))]
    NoPage { index: usize, backtrace: Backtrace },
    #[snafu(display("PNG export failed: {message}"))]
    Image { message: String, backtrace: Backtrace },
    #[snafu(display("invalid virtual path {path}"))]
    Path { path: String, source: typst::syntax::PathError, backtrace: Backtrace },
    #[snafu(display("the deck could not be written as JSON"))]
    DeckJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("the base widget registry could not be read"))]
    BaseRegistry { source: Box<RegistryError>, backtrace: Backtrace },
}

impl RenderError {
    pub fn kind(&self) -> RenderErrorKind {
        match self.0 {
            RenderInner::Compile { .. } => RenderErrorKind::Compile,
            RenderInner::Pdf { .. } => RenderErrorKind::Pdf,
            RenderInner::NoPage { .. } => RenderErrorKind::NoPage,
            RenderInner::Image { .. } => RenderErrorKind::Image,
            RenderInner::Path { .. } | RenderInner::DeckJson { .. } | RenderInner::BaseRegistry { .. } => {
                RenderErrorKind::Setup
            }
        }
    }

    /// Messages de Typst (compilation ou export), un par problème.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match &self.0 {
            RenderInner::Compile { diagnostics, .. } | RenderInner::Pdf { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }
}

impl Localize for RenderError {
    /// Le message, puis ceux de Typst, un par ligne.
    fn localize(&self, l: &Localizer) -> String {
        if let RenderInner::BaseRegistry { source, .. } = &self.0 {
            return source.localize(l);
        }
        let mut out = l.tr(match self.kind() {
            RenderErrorKind::Compile => "render-compile",
            RenderErrorKind::Pdf => "render-pdf",
            RenderErrorKind::NoPage => "render-no-page",
            RenderErrorKind::Setup => "render-setup",
            RenderErrorKind::Image => "render-image",
        });
        for d in self.diagnostics() {
            out.push('\n');
            out.push_str(&d.localize(l));
        }
        out
    }
}

/// Le registre des widgets n'a pas pu être lu.
#[derive(Debug, Snafu)]
pub struct RegistryError(pub(crate) RegistryInner);

/// Nature d'une [`RegistryError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryErrorKind {
    /// Identifiant de pack refusé (il deviendrait un chemin).
    BadPackId,
    /// Les widgets ne compilent pas (voir [`RegistryError::diagnostics`]).
    Compile,
    /// La sonde n'a pas émis de registre.
    Missing,
    /// Un widget est mal déclaré ou remplace mal un widget de base.
    Schema,
    Setup,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum RegistryInner {
    #[snafu(display("invalid template pack identifier: {id}"))]
    BadPackId { id: String, backtrace: Backtrace },
    #[snafu(display("widget compilation failed:\n{}", lines(diagnostics)))]
    RegistryCompile { diagnostics: Vec<Diagnostic>, backtrace: Backtrace },
    #[snafu(display("the widget probe emitted no registry"))]
    Missing { backtrace: Backtrace },
    #[snafu(display("invalid widget registry"))]
    Schema { source: Box<oku_slides_core::WidgetError>, backtrace: Backtrace },
    #[snafu(display("the widget probe could not start"))]
    Setup { source: Box<RenderError>, backtrace: Backtrace },
}

impl RegistryError {
    pub fn kind(&self) -> RegistryErrorKind {
        match self.0 {
            RegistryInner::BadPackId { .. } => RegistryErrorKind::BadPackId,
            RegistryInner::RegistryCompile { .. } => RegistryErrorKind::Compile,
            RegistryInner::Missing { .. } => RegistryErrorKind::Missing,
            RegistryInner::Schema { .. } => RegistryErrorKind::Schema,
            RegistryInner::Setup { .. } => RegistryErrorKind::Setup,
        }
    }

    /// Messages de Typst, un par problème, si les widgets ne compilent pas.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match &self.0 {
            RegistryInner::RegistryCompile { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }

    /// Le défaut de déclaration, si un widget est mal déclaré.
    pub fn widget_error(&self) -> Option<&oku_slides_core::WidgetError> {
        match &self.0 {
            RegistryInner::Schema { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl Localize for RegistryError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            RegistryInner::BadPackId { id, .. } => l.tr_with("registry-bad-pack-id", &[("id", id.as_str().into())]),
            RegistryInner::RegistryCompile { diagnostics, .. } => {
                let mut out = l.tr("registry-compile");
                for d in diagnostics {
                    out.push('\n');
                    out.push_str(&d.localize(l));
                }
                out
            }
            RegistryInner::Missing { .. } => l.tr("registry-missing"),
            RegistryInner::Schema { source, .. } => source.localize(l),
            RegistryInner::Setup { source, .. } => source.localize(l),
        }
    }
}
