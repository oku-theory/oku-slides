//! Erreurs de la session : fichiers, rendu, opérations refusées, requêtes
//! impossibles dans l'état courant.

use std::path::PathBuf;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::storage::StorageError;
use snafu::{Backtrace, IntoError, Snafu};

/// Une requête de l'interface qui n'a pas abouti. La session est intacte.
///
/// Elle enveloppe les erreurs des autres crates : boîte, pour que les
/// `Result` de la session restent petits.
#[derive(Debug, Snafu)]
pub struct SessionError(Box<SessionInner>);

/// Nature d'une [`SessionError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionErrorKind {
    /// Fichier illisible ou impossible à écrire.
    File,
    /// Ce PDF ne contient pas de sources Oku Slides.
    NotOkuSlides,
    /// Sources ou deck illisibles.
    InvalidDocument,
    /// Une opération de la transaction a été refusée.
    Op,
    Render,
    /// Aucun fichier : il faut choisir où enregistrer.
    NoPath,
    NoSlide,
    /// L'élément n'est pas un graphique (ou n'existe pas).
    NoChart,
    /// L'élément n'est pas un tableau (ou n'existe pas).
    NoTable,
    /// Le graphique n'a pas pu changer de type.
    Chart,
    /// Un pack intégré ne se supprime pas.
    BuiltinTemplateSet,
    /// Pas de dossier pour la bibliothèque de l'utilisateur.
    NoLibraryDir,
    /// Pack de modèles inconnu.
    UnknownPack,
    /// Version de pack introuvable.
    PackVersionMissing,
    /// Pack ou thème du deck seul, que la bibliothèque n'enregistre pas.
    DeckOnly,
    /// Espace de noms déjà pris par un autre pack de la bibliothèque.
    NamespaceTaken,
    /// Espace de noms `deck`, réservé au pack propre d'un deck.
    NamespaceReserved,
    /// Nouvelle version d'un pack sous un autre espace de noms que les siennes.
    NamespaceChanged,
    /// Espace de noms qu'aucun pack ne porte (bibliothèque ou deck).
    UnknownNamespace,
    /// Packs qui se requièrent en boucle.
    RequiresCycle,
    /// L'import d'une présentation PowerPoint n'a pas abouti.
    Import,
    /// Un outil d'ajustement de l'import a refusé l'appel (voir `TuneErrorKind`).
    ImportTune,
    /// Requête impossible pendant l'édition des modèles.
    EditingTemplates,
    /// Requête qui demande l'édition des modèles.
    NotEditingTemplates,
    /// En mode modèles, un choix de thème mêlé à d'autres ops.
    ThemeChoiceAlone,
    /// Réglage de l'hôte inconnu.
    HostKey,
    /// La réponse n'a pas pu être écrite en JSON.
    Response,
    /// Arguments d'une surface de l'agent invalides.
    AgentForm,
    /// Adresse de l'agent : ni la slide ni un de ses éléments.
    UnknownOwner,
    /// Adresse de l'agent : pas un champ texte de l'objet.
    UnknownField,
    /// Adresse de l'agent : l'objet n'a pas de texte par défaut, il faut nommer son champ.
    FieldNeeded,
    /// Onglet inconnu (fermé entre-temps).
    UnknownTab,
    /// Fenêtre inconnue (fermée entre-temps).
    UnknownWindow,
    /// Hôte à un seul document : pas d'autre onglet.
    SingleDocument,
    /// Fermeture refusée : travail non enregistré.
    Unsaved,
    /// Fusion refusée : ce PDF est une autre présentation.
    MergeUnrelated,
    /// Requête qui demande l'historique ouvert.
    HistoryNotOpen,
    /// Modifier, enregistrer ou fusionner pendant que l'historique est ouvert.
    InHistory,
    /// Un fichier existe déjà là où déplacer ou copier le deck.
    TargetExists,
    /// Modifier l'aide, montrée en lecture seule.
    ReadOnly,
    /// Requête réservée à l'aide.
    NotHelp,
    /// Annuler un tour de l'agent qui n'est plus la dernière modification du deck.
    TurnNotOnTop,
    /// Thème inconnu.
    UnknownTheme,
    /// Version de thème introuvable.
    ThemeVersionMissing,
    /// Valeur de jeton de design refusée par le schéma.
    InvalidTokenValue,
    /// Jeton de design encore référencé par d'autres jetons.
    TokenInUse,
    /// Nom, taille ou nombre de fichiers de widgets refusé.
    InvalidWidget,
    /// Widget utilisé par un modèle du pack.
    WidgetInUse,
    /// Fichier de widget introuvable.
    UnknownWidget,
    /// Le fichier du widget ne compile pas.
    WidgetInvalid,
    /// Essai d'un paramètre que le widget ne déclare pas.
    TrialUndeclared,
    /// Essai d'un paramètre non réglable dans l'aperçu.
    TrialFixed,
    /// Valeur d'essai trop longue.
    TrialTooLong,
    /// Valeur d'essai refusée pour le type du paramètre.
    TrialValue,
    /// Valeur d'essai impossible à garder.
    TrialEncode,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum SessionInner {
    #[snafu(display("cannot read {}", path.display()))]
    Read { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Write { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("this PDF was not created by Oku Slides: it cannot be edited"))]
    NotOkuSlides { backtrace: Backtrace },
    #[snafu(display("unreadable PDF"))]
    Pdf { source: oku_slides_pdf::PdfError, backtrace: Backtrace },
    #[snafu(display("unreadable sources"))]
    Sources { source: oku_slides_core::SourcesError, backtrace: Backtrace },
    #[snafu(display("invalid deck JSON"))]
    DeckJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("transaction refused"))]
    Op { source: oku_slides_core::TransactionError, backtrace: Backtrace },
    #[snafu(display("rendering failed"))]
    Render { source: oku_slides_render::RenderError, backtrace: Backtrace },
    #[snafu(display("widgets unavailable"))]
    Registry { source: oku_slides_render::RegistryError, backtrace: Backtrace },
    #[snafu(display("rendering unavailable"))]
    NotRendered { backtrace: Backtrace },
    #[snafu(display("no file: choose where to save"))]
    NoPath { backtrace: Backtrace },
    #[snafu(display("slide not found: {id}"))]
    NoSlide { id: String, backtrace: Backtrace },
    #[snafu(display("not a chart: {id}"))]
    NoChart { id: String, backtrace: Backtrace },
    #[snafu(display("not a table: {id}"))]
    NoTable { id: String, backtrace: Backtrace },
    #[snafu(display("chart conversion failed"))]
    Chart { source: oku_slides_chart::ChartError, backtrace: Backtrace },
    #[snafu(display("built-in template set {id} cannot be deleted"))]
    BuiltinTemplateSet { id: String, backtrace: Backtrace },
    #[snafu(display("no folder for the user {kind} library"))]
    NoLibraryDir { kind: &'static str, backtrace: Backtrace },
    #[snafu(display("pack not found: {id}"))]
    UnknownPack { id: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("version {version} of pack {id} not found"))]
    PackVersionMissing { id: uuid::Uuid, version: oku_slides_core::PackVersion, backtrace: Backtrace },
    #[snafu(display("invalid manifest for pack {id}"))]
    PackJson { id: uuid::Uuid, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("manifest of pack {found} given for pack {id}"))]
    PackIdMismatch { id: uuid::Uuid, found: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("{id} belongs to its deck only and never enters the library"))]
    DeckOnly { id: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("namespace {namespace} already belongs to pack {owner}"))]
    NamespaceTaken { namespace: String, owner: String, backtrace: Backtrace },
    #[snafu(display("namespace {namespace} is reserved to a deck's own pack"))]
    NamespaceReserved { namespace: String, backtrace: Backtrace },
    #[snafu(display("pack {id} keeps its namespace {expected}, not {found}"))]
    NamespaceChanged { id: uuid::Uuid, expected: String, found: String, backtrace: Backtrace },
    #[snafu(display("no pack named {namespace} in the library"))]
    UnknownNamespace { namespace: String, backtrace: Backtrace },
    #[snafu(display("the deck pins no widget pack {namespace}"))]
    NotPinned { namespace: String, backtrace: Backtrace },
    #[snafu(display("packs require each other in a loop: {chain}"))]
    RequiresCycle { chain: String, backtrace: Backtrace },
    #[snafu(display("embedded pack {namespace} does not match its pin"))]
    PackPinMismatch { namespace: String, backtrace: Backtrace },
    #[snafu(display("PowerPoint import failed"))]
    Import { source: crate::import::ImportError, backtrace: Backtrace },
    #[snafu(display("import tuning refused"))]
    Tune { source: crate::import::TuneError, backtrace: Backtrace },
    #[snafu(display("finish editing the templates first"))]
    EditingTemplates { backtrace: Backtrace },
    #[snafu(display("no template set is being edited"))]
    NotEditingTemplates { backtrace: Backtrace },
    #[snafu(display("in template mode, choose the theme alone and edit its tokens with set_token"))]
    ThemeChoiceAlone { backtrace: Backtrace },
    #[snafu(display("unknown host setting: {key}"))]
    HostKey { key: String, backtrace: Backtrace },
    #[snafu(display("the response could not be written as JSON"))]
    Response { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("invalid surface: {reason}"))]
    AgentForm { reason: crate::agent::AgentFormReason, backtrace: Backtrace },
    #[snafu(display("{owner} is neither slide {slide} nor one of its elements"))]
    UnknownOwner { slide: String, owner: String, backtrace: Backtrace },
    #[snafu(display("{owner} has no text field {field}"))]
    UnknownField { owner: String, field: String, backtrace: Backtrace },
    #[snafu(display("name a text field of {owner}"))]
    FieldNeeded { owner: String, backtrace: Backtrace },
    #[snafu(display("tab not found: {id}"))]
    UnknownTab { id: String, backtrace: Backtrace },
    #[snafu(display("window not found: {label}"))]
    UnknownWindow { label: String, backtrace: Backtrace },
    #[snafu(display("only one presentation can be open here"))]
    SingleDocument { backtrace: Backtrace },
    #[snafu(display("{title} has unsaved changes"))]
    Unsaved { title: String, backtrace: Backtrace },
    #[snafu(display("cannot merge another presentation"))]
    MergeUnrelated { source: oku_slides_core::SourcesError, backtrace: Backtrace },
    #[snafu(display("the history is not open"))]
    HistoryNotOpen { backtrace: Backtrace },
    #[snafu(display("close the history first"))]
    InHistory { backtrace: Backtrace },
    #[snafu(display("a file already exists at {}", path.display()))]
    TargetExists { path: PathBuf, backtrace: Backtrace },
    #[snafu(display("the help is read-only"))]
    ReadOnly { backtrace: Backtrace },
    #[snafu(display("this tab is not the help"))]
    NotHelp { backtrace: Backtrace },
    #[snafu(display("agent turn {turn} is no longer the last change"))]
    TurnNotOnTop { turn: u32, backtrace: Backtrace },
    #[snafu(display("unknown theme: {id}"))]
    UnknownTheme { id: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("version {version} of theme {id} not found"))]
    ThemeVersionMissing { id: uuid::Uuid, version: oku_slides_core::PackVersion, backtrace: Backtrace },
    #[snafu(display("invalid manifest for theme {id}"))]
    ThemeJson { id: uuid::Uuid, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("manifest of theme {found} given for theme {id}"))]
    ThemeIdMismatch { id: uuid::Uuid, found: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("invalid value for {path}: {reason}"))]
    InvalidTokenValue { path: String, reason: String, backtrace: Backtrace },
    #[snafu(display("{path} is used by: {}", users.join(", ")))]
    TokenInUse { path: String, users: Vec<String>, backtrace: Backtrace },
    #[snafu(display("unreadable help deck"))]
    HelpDeck { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("unreadable version"))]
    Version { source: oku_slides_core::ReplicaError, backtrace: Backtrace },
    #[snafu(display("invalid widget name: {name}"))]
    InvalidWidgetName { name: String, backtrace: Backtrace },
    #[snafu(display("widget {name} is larger than {max} KiB"))]
    WidgetTooLarge { name: String, max: usize, backtrace: Backtrace },
    #[snafu(display("widget {name} contains a nul character"))]
    WidgetHasNul { name: String, backtrace: Backtrace },
    #[snafu(display("a pack holds at most {max} widget files"))]
    TooManyWidgets { max: usize, backtrace: Backtrace },
    #[snafu(display("widget {name} is still used by: {}", uses.join(", ")))]
    WidgetInUse { name: String, uses: Vec<String>, backtrace: Backtrace },
    #[snafu(display("widget not found: {name}"))]
    UnknownWidget { name: String, backtrace: Backtrace },
    #[snafu(display("widget file does not compile: {name}"))]
    WidgetInvalid { name: String, backtrace: Backtrace },
    #[snafu(display("{param} is not a parameter of this widget"))]
    TrialUndeclared { param: String, backtrace: Backtrace },
    #[snafu(display("{param} cannot be set in the preview"))]
    TrialFixed { param: String, backtrace: Backtrace },
    #[snafu(display("trial value too long for {param}"))]
    TrialTooLong { param: String, backtrace: Backtrace },
    #[snafu(display("trial value refused for {param}"))]
    TrialValue { param: String, source: oku_slides_core::OpError, backtrace: Backtrace },
    #[snafu(display("trial value of {param} cannot be stored"))]
    TrialEncode { param: String, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("widget already exists: {name}"))]
    WidgetExists { name: String, backtrace: Backtrace },
}

