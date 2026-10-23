//! Erreurs du cœur : opérations refusées, transactions, zip des sources,
//! replica Loro.
//!
//! Chaque erreur publique est opaque ; [`OpError::kind`] et
//! [`SourcesError::kind`] disent de quoi il s'agit sans exposer les détails.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, IntoError, Snafu};

mod reason;
mod token;

pub use reason::ValueReason;
pub(crate) use reason::{ExpectedType, Shape};
pub(crate) use token::{
    EmptyPathSnafu, ManifestJsonSnafu, ManifestTokensSnafu, MissingDefaultSnafu, PathConflictSnafu,
};
pub use token::{PackManifestError, PackManifestErrorKind, TokenEntryError, TokenEntryErrorKind};

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
    InvalidValue,
    /// Options ou données de graphique refusées par `oku-slides-chart`.
    InvalidChart,
    /// Options ou données de tableau refusées par `oku-slides-chart`.
    InvalidTable,
    /// `kind` absent du registre des widgets.
    UnknownWidget,
    /// Paramètre non déclaré par le widget.
    UnknownParam,
    /// Valeur d'un autre type que celui du paramètre.
    ParamType,
    /// Item absent d'une liste.
    ItemNotFound,
    /// Valeur hors de la liste d'un `choice`.
    ChoiceOutOfRange,
    /// Le slot n'accepte pas ce widget (`accepts`).
    SlotRejects,
    /// Widget d'une autre catégorie (slide au lieu d'élément, ou l'inverse).
    WrongCategory,
    /// Une borne de liste refusée (taille, cellule, id) ; voir [`ListBound`].
    ListBound,
    /// Aucun pack de widgets du deck sous cet espace de noms.
    UnknownNamespace,
    /// Le pack est requis par un autre pack du deck.
    PackRequired,
    /// Des slides ou éléments utilisent encore un widget du pack.
    PackInUse,
    /// Un pack dont le pack ajouté a besoin n'est pas épinglé.
    PackRequirementMissing,
    /// Le pack `base` ne se retire jamais d'un deck.
    BaseRequired,
    /// L'espace de noms est déjà épinglé sous l'autre sorte de pack.
    PackKindConflict,
    /// Les `extends` des thèmes épinglés bouclent.
    ThemeCycle,
    /// Les thèmes épinglés ne sont pas exactement la fermeture du thème du deck.
    ThemeClosure,
}

