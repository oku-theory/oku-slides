//! Erreurs de l'import d'une présentation : lecture, cible, sonde du pack,
//! application du plan. Messages localisés (`session-import-*`).

use std::path::PathBuf;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::storage::StorageError;
use snafu::{Backtrace, Snafu};

/// Un import qui n'a pas abouti ; rien n'a été écrit.
#[derive(Debug, Snafu)]
pub struct ImportError(Box<ImportInner>);

/// Nature d'une [`ImportError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportErrorKind {
    /// Le `.pptx`, ou un pack lu dans un dossier (CLI), ne se lit pas.
    File,
    /// Le `.pptx` ou un pack lu dans un dossier est trop gros.
    ///
    /// Le `.pptx` dépasse `max_input_bytes`, ou le pack le budget des sources.
    TooLarge,
    /// Le lecteur refuse la présentation ou panique, ou le plan dépasse son budget.
    Presentation,
    /// `[import] score_threshold` n'est pas un nombre fini positif.
    Threshold,
    /// Pack cible introuvable.
    UnknownPack,
    /// Pack cible du retrofit qui n'est pas un pack de layouts.
    NotLayoutPack,
    /// Pack cible illisible : `widgets.json` invalide, ou un widget qui n'est pas du texte.
    BadPack,
    /// Thème cible introuvable.
    UnknownTheme,
    /// Thème cible dont les jetons ne se lisent pas.
    BadTheme,
    /// Le projecteur n'a produit aucun pack pour le deck : un bogue.
    Generated,
    /// Le pack généré se sonde mal : autrement que déclaré, hors bornes, en échec.
    Probe,
    /// Pack ou thème par défaut impossible à adopter, ou fermeture d'un thème introuvable.
    Library,
    /// Widgets de base illisibles.
    Render,
    /// Le plan n'a pas pu s'appliquer aux sources.
    Apply,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ImportInner {
    #[snafu(display("cannot read {}", path.display()))]
    Read { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("cannot start the thread reading {}", path.display()))]
    ReadThread { path: PathBuf, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("presentation too large"))]
    TooLarge { backtrace: Backtrace },
    #[snafu(display("presentation refused"))]
    Presentation { source: oku_slides_import::ImportError, backtrace: Backtrace },
    #[snafu(display("the reader panicked on {}", path.display()))]
    ReaderPanic { path: PathBuf, backtrace: Backtrace },
    #[snafu(display("invalid score_threshold: {value}"))]
    Threshold { value: f64, backtrace: Backtrace },
    #[snafu(display("unknown pack: {pack}"))]
    UnknownPack { pack: String, backtrace: Backtrace },
    #[snafu(display("not a layout pack: {pack}"))]
    NotLayoutPack { pack: String, backtrace: Backtrace },
    #[snafu(display("unreadable pack: {pack}"))]
    BadPack { pack: String, source: oku_slides_core::PackManifestError, backtrace: Backtrace },
    #[snafu(display("unreadable pack files: {pack}"))]
    BadPackFiles { pack: String, backtrace: Backtrace },
    #[snafu(display("cannot read the pack file {}", path.display()))]
    PackDir { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("pack {} larger than {max} bytes", path.display()))]
    PackTooLarge { path: PathBuf, max: u64, backtrace: Backtrace },
    #[snafu(display("unknown theme: {theme}"))]
    UnknownTheme { theme: String, backtrace: Backtrace },
    #[snafu(display("unreadable target theme tokens"))]
    BadTheme { source: crate::SessionError, backtrace: Backtrace },
    #[snafu(display("the importer produced no pack"))]
    PackMissing { backtrace: Backtrace },
    #[snafu(display("generated pack probed differently: {widget}"))]
    ProbeMismatch { widget: String, backtrace: Backtrace },
    #[snafu(display("generated pack beyond the probe limits: {widget}"))]
    ProbeLimit { widget: String, backtrace: Backtrace },
    #[snafu(display("generated pack probe failed: {error}"))]
    ProbeFailed { error: String, backtrace: Backtrace },
    #[snafu(display("default pack or theme unavailable"))]
    Library { source: crate::SessionError, backtrace: Backtrace },
    #[snafu(display("widgets unavailable"))]
    Registry { source: oku_slides_render::RegistryError, backtrace: Backtrace },
    #[snafu(display("unreadable sources"))]
    Sources { source: oku_slides_core::SourcesError, backtrace: Backtrace },
    #[snafu(display("import plan refused"))]
    Op { source: oku_slides_core::TransactionError, backtrace: Backtrace },
}

