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
    /// Le graphique n'a pas pu changer de type.
    Chart,
    /// Un pack intégré ne se supprime pas.
    BuiltinTemplateSet,
    /// Pas de dossier pour les modèles de l'utilisateur.
    NoTemplatesDir,
    /// Pack de modèles inconnu.
    UnknownPack,
    /// Version de pack introuvable.
    PackVersionMissing,
    /// Requête impossible pendant l'édition des modèles.
    EditingTemplates,
    /// Requête qui demande l'édition des modèles.
    NotEditingTemplates,
    /// La réponse n'a pas pu être écrite en JSON.
    Response,
    /// Arguments d'une surface de l'agent invalides.
    AgentForm,
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
    #[snafu(display("chart conversion failed"))]
    Chart { source: oku_slides_chart::ChartError, backtrace: Backtrace },
    #[snafu(display("built-in template set {id} cannot be deleted"))]
    BuiltinTemplateSet { id: String, backtrace: Backtrace },
    #[snafu(display("no folder for user templates"))]
    NoTemplatesDir { backtrace: Backtrace },
    #[snafu(display("template pack not found: {id}"))]
    UnknownPack { id: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("version {version} of template pack {id} not found"))]
    PackVersionMissing { id: uuid::Uuid, version: oku_slides_core::PackVersion, backtrace: Backtrace },
    #[snafu(display("invalid pack.json for template pack {id}"))]
    PackJson { id: uuid::Uuid, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("pack.json of template pack {found} given for template pack {id}"))]
    PackIdMismatch { id: uuid::Uuid, found: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("finish editing the templates first"))]
    EditingTemplates { backtrace: Backtrace },
    #[snafu(display("no template set is being edited"))]
    NotEditingTemplates { backtrace: Backtrace },
    #[snafu(display("the response could not be written as JSON"))]
    Response { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("invalid surface: {reason}"))]
    AgentForm { reason: String, backtrace: Backtrace },
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
    #[snafu(display("unreadable version"))]
    Version { source: oku_slides_core::ReplicaError, backtrace: Backtrace },
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
            | Version { .. } => SessionErrorKind::InvalidDocument,
            Op { .. } => SessionErrorKind::Op,
            Render { .. } | NotRendered { .. } | Registry { .. } => SessionErrorKind::Render,
            NoPath { .. } => SessionErrorKind::NoPath,
            NoSlide { .. } => SessionErrorKind::NoSlide,
            NoChart { .. } => SessionErrorKind::NoChart,
            Chart { .. } => SessionErrorKind::Chart,
            BuiltinTemplateSet { .. } => SessionErrorKind::BuiltinTemplateSet,
            NoTemplatesDir { .. } => SessionErrorKind::NoTemplatesDir,
            UnknownPack { .. } => SessionErrorKind::UnknownPack,
            PackVersionMissing { .. } => SessionErrorKind::PackVersionMissing,
            EditingTemplates { .. } => SessionErrorKind::EditingTemplates,
            NotEditingTemplates { .. } => SessionErrorKind::NotEditingTemplates,
            Response { .. } => SessionErrorKind::Response,
            AgentForm { .. } => SessionErrorKind::AgentForm,
            UnknownTab { .. } => SessionErrorKind::UnknownTab,
            UnknownWindow { .. } => SessionErrorKind::UnknownWindow,
            SingleDocument { .. } => SessionErrorKind::SingleDocument,
            Unsaved { .. } => SessionErrorKind::Unsaved,
            MergeUnrelated { .. } => SessionErrorKind::MergeUnrelated,
            HistoryNotOpen { .. } => SessionErrorKind::HistoryNotOpen,
            InHistory { .. } => SessionErrorKind::InHistory,
        }
    }

    /// L'erreur de transaction, si une opération a été refusée.
    pub fn transaction(&self) -> Option<&oku_slides_core::TransactionError> {
        match &*self.0 {
            SessionInner::Op { source, .. } => Some(source),
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
            Chart { source, .. } => source.localize(l),
            BuiltinTemplateSet { .. } => l.tr("session-builtin-template-set"),
            NoTemplatesDir { .. } => l.tr("session-no-templates-dir"),
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
            EditingTemplates { .. } => l.tr("session-editing-templates"),
            NotEditingTemplates { .. } => l.tr("session-not-editing-templates"),
            Response { .. } => l.tr("session-response"),
            AgentForm { reason, .. } => l.tr_with("session-agent-form", &[("reason", reason.as_str().into())]),
            UnknownTab { .. } => l.tr("session-unknown-tab"),
            UnknownWindow { .. } => l.tr("session-unknown-window"),
            SingleDocument { .. } => l.tr("session-single-document"),
            Unsaved { title, .. } => l.tr_with("session-unsaved", &[("title", title.as_str().into())]),
            MergeUnrelated { .. } => l.tr("replica-unrelated"),
            HistoryNotOpen { .. } => l.tr("session-history-not-open"),
            InHistory { .. } => l.tr("session-in-history"),
            Version { source, .. } => source.localize(l),
        }
    }
}