/// La borne d'une liste qu'une opération dépasserait.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ListBound {
    /// La liste a déjà `max` items.
    Full { max: usize },
    /// La liste doit garder au moins `min` items.
    Minimum { min: usize },
    /// Une cellule de la colonne `column` dépasse sa longueur permise.
    CellTooLong { column: String },
    /// Un id d'item dépasse [`ITEM_ID_MAX`](crate::ITEM_ID_MAX) caractères.
    IdTooLong,
    /// Plus de `max` items d'un coup (liste entière, sous-arbre inséré).
    TooManyItems { max: usize },
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
    #[snafu(display("invalid value for {field}: {reason}"))]
    InvalidValue { field: String, reason: ValueReason, backtrace: Backtrace },
    #[snafu(display("invalid value for {field}"))]
    ParseValue { field: String, source: serde_json::Error, backtrace: Backtrace },
    /// `subject` : l'élément du graphique, ou le paramètre quand l'élément
    /// n'est pas connu (conversion d'une valeur).
    #[snafu(display("chart {subject} refused the change"))]
    Chart { subject: String, source: oku_slides_chart::ChartError, backtrace: Backtrace },
    /// Tableau refusé ; `subject` : l'élément, ou le paramètre s'il n'est pas connu.
    #[snafu(display("table {subject} refused the change"))]
    Table { subject: String, source: oku_slides_chart::ChartError, backtrace: Backtrace },
    #[snafu(display("unknown widget: {kind}"))]
    UnknownWidget { kind: String, backtrace: Backtrace },
    #[snafu(display("{widget} has no parameter {param}"))]
    UnknownParam { widget: String, param: String, backtrace: Backtrace },
    #[snafu(display("{param} expects {expected}"))]
    ParamType { param: String, expected: ExpectedType, backtrace: Backtrace },
    #[snafu(display("{slot} does not accept {kind}"))]
    SlotRejects { slot: String, kind: String, backtrace: Backtrace },
    #[snafu(display("{kind} cannot be used here"))]
    WrongCategory { kind: String, backtrace: Backtrace },
    #[snafu(display("{value} is not allowed for {param}"))]
    ChoiceOutOfRange { param: String, value: String, backtrace: Backtrace },
    #[snafu(display("{list} has no item {item}"))]
    ItemNotFound { list: String, item: String, backtrace: Backtrace },
    #[snafu(display("{list}: {}", bound_reason(bound)))]
    ListBound { list: String, bound: ListBound, backtrace: Backtrace },
    #[snafu(display("no widget pack {namespace} in this deck"))]
    UnknownNamespace { namespace: String, backtrace: Backtrace },
    #[snafu(display("widget pack {namespace} is required by {by}"))]
    PackRequired { namespace: String, by: String, backtrace: Backtrace },
    #[snafu(display("widget pack {namespace} is used by {}", users.join(", ")))]
    PackInUse { namespace: String, users: Vec<String>, backtrace: Backtrace },
    #[snafu(display("widget pack {namespace} needs {required} first"))]
    PackRequirementMissing { namespace: String, required: String, backtrace: Backtrace },
    #[snafu(display("every deck keeps the base widget pack"))]
    BaseRequired { backtrace: Backtrace },
    #[snafu(display("pack {namespace} is already pinned under the other pack kind"))]
    PackKindConflict { namespace: String, backtrace: Backtrace },
    /// `chain` : les espaces de noms de la boucle, joints par ` → `.
    #[snafu(display("themes extend each other in a cycle: {chain}"))]
    ThemeCycle { chain: String, backtrace: Backtrace },
    #[snafu(display("the pinned themes are not the closure of the deck theme ({namespace})"))]
    ThemeClosure { namespace: String, backtrace: Backtrace },
}

/// Un refus de la chaîne d'un thème devient le refus de l'op qui la pose.
impl From<crate::theme_chain::ThemeChainError> for OpError {
    fn from(e: crate::theme_chain::ThemeChainError) -> Self {
        use crate::theme_chain::ThemeChainError as E;
        match e {
            E::Cycle { chain } => {
                let chain = chain.iter().map(crate::Namespace::as_str).collect::<Vec<_>>().join(" → ");
                ThemeCycleSnafu { chain }.build().into()
            }
            E::Missing { namespace } | E::Extra { namespace } => {
                ThemeClosureSnafu { namespace: namespace.as_str() }.build().into()
            }
        }
    }
}

fn bound_reason(bound: &ListBound) -> String {
    match bound {
        ListBound::Full { max } => format!("the list is full ({max} items)"),
        ListBound::Minimum { min } => format!("the list needs at least {min} items"),
        ListBound::CellTooLong { column } => format!("the cell {column} is too long"),
        ListBound::IdTooLong => "an item id is too long".to_owned(),
        ListBound::TooManyItems { max } => format!("the list has more than {max} items"),
    }
}

impl OpError {
    pub fn kind(&self) -> OpErrorKind {
        match *self.0 {
            OpInner::SlideNotFound { .. } => OpErrorKind::SlideNotFound,
            OpInner::ElementNotFound { .. } => OpErrorKind::ElementNotFound,
            OpInner::DuplicateId { .. } => OpErrorKind::DuplicateId,
            OpInner::IndexOutOfRange { .. } => OpErrorKind::IndexOutOfRange,
            OpInner::InvalidValue { .. } | OpInner::ParseValue { .. } => OpErrorKind::InvalidValue,
            OpInner::Chart { .. } => OpErrorKind::InvalidChart,
            OpInner::Table { .. } => OpErrorKind::InvalidTable,
            OpInner::UnknownWidget { .. } => OpErrorKind::UnknownWidget,
            OpInner::UnknownParam { .. } => OpErrorKind::UnknownParam,
            OpInner::ParamType { .. } => OpErrorKind::ParamType,
            OpInner::ItemNotFound { .. } => OpErrorKind::ItemNotFound,
            OpInner::ChoiceOutOfRange { .. } => OpErrorKind::ChoiceOutOfRange,
            OpInner::SlotRejects { .. } => OpErrorKind::SlotRejects,
            OpInner::WrongCategory { .. } => OpErrorKind::WrongCategory,
            OpInner::ListBound { .. } => OpErrorKind::ListBound,
            OpInner::UnknownNamespace { .. } => OpErrorKind::UnknownNamespace,
            OpInner::PackRequired { .. } => OpErrorKind::PackRequired,
            OpInner::PackInUse { .. } => OpErrorKind::PackInUse,
            OpInner::PackRequirementMissing { .. } => OpErrorKind::PackRequirementMissing,
            OpInner::BaseRequired { .. } => OpErrorKind::BaseRequired,
            OpInner::PackKindConflict { .. } => OpErrorKind::PackKindConflict,
            OpInner::ThemeCycle { .. } => OpErrorKind::ThemeCycle,
            OpInner::ThemeClosure { .. } => OpErrorKind::ThemeClosure,
        }
    }

