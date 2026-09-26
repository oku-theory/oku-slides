//! Erreurs du cœur : opérations refusées, transactions, zip des sources.
//!
//! Chaque erreur publique est opaque ; [`OpError::kind`] et
//! [`SourcesError::kind`] disent de quoi il s'agit sans exposer les détails.

use slider_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// Une opération refusée. Le deck n'a pas changé.
#[derive(Debug, Snafu)]
pub struct OpError(Box<OpInner>);

/// Nature d'une [`OpError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpErrorKind {
    SlideNotFound,
    ElementNotFound,
    DuplicateId,
    IndexOutOfRange,
    NotAGroup,
    NoText,
    InvalidValue,
    NotAChart,
    /// Options ou données de graphique refusées par `slider-chart`.
    InvalidChart,
    /// Un annuler n'a pas pu défaire une op : l'historique ne correspond plus au deck.
    Inconsistent,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum OpInner {
    #[snafu(display("slide not found: {slide}"))]
    SlideNotFound { slide: String, backtrace: Backtrace },
    #[snafu(display("element not found: {element} (slide {slide})"))]
    ElementNotFound { slide: String, element: String, backtrace: Backtrace },
    #[snafu(display("identifier already used: {id}"))]
    DuplicateId { id: String, backtrace: Backtrace },
    #[snafu(display("index out of range: {index} (length {len})"))]
    IndexOutOfRange { index: usize, len: usize, backtrace: Backtrace },
    #[snafu(display("element {element} is not a group"))]
    NotAGroup { element: String, backtrace: Backtrace },
    #[snafu(display("element {element} has no text"))]
    NoText { element: String, backtrace: Backtrace },
    #[snafu(display("invalid value for {field}: {reason}"))]
    InvalidValue { field: String, reason: &'static str, backtrace: Backtrace },
    #[snafu(display("invalid value for {field}"))]
    ParseValue { field: String, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("element {element} is not a chart"))]
    NotAChart { element: String, backtrace: Backtrace },
    #[snafu(display("chart {element} refused the change"))]
    Chart { element: String, source: slider_chart::ChartError, backtrace: Backtrace },
    #[snafu(display("history does not match the deck"))]
    Inconsistent { source: Box<OpError>, backtrace: Backtrace },
}

impl OpError {
    pub fn kind(&self) -> OpErrorKind {
        match *self.0 {
            OpInner::SlideNotFound { .. } => OpErrorKind::SlideNotFound,
            OpInner::ElementNotFound { .. } => OpErrorKind::ElementNotFound,
            OpInner::DuplicateId { .. } => OpErrorKind::DuplicateId,
            OpInner::IndexOutOfRange { .. } => OpErrorKind::IndexOutOfRange,
            OpInner::NotAGroup { .. } => OpErrorKind::NotAGroup,
            OpInner::NoText { .. } => OpErrorKind::NoText,
            OpInner::InvalidValue { .. } | OpInner::ParseValue { .. } => OpErrorKind::InvalidValue,
            OpInner::NotAChart { .. } => OpErrorKind::NotAChart,
            OpInner::Chart { .. } => OpErrorKind::InvalidChart,
            OpInner::Inconsistent { .. } => OpErrorKind::Inconsistent,
        }
    }

    /// Ce que l'erreur désigne (slide, élément, identifiant ou champ), s'il y en a un.
    pub fn subject(&self) -> Option<&str> {
        match &*self.0 {
            OpInner::SlideNotFound { slide, .. } => Some(slide),
            OpInner::ElementNotFound { element, .. }
            | OpInner::NotAGroup { element, .. }
            | OpInner::NoText { element, .. }
            | OpInner::NotAChart { element, .. }
            | OpInner::Chart { element, .. } => Some(element),
            OpInner::DuplicateId { id, .. } => Some(id),
            OpInner::InvalidValue { field, .. } | OpInner::ParseValue { field, .. } => Some(field),
            OpInner::IndexOutOfRange { .. } | OpInner::Inconsistent { .. } => None,
        }
    }
}

