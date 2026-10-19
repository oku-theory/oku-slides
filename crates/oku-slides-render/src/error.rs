//! Erreurs du rendu : compilation Typst, export PDF, page absente.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, IntoError, Snafu};

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
    /// Le rendu a dépassé `render.timeout_ms`.
    Timeout,
    /// Le processus de rendu a dépassé `render.memory_mib`.
    OutOfMemory,
    /// Le processus de rendu s'est arrêté (plantage, panique de Typst).
    WorkerLost,
    /// Un message du processus de rendu est illisible ou trop grand (une page
    /// trop grande pour l'aperçu en SVG compris).
    Frame,
    /// Le rendu demandé a été remplacé (processus relancé).
    Stale,
    /// Les widgets de ce pack ont déjà fait échouer le rendu : ils ne sont plus exécutés.
    PackDisabled,
    /// Le processus de rendu parle une autre version du protocole (autre binaire).
    Protocol,
}

impl RenderErrorKind {
    /// Un échec du processus de rendu (délai, mémoire, arrêt, pack écarté) :
    /// la session se replie sur les widgets de base.
    pub fn is_bounded(self) -> bool {
        matches!(self, Self::Timeout | Self::OutOfMemory | Self::WorkerLost | Self::PackDisabled)
    }
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
    #[snafu(display("the built-in widget registry could not be read"))]
    BaseRegistry { source: Box<RegistryError>, backtrace: Backtrace },
    #[snafu(display("rendering took more than {ms} ms"))]
    Timeout { ms: u64, backtrace: Backtrace },
    #[snafu(display("rendering used more than {mib} MiB"))]
    OutOfMemory { mib: u64, backtrace: Backtrace },
    #[snafu(display("the rendering process stopped: {message}"))]
    WorkerLost { message: String, backtrace: Backtrace },
    #[snafu(display("unreadable message from the rendering process: {message}"))]
    Frame { message: String, backtrace: Backtrace },
    #[snafu(display("the SVG of the page exceeds {mib} MiB"))]
    PageTooLarge { mib: u64, backtrace: Backtrace },
    #[snafu(display("the render was replaced"))]
    Stale { backtrace: Backtrace },
    #[snafu(display("the widgets of pack {pack} are disabled"))]
    PackDisabled { pack: String, backtrace: Backtrace },
    #[snafu(display("rendering failed: {message}"))]
    Remote { message: String, backtrace: Backtrace },
    #[snafu(display("the rendering process speaks protocol {found}, expected {expected}"))]
    Protocol { expected: u32, found: String, backtrace: Backtrace },
}

impl RenderError {
    pub fn kind(&self) -> RenderErrorKind {
        match self.0 {
            RenderInner::Compile { .. } => RenderErrorKind::Compile,
            RenderInner::Pdf { .. } => RenderErrorKind::Pdf,
            RenderInner::NoPage { .. } => RenderErrorKind::NoPage,
            RenderInner::Image { .. } => RenderErrorKind::Image,
            RenderInner::Timeout { .. } => RenderErrorKind::Timeout,
            RenderInner::OutOfMemory { .. } => RenderErrorKind::OutOfMemory,
            RenderInner::WorkerLost { .. } => RenderErrorKind::WorkerLost,
            RenderInner::Frame { .. } | RenderInner::PageTooLarge { .. } => RenderErrorKind::Frame,
            RenderInner::Stale { .. } => RenderErrorKind::Stale,
            RenderInner::PackDisabled { .. } => RenderErrorKind::PackDisabled,
            RenderInner::Protocol { .. } => RenderErrorKind::Protocol,
            RenderInner::Path { .. }
            | RenderInner::DeckJson { .. }
            | RenderInner::BaseRegistry { .. }
            | RenderInner::Remote { .. } => RenderErrorKind::Setup,
        }
    }

    /// Le rendu a dépassé son délai.
    pub fn timeout(ms: u64) -> Self {
        TimeoutSnafu { ms }.build().into()
    }

    /// Le processus de rendu a dépassé sa mémoire.
    pub fn out_of_memory(mib: u64) -> Self {
        OutOfMemorySnafu { mib }.build().into()
    }

    /// Le processus de rendu s'est arrêté.
    pub fn worker_lost(message: impl Into<String>) -> Self {
        WorkerLostSnafu { message: message.into() }.build().into()
    }

    /// Un message du processus de rendu est illisible.
    pub fn frame(message: impl Into<String>) -> Self {
        FrameSnafu { message: message.into() }.build().into()
    }

    /// Le SVG d'une page dépasse sa borne (`mib` Mio) : trop grande pour
    /// l'aperçu, elle s'exporte encore en PDF ou en PNG.
    pub fn page_too_large(mib: u64) -> Self {
        PageTooLargeSnafu { mib }.build().into()
    }

    /// Le rendu demandé a été remplacé.
    pub fn stale() -> Self {
        StaleSnafu.build().into()
    }

    /// Les widgets de ce pack sont écartés.
    pub fn pack_disabled(pack: impl Into<String>) -> Self {
        PackDisabledSnafu { pack: pack.into() }.build().into()
    }