    /// Ce que l'erreur désigne (slide, élément, identifiant, champ, widget
    /// ou paramètre), s'il y en a un.
    pub fn subject(&self) -> Option<&str> {
        match &*self.0 {
            OpInner::SlideNotFound { slide, .. } => Some(slide),
            OpInner::ElementNotFound { element, .. } => Some(element),
            OpInner::Chart { subject, .. } | OpInner::Table { subject, .. } => Some(subject),
            OpInner::DuplicateId { id, .. } => Some(id),
            OpInner::InvalidValue { field, .. } | OpInner::ParseValue { field, .. } => Some(field),
            OpInner::UnknownWidget { kind, .. }
            | OpInner::SlotRejects { kind, .. }
            | OpInner::WrongCategory { kind, .. } => Some(kind),
            OpInner::UnknownParam { param, .. }
            | OpInner::ParamType { param, .. }
            | OpInner::ChoiceOutOfRange { param, .. } => Some(param),
            OpInner::ItemNotFound { item, .. } => Some(item),
            OpInner::ListBound { list, .. } => Some(list),
            OpInner::UnknownNamespace { namespace, .. }
            | OpInner::PackRequired { namespace, .. }
            | OpInner::PackInUse { namespace, .. }
            | OpInner::PackRequirementMissing { namespace, .. }
            | OpInner::PackKindConflict { namespace, .. }
            | OpInner::ThemeClosure { namespace, .. } => Some(namespace),
            OpInner::ThemeCycle { chain, .. } => Some(chain),
            OpInner::IndexOutOfRange { .. } | OpInner::BaseRequired { .. } => None,
        }
    }

    /// Pourquoi la valeur est refusée, pour une erreur [`OpErrorKind::InvalidValue`].
    pub fn value_reason(&self) -> Option<ValueReason> {
        match &*self.0 {
            OpInner::InvalidValue { reason, .. } => Some(*reason),
            OpInner::ParseValue { .. } => Some(ValueReason::Unreadable),
            _ => None,
        }
    }

    /// Ce qui était attendu, pour une erreur [`OpErrorKind::ParamType`].
    pub(crate) fn expected_type(&self) -> Option<ExpectedType> {
        match &*self.0 {
            OpInner::ParamType { expected, .. } => Some(*expected),
            _ => None,
        }
    }

    /// La borne de liste refusée, pour une erreur [`OpErrorKind::ListBound`].
    pub fn list_bound(&self) -> Option<&ListBound> {
        match &*self.0 {
            OpInner::ListBound { bound, .. } => Some(bound),
            _ => None,
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
    /// Des éléments s'imbriquent au-delà de [`MAX_SLOT_DEPTH`](crate::MAX_SLOT_DEPTH).
    TooDeep,
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
    #[snafu(display("elements nest deeper than {max} slots"))]
    TooDeep { max: usize, backtrace: Backtrace },
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
            SourcesInner::TooDeep { .. } => SourcesErrorKind::TooDeep,
            SourcesInner::Replica { ref source, .. } => SourcesErrorKind::Replica(source.kind()),
        }
    }
}