impl ImportError {
    /// Nature de l'erreur, pour choisir quoi en dire.
    pub fn kind(&self) -> ImportErrorKind {
        use ImportInner::*;
        match *self.0 {
            Read { .. } | ReadThread { .. } | PackDir { .. } => ImportErrorKind::File,
            TooLarge { .. } | PackTooLarge { .. } => ImportErrorKind::TooLarge,
            Presentation { .. } | ReaderPanic { .. } => ImportErrorKind::Presentation,
            Threshold { .. } => ImportErrorKind::Threshold,
            UnknownPack { .. } => ImportErrorKind::UnknownPack,
            NotLayoutPack { .. } => ImportErrorKind::NotLayoutPack,
            BadPack { .. } | BadPackFiles { .. } => ImportErrorKind::BadPack,
            UnknownTheme { .. } => ImportErrorKind::UnknownTheme,
            BadTheme { .. } => ImportErrorKind::BadTheme,
            PackMissing { .. } => ImportErrorKind::Generated,
            ProbeMismatch { .. } | ProbeLimit { .. } | ProbeFailed { .. } => ImportErrorKind::Probe,
            Library { .. } => ImportErrorKind::Library,
            Registry { .. } => ImportErrorKind::Render,
            Sources { .. } | Op { .. } => ImportErrorKind::Apply,
        }
    }

    /// L'erreur et ses causes, sur une ligne.
    pub fn one_line(&self) -> String {
        oku_slides_platform::errors::one_line(self)
    }
}

/// `bytes` en Mio entiers, arrondis vers le haut.
///
/// La borne dite ne passe jamais sous la vraie.
fn mib(bytes: u64) -> u64 {
    bytes.div_ceil(1 << 20)
}

/// Le résultat d'une étape de l'import.
pub(crate) type Result<T, E = ImportError> = std::result::Result<T, E>;

impl Localize for ImportError {
    /// Le message, ou celui de l'erreur d'en dessous s'il parle à l'utilisateur.
    fn localize(&self, l: &Localizer) -> String {
        use ImportInner::*;
        match &*self.0 {
            Read { path, .. } | ReadThread { path, .. } | PackDir { path, .. } => {
                l.tr_with("session-read", &[("path", path.display().to_string().into())])
            }
            TooLarge { .. } => l.tr("import-too-large"),
            Presentation { source, .. } => source.localize(l),
            ReaderPanic { path, .. } => {
                l.tr_with("session-import-reader-panic", &[("path", path.display().to_string().into())])
            }
            Threshold { value, .. } => {
                l.tr_with("session-import-bad-threshold", &[("value", value.to_string().into())])
            }
            UnknownPack { pack, .. } => l.tr_with("session-import-unknown-pack", &[("pack", pack.as_str().into())]),
            NotLayoutPack { pack, .. } => l.tr_with("import-not-layout-pack", &[("pack", pack.as_str().into())]),
            BadPack { pack, .. } | BadPackFiles { pack, .. } => {
                l.tr_with("session-import-bad-pack", &[("pack", pack.as_str().into())])
            }
            PackTooLarge { path, max, .. } => {
                let args = [("path", path.display().to_string().into()), ("max", mib(*max).into())];
                l.tr_with("session-import-pack-too-large", &args)
            }
            UnknownTheme { theme, .. } => l.tr_with("session-unknown-theme", &[("theme", theme.as_str().into())]),
            BadTheme { .. } => l.tr("session-import-bad-theme"),
            PackMissing { .. } => l.tr("session-import-pack-missing"),
            ProbeMismatch { widget, .. } => {
                l.tr_with("session-import-probe-mismatch", &[("widget", widget.as_str().into())])
            }
            ProbeLimit { widget, .. } => l.tr_with("session-import-probe-limit", &[("widget", widget.as_str().into())]),
            ProbeFailed { error, .. } => l.tr_with("session-import-probe-failed", &[("error", error.as_str().into())]),
            Library { source, .. } => source.localize(l),
            Registry { source, .. } => source.localize(l),
            Sources { source, .. } => source.localize(l),
            Op { source, .. } => source.localize(l),
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