impl SessionError {
    pub fn kind(&self) -> SessionErrorKind {
        use SessionInner::*;
        match *self.0 {
            Read { .. } | Write { .. } => SessionErrorKind::File,
            NotOkuSlides { .. } => SessionErrorKind::NotOkuSlides,
            Pdf { .. }
            | Sources { .. }
            | DeckJson { .. }
            | PackJson { .. }
            | PackIdMismatch { .. }
            | PackPinMismatch { .. }
            | ThemeJson { .. }
            | ThemeIdMismatch { .. }
            | Version { .. } => SessionErrorKind::InvalidDocument,
            Op { .. } => SessionErrorKind::Op,
            Render { .. } | NotRendered { .. } | Registry { .. } => SessionErrorKind::Render,
            NoPath { .. } => SessionErrorKind::NoPath,
            NoSlide { .. } => SessionErrorKind::NoSlide,
            NoChart { .. } => SessionErrorKind::NoChart,
            NoTable { .. } => SessionErrorKind::NoTable,
            Chart { .. } => SessionErrorKind::Chart,
            BuiltinTemplateSet { .. } => SessionErrorKind::BuiltinTemplateSet,
            NoLibraryDir { .. } => SessionErrorKind::NoLibraryDir,
            UnknownPack { .. } => SessionErrorKind::UnknownPack,
            PackVersionMissing { .. } => SessionErrorKind::PackVersionMissing,
            DeckOnly { .. } => SessionErrorKind::DeckOnly,
            NamespaceTaken { .. } => SessionErrorKind::NamespaceTaken,
            NamespaceReserved { .. } => SessionErrorKind::NamespaceReserved,
            NamespaceChanged { .. } => SessionErrorKind::NamespaceChanged,
            UnknownNamespace { .. } | NotPinned { .. } => SessionErrorKind::UnknownNamespace,
            RequiresCycle { .. } => SessionErrorKind::RequiresCycle,
            Import { .. } => SessionErrorKind::Import,
            Tune { .. } => SessionErrorKind::ImportTune,
            EditingTemplates { .. } => SessionErrorKind::EditingTemplates,
            NotEditingTemplates { .. } => SessionErrorKind::NotEditingTemplates,
            ThemeChoiceAlone { .. } => SessionErrorKind::ThemeChoiceAlone,
            HostKey { .. } => SessionErrorKind::HostKey,
            Response { .. } => SessionErrorKind::Response,
            AgentForm { .. } => SessionErrorKind::AgentForm,
            UnknownOwner { .. } => SessionErrorKind::UnknownOwner,
            UnknownField { .. } => SessionErrorKind::UnknownField,
            FieldNeeded { .. } => SessionErrorKind::FieldNeeded,
            UnknownTab { .. } => SessionErrorKind::UnknownTab,
            UnknownWindow { .. } => SessionErrorKind::UnknownWindow,
            SingleDocument { .. } => SessionErrorKind::SingleDocument,
            Unsaved { .. } => SessionErrorKind::Unsaved,
            MergeUnrelated { .. } => SessionErrorKind::MergeUnrelated,
            HistoryNotOpen { .. } => SessionErrorKind::HistoryNotOpen,
            InHistory { .. } => SessionErrorKind::InHistory,
            TargetExists { .. } => SessionErrorKind::TargetExists,
            ReadOnly { .. } => SessionErrorKind::ReadOnly,
            NotHelp { .. } => SessionErrorKind::NotHelp,
            TurnNotOnTop { .. } => SessionErrorKind::TurnNotOnTop,
            UnknownTheme { .. } => SessionErrorKind::UnknownTheme,
            ThemeVersionMissing { .. } => SessionErrorKind::ThemeVersionMissing,
            InvalidTokenValue { .. } => SessionErrorKind::InvalidTokenValue,
            TokenInUse { .. } => SessionErrorKind::TokenInUse,
            HelpDeck { .. } => SessionErrorKind::InvalidDocument,
            InvalidWidgetName { .. }
            | WidgetTooLarge { .. }
            | WidgetHasNul { .. }
            | TooManyWidgets { .. }
            | WidgetExists { .. } => SessionErrorKind::InvalidWidget,
            WidgetInUse { .. } => SessionErrorKind::WidgetInUse,
            UnknownWidget { .. } => SessionErrorKind::UnknownWidget,
            WidgetInvalid { .. } => SessionErrorKind::WidgetInvalid,
            TrialUndeclared { .. } => SessionErrorKind::TrialUndeclared,
            TrialFixed { .. } => SessionErrorKind::TrialFixed,
            TrialTooLong { .. } => SessionErrorKind::TrialTooLong,
            TrialValue { .. } => SessionErrorKind::TrialValue,
            TrialEncode { .. } => SessionErrorKind::TrialEncode,
        }
    }