impl Localize for OpError {
    fn localize(&self, l: &Localizer) -> String {
        let host = match &*self.0 {
            OpInner::Chart { subject, source, .. } => Some(("op-invalid-chart", subject, source)),
            OpInner::Table { subject, source, .. } => Some(("op-invalid-table", subject, source)),
            _ => None,
        };
        if let Some((key, subject, source)) = host {
            return l.tr_with(key, &[("subject", subject.as_str().into()), ("reason", source.localize(l).into())]);
        }
        if let Some(reason) = self.value_reason() {
            return l.tr_with(
                "op-invalid-value-because",
                &[("subject", self.subject().unwrap_or_default().into()), ("reason", l.tr(reason.key()).into())],
            );
        }
        if let Some(expected) = self.expected_type() {
            let nullable = if expected.nullable { "yes" } else { "no" };
            let reason = l.tr_with(expected.shape.key(), &[("nullable", nullable.into())]);
            return l.tr_with(
                "op-param-type-because",
                &[("subject", self.subject().unwrap_or_default().into()), ("reason", reason.into())],
            );
        }
        if let OpInner::ListBound { list, bound, .. } = &*self.0 {
            let list = list.as_str().into();
            return match bound {
                ListBound::Full { max } => l.tr_with("op-list-full", &[("subject", list), ("max", (*max).into())]),
                ListBound::Minimum { min } => {
                    l.tr_with("op-list-minimum", &[("subject", list), ("min", (*min).into())])
                }
                ListBound::CellTooLong { column } => {
                    l.tr_with("op-list-cell-too-long", &[("subject", list), ("column", column.as_str().into())])
                }
                ListBound::IdTooLong => l.tr_with("op-list-id-too-long", &[("subject", list)]),
                ListBound::TooManyItems { max } => {
                    l.tr_with("op-list-too-many-items", &[("subject", list), ("max", (*max).into())])
                }
            };
        }
        if let Some(text) = self.localize_pack(l) {
            return text;
        }
        let id = match self.kind() {
            OpErrorKind::SlideNotFound => "op-slide-not-found",
            OpErrorKind::ElementNotFound => "op-element-not-found",
            OpErrorKind::DuplicateId => "op-duplicate-id",
            OpErrorKind::IndexOutOfRange => "op-index-out-of-range",
            OpErrorKind::InvalidValue => "op-invalid-value",
            OpErrorKind::InvalidChart => "op-invalid-chart",
            OpErrorKind::InvalidTable => "op-invalid-table",
            OpErrorKind::UnknownWidget => "op-unknown-widget",
            OpErrorKind::UnknownParam => "op-unknown-param",
            OpErrorKind::ItemNotFound => "op-item-not-found",
            OpErrorKind::ChoiceOutOfRange => "op-choice-out-of-range",
            OpErrorKind::SlotRejects => "op-slot-rejects",
            OpErrorKind::WrongCategory => "op-wrong-category",
            OpErrorKind::BaseRequired => "op-base-required",
            // Traités plus haut : ces erreurs portent leurs arguments.
            OpErrorKind::ParamType
            | OpErrorKind::ListBound
            | OpErrorKind::UnknownNamespace
            | OpErrorKind::PackRequired
            | OpErrorKind::PackInUse
            | OpErrorKind::PackRequirementMissing
            | OpErrorKind::PackKindConflict
            | OpErrorKind::ThemeCycle
            | OpErrorKind::ThemeClosure => "op-invalid-value",
        };
        l.tr_with(id, &[("subject", self.subject().unwrap_or_default().into())])
    }
}