    /// Le processus de rendu parle le protocole `found` (`None` : il ne le dit
    /// pas), quand l'hôte attend `expected`.
    pub fn protocol(expected: u32, found: Option<u32>) -> Self {
        let found = found.map_or_else(|| "unknown".to_owned(), |v| v.to_string());
        ProtocolSnafu { expected, found }.build().into()
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
        let key = match &self.0 {
            RenderInner::BaseRegistry { source, .. } => return source.localize(l),
            RenderInner::Timeout { ms, .. } => {
                let seconds = (ms / 1000).max(1);
                return l.tr_with("render-timeout", &[("seconds", seconds.into())]);
            }
            RenderInner::OutOfMemory { mib, .. } => {
                return l.tr_with("render-out-of-memory", &[("mib", (*mib).into())]);
            }
            RenderInner::PackDisabled { pack, .. } => {
                return l.tr_with("render-pack-disabled", &[("pack", pack.as_str().into())]);
            }
            RenderInner::PageTooLarge { mib, .. } => {
                return l.tr_with("render-page-too-large", &[("mib", (*mib).into())]);
            }
            RenderInner::WorkerLost { .. } => "render-worker-lost",
            RenderInner::Frame { .. } => "render-frame",
            RenderInner::Stale { .. } => "render-stale",
            RenderInner::Protocol { .. } => "render-protocol",
            RenderInner::Compile { .. } => "render-compile",
            RenderInner::Pdf { .. } => "render-pdf",
            RenderInner::NoPage { .. } => "render-no-page",
            RenderInner::Image { .. } => "render-image",
            RenderInner::Remote { .. } => "render-remote",
            RenderInner::Path { .. } | RenderInner::DeckJson { .. } => "render-setup",
        };
        let mut out = l.tr(key);
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
    /// Deux packs d'un ensemble partagent un espace de noms ou un identifiant.
    DuplicatePack,
    /// Les widgets ne compilent pas (voir [`RegistryError::diagnostics`]).
    Compile,
    /// La sonde n'a pas émis de registre.
    Missing,
    /// Un widget est mal déclaré ou remplace mal un widget de base.
    Schema,
    Setup,
    /// Le pack `base` intégré est illisible (binaire abîmé).
    BasePack,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum RegistryInner {
    #[snafu(display("invalid template pack identifier: {id}"))]
    BadPackId { id: String, backtrace: Backtrace },
    #[snafu(display("two widget packs share {pack}"))]
    DuplicatePack { pack: String, backtrace: Backtrace },
    #[snafu(display("widget compilation failed:\n{}", lines(diagnostics)))]
    RegistryCompile { diagnostics: Vec<Diagnostic>, backtrace: Backtrace },
    #[snafu(display("the widget probe emitted no registry"))]
    Missing { backtrace: Backtrace },
    #[snafu(display("invalid widget registry"))]
    Schema { source: Box<oku_slides_core::WidgetError>, backtrace: Backtrace },
    #[snafu(display("the widget probe could not start"))]
    Setup { source: Box<RenderError>, backtrace: Backtrace },
    #[snafu(display("the widget probe failed"))]
    Worker { source: Box<RenderError>, backtrace: Backtrace },
    #[snafu(display("invalid widget registry: {message}"))]
    RemoteSchema { message: String, backtrace: Backtrace },
    #[snafu(display("built-in base pack unreadable: {file}"))]
    BasePack { file: String, backtrace: Backtrace },
}

impl RegistryError {
    pub fn kind(&self) -> RegistryErrorKind {
        match self.0 {
            RegistryInner::BadPackId { .. } => RegistryErrorKind::BadPackId,
            RegistryInner::DuplicatePack { .. } => RegistryErrorKind::DuplicatePack,
            RegistryInner::RegistryCompile { .. } => RegistryErrorKind::Compile,
            RegistryInner::Missing { .. } => RegistryErrorKind::Missing,
            RegistryInner::Schema { .. } | RegistryInner::RemoteSchema { .. } => RegistryErrorKind::Schema,
            RegistryInner::Setup { .. } | RegistryInner::Worker { .. } => RegistryErrorKind::Setup,
            RegistryInner::BasePack { .. } => RegistryErrorKind::BasePack,
        }
    }

    /// La sonde n'a pas abouti dans le processus de rendu.
    pub fn worker(source: RenderError) -> Self {
        WorkerSnafu.into_error(Box::new(source)).into()
    }

    /// L'erreur de rendu à l'origine de celle-ci (délai, mémoire, arrêt).
    pub fn render_error(&self) -> Option<&RenderError> {
        match &self.0 {
            RegistryInner::Worker { source, .. } => Some(source),
            _ => None,
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
            RegistryInner::DuplicatePack { pack, .. } => {
                l.tr_with("registry-duplicate-pack", &[("pack", pack.as_str().into())])
            }
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
            RegistryInner::Setup { source, .. } | RegistryInner::Worker { source, .. } => source.localize(l),
            RegistryInner::RemoteSchema { message, .. } => {
                l.tr_with("registry-schema-remote", &[("message", message.as_str().into())])
            }
            RegistryInner::BasePack { file, .. } => {
                l.tr_with("registry-base-pack-broken", &[("file", file.as_str().into())])
            }
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
