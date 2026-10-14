//! Erreurs fatales de l'import ; tout le reste devient `Opaque`, une part omise ou un `Warning`.

use std::string::FromUtf8Error;

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, IntoError, Snafu};

use crate::collect::clip;

/// La présentation ne peut pas être importée du tout.
#[derive(Debug, Snafu)]
pub struct ImportError(Box<Inner>);

/// Nature d'une [`ImportError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportErrorKind {
    /// Paquet, part essentielle ou XML illisible.
    Unreadable,
    /// Un paquet OOXML, mais pas une présentation.
    NotAPresentation,
    /// Fichier composé (chiffré, ou `.ppt` 97-2003).
    Protected,
    /// Une limite de [`crate::ReadLimits`] est dépassée.
    TooLarge,
}

impl ImportError {
    /// Nature de l'erreur, pour choisir le message affiché.
    pub fn kind(&self) -> ImportErrorKind {
        match *self.0 {
            Inner::Zip { .. } | Inner::Io { .. } | Inner::Utf8 { .. } | Inner::Xml { .. } | Inner::Structure { .. } => {
                ImportErrorKind::Unreadable
            }
            Inner::NotAPresentation { .. } => ImportErrorKind::NotAPresentation,
            Inner::Protected { .. } => ImportErrorKind::Protected,
            Inner::TooLarge { .. } | Inner::OutputTooLarge { .. } => ImportErrorKind::TooLarge,
        }
    }
}

/// Clé Fluent du message d'une nature d'erreur.
pub(crate) fn message_id(kind: ImportErrorKind) -> &'static str {
    match kind {
        ImportErrorKind::Unreadable => "import-unreadable",
        ImportErrorKind::NotAPresentation => "import-not-a-presentation",
        ImportErrorKind::Protected => "import-protected",
        ImportErrorKind::TooLarge => "import-too-large",
    }
}

/// Clé Fluent d'une sortie au-delà de [`crate::ReadLimits::max_output_bytes`].
pub(crate) const OUTPUT_TOO_LARGE: &str = "import-output-too-large";

impl Localize for ImportError {
    fn localize(&self, l: &Localizer) -> String {
        match *self.0 {
            Inner::OutputTooLarge { .. } => l.tr(OUTPUT_TOO_LARGE),
            _ => l.tr(message_id(self.kind())),
        }
    }
}

#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("unreadable zip entry {part}"))]
    Zip { part: String, source: zip::result::ZipError, backtrace: Backtrace },
    #[snafu(display("could not read {part}"))]
    Io { part: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("{part} is not UTF-8"))]
    Utf8 { part: String, source: FromUtf8Error, backtrace: Backtrace },
    #[snafu(display("malformed XML in {part}"))]
    Xml { part: String, source: roxmltree::Error, backtrace: Backtrace },
    #[snafu(display("unexpected structure: {what}"))]
    Structure { what: String, backtrace: Backtrace },
    #[snafu(display("not a presentation: {content_type}"))]
    NotAPresentation { content_type: String, backtrace: Backtrace },
    #[snafu(display("protected file (encrypted or legacy .ppt)"))]
    Protected { backtrace: Backtrace },
    #[snafu(display("too large: {what}"))]
    TooLarge { what: String, backtrace: Backtrace },
    #[snafu(display("import output exceeds {max} bytes"))]
    OutputTooLarge { max: u64, backtrace: Backtrace },
}

// Constructeurs plutôt que `.context()` : `clip` n'est appelé que sur le chemin d'erreur, et
// l'erreur reste boxée (`clippy::result_large_err`).

pub(crate) fn zip_error(part: &str, source: zip::result::ZipError) -> ImportError {
    ImportError(Box::new(ZipSnafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn io_error(part: &str, source: std::io::Error) -> ImportError {
    ImportError(Box::new(IoSnafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn utf8_error(part: &str, source: FromUtf8Error) -> ImportError {
    ImportError(Box::new(Utf8Snafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn xml_error(part: &str, source: roxmltree::Error) -> ImportError {
    if matches!(source, roxmltree::Error::NodesLimitReached) {
        return too_large(&format!("{part}: XML nodes"));
    }
    ImportError(Box::new(XmlSnafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn structure(what: &str) -> ImportError {
    ImportError(Box::new(StructureSnafu { what: clip(what) }.build()))
}

pub(crate) fn too_large(what: &str) -> ImportError {
    ImportError(Box::new(TooLargeSnafu { what: clip(what) }.build()))
}

pub(crate) fn output_too_large(max: u64) -> ImportError {
    ImportError(Box::new(OutputTooLargeSnafu { max }.build()))
}

pub(crate) fn not_a_presentation(content_type: &str) -> ImportError {
    ImportError(Box::new(NotAPresentationSnafu { content_type: clip(content_type) }.build()))
}

pub(crate) fn protected() -> ImportError {
    ImportError(Box::new(ProtectedSnafu.build()))
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