impl OpError {
    /// Message d'un refus d'épinglage de pack ou de thème, avec ses arguments.
    fn localize_pack(&self, l: &Localizer) -> Option<String> {
        Some(match &*self.0 {
            OpInner::UnknownNamespace { namespace, .. } => {
                l.tr_with("op-unknown-namespace", &[("pack", namespace.as_str().into())])
            }
            OpInner::PackRequired { namespace, by, .. } => {
                l.tr_with("op-pack-required", &[("pack", namespace.as_str().into()), ("by", by.as_str().into())])
            }
            OpInner::PackInUse { namespace, users, .. } => {
                l.tr_with("op-pack-in-use", &[("pack", namespace.as_str().into()), ("count", users.len().into())])
            }
            OpInner::PackRequirementMissing { namespace, required, .. } => l.tr_with(
                "op-pack-requirement-missing",
                &[("pack", namespace.as_str().into()), ("required", required.as_str().into())],
            ),
            OpInner::PackKindConflict { namespace, .. } => {
                l.tr_with("op-pack-kind-conflict", &[("pack", namespace.as_str().into())])
            }
            OpInner::ThemeCycle { chain, .. } => l.tr_with("op-theme-cycle", &[("chain", chain.as_str().into())]),
            OpInner::ThemeClosure { namespace, .. } => {
                l.tr_with("op-theme-closure", &[("namespace", namespace.as_str().into())])
            }
            _ => return None,
        })
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
            SourcesErrorKind::TooDeep => "sources-too-deep",
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
    /// Deck d'une autre version (`DECK_VERSION`), ou replica d'un autre format (`meta.format`).
    UnsupportedVersion,
    /// Enregistrement refusé : des éléments retirés s'empilent au-delà de la profondeur relue (t-866).
    TooDeep,
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
    #[snafu(display("container outside the widget params"))]
    ContainerOutsideParams { backtrace: Backtrace },
    #[snafu(display("the two replicas have no common origin"))]
    Unrelated { backtrace: Backtrace },
    #[snafu(display("unsupported deck version: {found}"))]
    UnsupportedDeckVersion { found: String, backtrace: Backtrace },
    #[snafu(display("unsupported replica format: {found}"))]
    UnsupportedReplicaFormat { found: String, backtrace: Backtrace },
    #[snafu(display("malformed deck tree: {what}"))]
    MalformedDeckTree { what: &'static str, backtrace: Backtrace },
    #[snafu(display("element placed before its owner"))]
    TreeOrder { backtrace: Backtrace },
    #[snafu(display("slides inside the widget params"))]
    SlidesInParams { backtrace: Backtrace },
    #[snafu(display("elements nested deeper than {max} slots"))]
    SlotsTooDeep { max: usize, backtrace: Backtrace },
    #[snafu(display("deck tree deeper than {max} levels, removed elements included: not saved"))]
    TreeTooDeep { max: usize, backtrace: Backtrace },
}

impl ReplicaError {
    pub fn kind(&self) -> ReplicaErrorKind {
        match self.0 {
            ReplicaInner::Import { .. }
            | ReplicaInner::NotADeck { .. }
            | ReplicaInner::ImportPanicked { .. }
            | ReplicaInner::MalformedDeckTree { .. } => ReplicaErrorKind::Corrupted,
            ReplicaInner::TooLarge { .. } => ReplicaErrorKind::TooLarge,
            ReplicaInner::TreeTooDeep { .. } => ReplicaErrorKind::TooDeep,
            ReplicaInner::Unrelated { .. } => ReplicaErrorKind::Unrelated,
            ReplicaInner::UnsupportedDeckVersion { .. } | ReplicaInner::UnsupportedReplicaFormat { .. } => {
                ReplicaErrorKind::UnsupportedVersion
            }
            ReplicaInner::Loro { .. }
            | ReplicaInner::TextUpdate { .. }
            | ReplicaInner::Export { .. }
            | ReplicaInner::ContainerOutsideParams { .. }
            | ReplicaInner::TreeOrder { .. }
            | ReplicaInner::SlidesInParams { .. }
            | ReplicaInner::SlotsTooDeep { .. }
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
            ReplicaErrorKind::UnsupportedVersion => "replica-unsupported-version",
            ReplicaErrorKind::TooDeep => "replica-too-deep",
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
    /// Une liste sans colonne.
    EmptyList,
    /// Type de colonne de liste non admis.
    ColumnType,
    /// Nom de paramètre ou de colonne réservé (`id`, `kind`) ou contenant `/`.
    ReservedName,
    /// Le `max` d'une liste est sous son `min`.
    ListBounds,
    /// Le défaut d'une liste dépasse ses bornes de taille (items, cellules).
    ListDefaultTooLarge,
    /// Une liste imbriquée (colonne `list`) porte elle-même une colonne `list` : un seul niveau.
    ListDepth,
    /// Le défaut d'une liste imbriquée (le sien, ou celui d'une ligne parente) a moins
    /// d'items que son `min`.
    ListDefaultBelowMin,
    /// Une slide déclare `grow` ou `typography`, réservés aux éléments.
    SlideSizing,
    /// La catégorie d'un widget ne correspond pas à la sorte de son pack.
    PackCategory,
    /// Un widget de registre sans espace de noms (un registre se lit toujours qualifié).
    Unqualified,
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
            SchemaFault::EmptyList => "list without columns",
            SchemaFault::ColumnType => "column type not allowed in a list",
            SchemaFault::ReservedName => "reserved parameter name",
            SchemaFault::ListBounds => "list max below min",
            SchemaFault::ListDefaultTooLarge => "list default exceeds the list size limits",
            SchemaFault::ListDepth => "list nested more than one level",
            SchemaFault::ListDefaultBelowMin => "nested list default below its min",
            SchemaFault::SlideSizing => "slide declares grow or typography",
            SchemaFault::PackCategory => "widget category differs from its pack kind",
            SchemaFault::Unqualified => "registry widget without a namespace",
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
            SchemaFault::EmptyList => "widget-fault-empty-list",
            SchemaFault::ColumnType => "widget-fault-column-type",
            SchemaFault::ReservedName => "widget-fault-reserved-name",
            SchemaFault::ListBounds => "widget-fault-list-bounds",
            SchemaFault::ListDefaultTooLarge => "widget-fault-list-default-too-large",
            SchemaFault::ListDepth => "widget-fault-list-depth",
            SchemaFault::ListDefaultBelowMin => "widget-fault-list-default-below-min",
            SchemaFault::SlideSizing => "widget-fault-slide-sizing",
            SchemaFault::PackCategory => "widget-fault-pack-category",
            SchemaFault::Unqualified => "widget-fault-unqualified",
        }
    }
}

/// Nature d'une [`WidgetError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WidgetErrorKind {
    Unreadable,
    BadSchema(SchemaFault),
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum WidgetInner {
    #[snafu(display("unreadable widget registry"))]
    Unreadable { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("widget {widget}: {fault}"))]
    BadSchema { widget: String, fault: SchemaFault, backtrace: Backtrace },
}

impl WidgetError {
    /// Construit une [`WidgetError::kind`] `Unreadable` à partir d'une défaillance
    /// JSON survenue hors du cœur (p. ex. la sonde de rendu ne peut pas
    /// convertir la valeur Typst en JSON).
    pub fn unreadable(source: serde_json::Error) -> Self {
        UnreadableSnafu.into_error(source).into()
    }

