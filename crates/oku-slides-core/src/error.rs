//! Erreurs du cœur : opérations refusées, transactions, zip des sources,
//! replica Loro.
//!
//! Chaque erreur publique est opaque ; [`OpError::kind`] et
//! [`SourcesError::kind`] disent de quoi il s'agit sans exposer les détails.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, IntoError, Snafu};

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
    /// Options ou données de graphique refusées par `oku-slides-chart`.
    InvalidChart,
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
    Chart { element: String, source: oku_slides_chart::ChartError, backtrace: Backtrace },
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
            OpInner::IndexOutOfRange { .. } => None,
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
    /// `deck.loro` manque.
    MissingDeck,
    /// Le zip décompressé dépasse la taille permise.
    TooLarge,
    /// Le replica (`deck.loro`) est illisible, trop gros, ou ne s'écrit pas.
    Replica(ReplicaErrorKind),
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
    #[snafu(display("deck.loro is missing"))]
    MissingDeck { backtrace: Backtrace },
    #[snafu(display("sources too large once decompressed (max {max} bytes)"))]
    TotalTooLarge { max: usize, backtrace: Backtrace },
    /// En boîte : l'erreur de replica porte déjà sa trace, et gonflerait
    /// chaque `Result` du zip.
    #[snafu(display("unusable deck.loro"))]
    Replica {
        #[snafu(source(from(ReplicaError, Box::new)))]
        source: Box<ReplicaError>,
        backtrace: Backtrace,
    },
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
            SourcesInner::TotalTooLarge { .. } => SourcesErrorKind::TooLarge,
            SourcesInner::Replica { ref source, .. } => SourcesErrorKind::Replica(source.kind()),
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
        if let SourcesInner::Replica { source, .. } = &self.0 {
            return source.localize(l);
        }
        l.tr(match self.kind() {
            SourcesErrorKind::Unreadable => "sources-unreadable",
            SourcesErrorKind::NotSources => "sources-not-sources",
            SourcesErrorKind::UnsupportedVersion => "sources-unsupported-version",
            SourcesErrorKind::BadName => "sources-bad-name",
            SourcesErrorKind::MissingDeck => "sources-missing-deck",
            SourcesErrorKind::TooLarge => "sources-too-large",
            // Traité plus haut : l'erreur de replica se localise elle-même.
            SourcesErrorKind::Replica(_) => "sources-unreadable",
        })
    }
}

/// Le replica Loro du deck ne se lit pas, ou ne s'écrit pas.
#[derive(Debug, Snafu)]
pub struct ReplicaError(ReplicaInner);

/// Nature d'une [`ReplicaError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplicaErrorKind {
    /// Snapshot illisible, ou qui n'est pas un deck Oku Slides.
    Corrupted,
    /// Snapshot au-delà de la taille permise.
    TooLarge,
    /// Écriture refusée par Loro.
    Write,
    /// Deux replicas sans origine commune.
    Unrelated,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ReplicaInner {
    #[snafu(display("unreadable replica snapshot"))]
    Import { source: loro::LoroError, backtrace: Backtrace },
    #[snafu(display("the replica snapshot is not an Oku Slides deck: {what}"))]
    NotADeck { what: &'static str, backtrace: Backtrace },
    #[snafu(display("replica import panicked"))]
    ImportPanicked { backtrace: Backtrace },
    #[snafu(display("replica snapshot too large: {size} bytes (max {max})"))]
    TooLarge { size: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("replica write failed"))]
    Loro { source: loro::LoroError, backtrace: Backtrace },
    #[snafu(display("replica text update timed out"))]
    TextUpdate { source: loro::UpdateTimeoutError, backtrace: Backtrace },
    #[snafu(display("replica export failed"))]
    Export { source: loro::LoroEncodeError, backtrace: Backtrace },
    #[snafu(display("deck (de)serialization failed"))]
    DeckJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("the two replicas have no common origin"))]
    Unrelated { backtrace: Backtrace },
}

impl ReplicaError {
    pub fn kind(&self) -> ReplicaErrorKind {
        match self.0 {
            ReplicaInner::Import { .. } | ReplicaInner::NotADeck { .. } | ReplicaInner::ImportPanicked { .. } => {
                ReplicaErrorKind::Corrupted
            }
            ReplicaInner::TooLarge { .. } => ReplicaErrorKind::TooLarge,
            ReplicaInner::Unrelated { .. } => ReplicaErrorKind::Unrelated,
            ReplicaInner::Loro { .. }
            | ReplicaInner::TextUpdate { .. }
            | ReplicaInner::Export { .. }
            | ReplicaInner::DeckJson { .. } => ReplicaErrorKind::Write,
        }
    }
}