/// Une transaction refusée : l'op `index` a échoué, le deck est revenu à l'état d'avant.
#[derive(Debug, Snafu)]
#[snafu(display("operation {index} refused"), visibility(pub(crate)))]
pub struct TransactionError {
    index: usize,
    source: OpError,
}

impl TransactionError {
    /// Position de l'op refusée dans la transaction.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Pourquoi elle a été refusée.
    pub fn op_error(&self) -> &OpError {
        &self.source
    }
}

/// Un zip de sources illisible ou impossible à écrire.
#[derive(Debug, Snafu)]
pub struct SourcesError(SourcesInner);

/// Nature d'une [`SourcesError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourcesErrorKind {
    /// Le zip ou son contenu est illisible.
    Unreadable,
    /// Ce n'est pas un zip de sources Oku Slides.
    NotSources,
    /// Écrit par une version plus récente.
    UnsupportedVersion,
    /// Nom de fichier refusé (chemin, fichier caché).
    BadName,
    MissingDeck,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum SourcesInner {
    #[snafu(display("unreadable zip"))]
    Zip { source: zip::result::ZipError, backtrace: Backtrace },
    #[snafu(display("i/o on the sources zip"))]
    Io { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("invalid JSON in {file}"))]
    Json { file: String, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("not an Oku Slides sources zip"))]
    NotSources { backtrace: Backtrace },
    #[snafu(display("unsupported format version: {version}"))]
    UnsupportedVersion { version: u32, backtrace: Backtrace },
    #[snafu(display("rejected file name: {name}"))]
    BadName { name: String, backtrace: Backtrace },
    #[snafu(display("deck.json is missing"))]
    MissingDeck { backtrace: Backtrace },
}

impl SourcesError {
    pub fn kind(&self) -> SourcesErrorKind {
        match self.0 {
            SourcesInner::Zip { .. } | SourcesInner::Io { .. } | SourcesInner::Json { .. } => {
                SourcesErrorKind::Unreadable
            }
            SourcesInner::NotSources { .. } => SourcesErrorKind::NotSources,
            SourcesInner::UnsupportedVersion { .. } => SourcesErrorKind::UnsupportedVersion,
            SourcesInner::BadName { .. } => SourcesErrorKind::BadName,
            SourcesInner::MissingDeck { .. } => SourcesErrorKind::MissingDeck,
        }
    }
}

impl Localize for OpError {
    fn localize(&self, l: &Localizer) -> String {
        if let OpInner::Chart { element, source, .. } = &*self.0 {
            return l.tr_with(
                "op-invalid-chart",
                &[("subject", element.as_str().into()), ("reason", source.localize(l).into())],
            );
        }
        let id = match self.kind() {
            OpErrorKind::SlideNotFound => "op-slide-not-found",
            OpErrorKind::ElementNotFound => "op-element-not-found",
            OpErrorKind::DuplicateId => "op-duplicate-id",
            OpErrorKind::IndexOutOfRange => "op-index-out-of-range",
            OpErrorKind::NotAGroup => "op-not-a-group",
            OpErrorKind::NoText => "op-no-text",
            OpErrorKind::InvalidValue => "op-invalid-value",
            OpErrorKind::NotAChart => "op-not-a-chart",
            OpErrorKind::InvalidChart => "op-invalid-chart",
            OpErrorKind::Inconsistent => "op-inconsistent",
        };
        l.tr_with(id, &[("subject", self.subject().unwrap_or_default().into())])
    }
}

impl Localize for TransactionError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr_with("transaction-refused", &[("index", self.index.into()), ("reason", self.source.localize(l).into())])
    }
}

impl Localize for SourcesError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(match self.kind() {
            SourcesErrorKind::Unreadable => "sources-unreadable",
            SourcesErrorKind::NotSources => "sources-not-sources",
            SourcesErrorKind::UnsupportedVersion => "sources-unsupported-version",
            SourcesErrorKind::BadName => "sources-bad-name",
            SourcesErrorKind::MissingDeck => "sources-missing-deck",
        })
    }
}