    /// Le widget `widget` mal déclaré pour `fault`, constaté hors du registre.
    pub fn bad_schema(widget: &str, fault: SchemaFault) -> Self {
        BadSchemaSnafu { widget, fault }.build().into()
    }

    pub fn kind(&self) -> WidgetErrorKind {
        match &self.0 {
            WidgetInner::Unreadable { .. } => WidgetErrorKind::Unreadable,
            WidgetInner::BadSchema { fault, .. } => WidgetErrorKind::BadSchema(*fault),
        }
    }

    /// Le widget refusé ; `None` si le registre est illisible.
    pub fn widget(&self) -> Option<&str> {
        match &self.0 {
            WidgetInner::Unreadable { .. } => None,
            WidgetInner::BadSchema { widget, .. } => Some(widget),
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
        }
    }
}

/// Une chaîne qui n'est pas une [`PackVersion`](crate::PackVersion).
#[derive(Debug, Snafu)]
#[snafu(display("invalid pack version: {value}"), visibility(pub(crate)))]
pub struct PackVersionError {
    value: String,
    backtrace: Backtrace,
}

/// Une chaîne qui n'est pas un [`WidgetFile`](crate::WidgetFile).
#[derive(Debug, Snafu)]
#[snafu(display("invalid widget file name: {value}"), visibility(pub(crate)))]
pub struct WidgetFileError {
    value: String,
    backtrace: Backtrace,
}

/// Un espace de noms de pack hors de la syntaxe permise.
#[derive(Debug, Snafu)]
#[snafu(display("invalid pack namespace: {value}"), context(suffix(Snafu)), visibility(pub(crate)))]
pub struct NamespaceError {
    value: String,
    backtrace: Backtrace,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