    /// La nature du refus d'un outil d'ajustement de l'import, pour une erreur [`SessionErrorKind::ImportTune`].
    pub fn tune_kind(&self) -> Option<crate::import::TuneErrorKind> {
        match &*self.0 {
            SessionInner::Tune { source, .. } => Some(source.kind()),
            _ => None,
        }
    }

    /// L'erreur de transaction, si une opération a été refusée.
    pub fn transaction(&self) -> Option<&oku_slides_core::TransactionError> {
        match &*self.0 {
            SessionInner::Op { source, .. } => Some(source),
            _ => None,
        }
    }

    /// Pourquoi une surface de l'agent est refusée, pour une erreur [`SessionErrorKind::AgentForm`].
    pub fn agent_form_reason(&self) -> Option<&crate::agent::AgentFormReason> {
        match &*self.0 {
            SessionInner::AgentForm { reason, .. } => Some(reason),
            _ => None,
        }
    }

    /// L'erreur de rendu d'en dessous, s'il y en a une (sonde des widgets comprise).
    pub(crate) fn render_error(&self) -> Option<&oku_slides_render::RenderError> {
        match &*self.0 {
            SessionInner::Render { source, .. } => Some(source),
            SessionInner::Registry { source, .. } => source.render_error(),
            _ => None,
        }
    }