impl Localize for ReplicaError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(match self.kind() {
            ReplicaErrorKind::Corrupted => "replica-corrupted",
            ReplicaErrorKind::TooLarge => "replica-too-large",
            ReplicaErrorKind::Write => "replica-write",
            ReplicaErrorKind::Unrelated => "replica-unrelated",
        })
    }
}

/// Le registre des widgets est mal déclaré.
#[derive(Debug, Snafu)]
pub struct WidgetError(WidgetInner);

/// Défaut d'une déclaration de widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaFault {
    EmptyName,
    DuplicateWidget,
    DuplicateParam,
    EmptyChoice,
    DefaultNotInChoice,
    /// La valeur par défaut d'un paramètre ne correspond pas à son type
    /// (ou, pour un nombre, sort de `min`/`max`).
    DefaultWrongType,
    /// Seul un paramètre `plain` se lie à `deck.params`.
    BindNotPlain,
    /// Un slot accepte un widget absent, ou qui n'est pas un élément.
    UnknownSlotKind,
}

impl std::fmt::Display for SchemaFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SchemaFault::EmptyName => "empty name",
            SchemaFault::DuplicateWidget => "duplicate widget",
            SchemaFault::DuplicateParam => "duplicate parameter",
            SchemaFault::EmptyChoice => "empty choice",
            SchemaFault::DefaultNotInChoice => "default not in choice",
            SchemaFault::DefaultWrongType => "default wrong type",
            SchemaFault::BindNotPlain => "bind not plain",
            SchemaFault::UnknownSlotKind => "unknown slot kind",
        })
    }
}

impl SchemaFault {
    fn message_id(self) -> &'static str {
        match self {
            SchemaFault::EmptyName => "widget-fault-empty-name",
            SchemaFault::DuplicateWidget => "widget-fault-duplicate-widget",
            SchemaFault::DuplicateParam => "widget-fault-duplicate-param",
            SchemaFault::EmptyChoice => "widget-fault-empty-choice",
            SchemaFault::DefaultNotInChoice => "widget-fault-default-not-in-choice",
            SchemaFault::DefaultWrongType => "widget-fault-default-wrong-type",
            SchemaFault::BindNotPlain => "widget-fault-bind-not-plain",
            SchemaFault::UnknownSlotKind => "widget-fault-unknown-slot-kind",
        }
    }
}

/// Nature d'une [`WidgetError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WidgetErrorKind {
    Unreadable,
    BadSchema(SchemaFault),
    /// Un pack remplace un widget de base en retirant ou retypant un paramètre.
    IncompatibleOverride,
    /// Un pack remplace un widget de base par un widget d'une autre catégorie.
    IncompatibleCategory,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum WidgetInner {
    #[snafu(display("unreadable widget registry"))]
    Unreadable { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("widget {widget}: {fault}"))]
    BadSchema { widget: String, fault: SchemaFault, backtrace: Backtrace },
    #[snafu(display("widget {widget} overrides parameter {param} incompatibly"))]
    IncompatibleOverride { widget: String, param: String, backtrace: Backtrace },
    #[snafu(display("widget {widget} overrides the base widget with another category"))]
    IncompatibleCategory { widget: String, backtrace: Backtrace },
}

impl WidgetError {
    /// Construit une [`WidgetError::kind`] `Unreadable` à partir d'une défaillance
    /// JSON survenue hors du cœur (p. ex. la sonde de rendu ne peut pas
    /// convertir la valeur Typst en JSON).
    pub fn unreadable(source: serde_json::Error) -> Self {
        UnreadableSnafu.into_error(source).into()
    }

    pub fn kind(&self) -> WidgetErrorKind {
        match &self.0 {
            WidgetInner::Unreadable { .. } => WidgetErrorKind::Unreadable,
            WidgetInner::BadSchema { fault, .. } => WidgetErrorKind::BadSchema(*fault),
            WidgetInner::IncompatibleOverride { .. } => WidgetErrorKind::IncompatibleOverride,
            WidgetInner::IncompatibleCategory { .. } => WidgetErrorKind::IncompatibleCategory,
        }
    }
}

impl Localize for WidgetError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            WidgetInner::Unreadable { .. } => l.tr("widget-unreadable"),
            WidgetInner::BadSchema { widget, fault, .. } => {
                l.tr_with(fault.message_id(), &[("widget", widget.as_str().into())])
            }
            WidgetInner::IncompatibleOverride { widget, param, .. } => l.tr_with(
                "widget-incompatible-override",
                &[("widget", widget.as_str().into()), ("param", param.as_str().into())],
            ),
            WidgetInner::IncompatibleCategory { widget, .. } => {
                l.tr_with("widget-incompatible-category", &[("widget", widget.as_str().into())])
            }
        }
    }
}