    /// L'erreur et ses causes, sur une ligne.
    pub fn one_line(&self) -> String {
        oku_slides_platform::errors::one_line(self)
    }
}

/// L'erreur d'une fusion : une autre présentation (sans origine commune)
/// a sa propre variante, les autres erreurs de sources restent `Sources`.
pub(crate) fn merge_error(source: oku_slides_core::SourcesError) -> SessionError {
    use oku_slides_core::{ReplicaErrorKind, SourcesErrorKind};
    if source.kind() == SourcesErrorKind::Replica(ReplicaErrorKind::Unrelated) {
        MergeUnrelatedSnafu.into_error(source).into()
    } else {
        SourcesSnafu.into_error(source).into()
    }
}

pub type Result<T, E = SessionError> = std::result::Result<T, E>;

impl Localize for SessionError {
    /// Le message, ou celui de l'erreur d'en dessous quand elle dit quelque
    /// chose à l'utilisateur (opération refusée, rendu, PDF).
    fn localize(&self, l: &Localizer) -> String {
        use SessionInner::*;
        let path_arg = |path: &PathBuf| [("path", path.display().to_string().into())];
        match &*self.0 {
            Read { path, .. } => l.tr_with("session-read", &path_arg(path)),
            Write { path, .. } => l.tr_with("session-write", &path_arg(path)),
            NotOkuSlides { .. } => l.tr("session-not-oku-slides"),
            Pdf { source, .. } => source.localize(l),
            Sources { source, .. } => source.localize(l),
            DeckJson { .. } => l.tr("session-invalid-deck"),
            Op { source, .. } => source.localize(l),
            Render { source, .. } => source.localize(l),
            Registry { source, .. } => source.localize(l),
            NotRendered { .. } => l.tr("session-not-rendered"),
            NoPath { .. } => l.tr("session-no-path"),
            NoSlide { id, .. } => l.tr_with("session-no-slide", &[("id", id.as_str().into())]),
            NoChart { id, .. } => l.tr_with("session-no-chart", &[("subject", id.as_str().into())]),
            NoTable { id, .. } => l.tr_with("session-no-table", &[("subject", id.as_str().into())]),
            Chart { source, .. } => source.localize(l),
            BuiltinTemplateSet { .. } => l.tr("session-builtin-template-set"),
            NoLibraryDir { .. } => l.tr("session-no-library-dir"),
            UnknownPack { id, .. } => l.tr_with("session-unknown-pack", &[("id", id.to_string().into())]),
            PackVersionMissing { id, version, .. } => l.tr_with(
                "session-pack-version-missing",
                &[("id", id.to_string().into()), ("version", version.as_str().into())],
            ),
            PackJson { id, .. } => l.tr_with("session-invalid-pack", &[("id", id.to_string().into())]),
            PackIdMismatch { id, found, .. } => l.tr_with(
                "session-pack-id-mismatch",
                &[("id", id.to_string().into()), ("found", found.to_string().into())],
            ),
            DeckOnly { id, .. } => l.tr_with("session-deck-only", &[("id", id.to_string().into())]),
            NamespaceReserved { namespace, .. } => {
                l.tr_with("session-namespace-reserved", &[("namespace", namespace.as_str().into())])
            }
            NamespaceChanged { id, expected, found, .. } => l.tr_with(
                "session-namespace-changed",
                &[
                    ("id", id.to_string().into()),
                    ("expected", expected.as_str().into()),
                    ("found", found.as_str().into()),
                ],
            ),
            NamespaceTaken { namespace, owner, .. } => l.tr_with(
                "session-namespace-taken",
                &[("namespace", namespace.as_str().into()), ("owner", owner.as_str().into())],
            ),
            UnknownNamespace { namespace, .. } => {
                l.tr_with("session-unknown-namespace", &[("namespace", namespace.as_str().into())])
            }
            NotPinned { namespace, .. } => l.tr_with("op-unknown-namespace", &[("pack", namespace.as_str().into())]),
            RequiresCycle { chain, .. } => l.tr_with("session-requires-cycle", &[("chain", chain.as_str().into())]),
            PackPinMismatch { namespace, .. } => {
                l.tr_with("issue-pack-pin-mismatch", &[("pack", namespace.as_str().into())])
            }
            Import { source, .. } => source.localize(l),
            Tune { source, .. } => source.localize(l),
            EditingTemplates { .. } => l.tr("session-editing-templates"),
            NotEditingTemplates { .. } => l.tr("session-not-editing-templates"),
            ThemeChoiceAlone { .. } => l.tr("session-theme-choice-alone"),
            HostKey { key, .. } => l.tr_with("session-host-key", &[("key", key.as_str().into())]),
            Response { .. } => l.tr("session-response"),
            AgentForm { reason, .. } => l.tr_with("session-agent-form", &[("reason", reason.localize(l).into())]),
            UnknownOwner { slide, owner, .. } => l.tr_with(
                "session-unknown-owner",
                &[("slide", slide.as_str().into()), ("owner", owner.as_str().into())],
            ),
            UnknownField { owner, field, .. } => l.tr_with(
                "session-unknown-field",
                &[("owner", owner.as_str().into()), ("field", field.as_str().into())],
            ),
            FieldNeeded { owner, .. } => l.tr_with("session-field-needed", &[("owner", owner.as_str().into())]),
            UnknownTab { .. } => l.tr("session-unknown-tab"),
            UnknownWindow { .. } => l.tr("session-unknown-window"),
            SingleDocument { .. } => l.tr("session-single-document"),
            Unsaved { title, .. } => l.tr_with("session-unsaved", &[("title", title.as_str().into())]),
            MergeUnrelated { .. } => l.tr("replica-unrelated"),
            HistoryNotOpen { .. } => l.tr("session-history-not-open"),
            InHistory { .. } => l.tr("session-in-history"),
            TargetExists { .. } => l.tr("session-target-exists"),
            ReadOnly { .. } => l.tr("session-read-only"),
            NotHelp { .. } => l.tr("session-not-help"),
            TurnNotOnTop { .. } => l.tr("session-turn-not-on-top"),
            UnknownTheme { id, .. } => l.tr_with("session-unknown-theme", &[("theme", id.to_string().into())]),
            ThemeVersionMissing { id, version, .. } => l.tr_with(
                "session-theme-version-missing",
                &[("id", id.to_string().into()), ("version", version.as_str().into())],
            ),
            ThemeJson { id, .. } => l.tr_with("session-invalid-theme", &[("id", id.to_string().into())]),
            ThemeIdMismatch { id, found, .. } => l.tr_with(
                "session-theme-id-mismatch",
                &[("id", id.to_string().into()), ("found", found.to_string().into())],
            ),
            InvalidTokenValue { path, reason, .. } => l.tr_with(
                "session-invalid-token-value",
                &[("path", path.as_str().into()), ("reason", l.tr(&format!("session-token-reason-{reason}")).into())],
            ),
            TokenInUse { path, users, .. } => {
                l.tr_with("session-token-in-use", &[("path", path.as_str().into()), ("users", users.join(", ").into())])
            }
            HelpDeck { .. } => l.tr("session-help-deck"),
            Version { source, .. } => source.localize(l),
            InvalidWidgetName { name, .. } => {
                l.tr_with("session-invalid-widget-name", &[("name", name.as_str().into())])
            }
            WidgetTooLarge { name, max, .. } => {
                l.tr_with("session-widget-too-large", &[("name", name.as_str().into()), ("max", (*max).into())])
            }
            WidgetHasNul { name, .. } => l.tr_with("session-widget-nul", &[("name", name.as_str().into())]),
            TooManyWidgets { max, .. } => l.tr_with("session-too-many-widgets", &[("max", (*max).into())]),
            WidgetInUse { name, uses, .. } => {
                l.tr_with("session-widget-in-use", &[("name", name.as_str().into()), ("uses", uses.join(", ").into())])
            }
            UnknownWidget { name, .. } => l.tr_with("session-unknown-widget", &[("name", name.as_str().into())]),
            WidgetInvalid { name, .. } => l.tr_with("session-widget-invalid", &[("name", name.as_str().into())]),
            TrialUndeclared { param, .. } => l.tr_with("session-trial-undeclared", &[("param", param.as_str().into())]),
            TrialFixed { param, .. } => l.tr_with("session-trial-fixed", &[("param", param.as_str().into())]),
            TrialTooLong { param, .. } => l.tr_with("session-trial-too-long", &[("param", param.as_str().into())]),
            TrialValue { param, source, .. } => l.tr_with(
                "session-trial-value",
                &[("param", param.as_str().into()), ("reason", source.localize(l).into())],
            ),
            TrialEncode { param, .. } => l.tr_with("session-trial-encode", &[("param", param.as_str().into())]),
            WidgetExists { name, .. } => l.tr_with("session-widget-exists", &[("name", name.as_str().into())]),
        }
    }
}
