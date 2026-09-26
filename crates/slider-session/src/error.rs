//! Erreurs de la session : fichiers, rendu, opérations refusées, requêtes
//! impossibles dans l'état courant.

use std::path::PathBuf;

use slider_i18n::{Localize, Localizer};
use slider_platform::storage::StorageError;
use snafu::{Backtrace, Snafu};

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
    /// Ce PDF ne contient pas de sources Écho.
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
    /// Ensemble de modèles introuvable.
    NoTemplateSet,
    /// Un ensemble intégré ne se supprime pas.
    BuiltinTemplateSet,
    /// Pas de dossier pour les modèles de l'utilisateur.
    NoTemplatesDir,
    /// Requête impossible pendant l'édition des modèles.
    EditingTemplates,
    /// Requête qui demande l'édition des modèles.
    NotEditingTemplates,
    /// La réponse n'a pas pu être écrite en JSON.
    Response,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum SessionInner {
    #[snafu(display("cannot read {}", path.display()))]
    Read { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Write { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("this PDF was not created by Écho: it cannot be edited"))]
    NotOkuSlides { backtrace: Backtrace },
    #[snafu(display("unreadable PDF"))]
    Pdf { source: slider_pdf::PdfError, backtrace: Backtrace },
    #[snafu(display("unreadable sources"))]
    Sources { source: slider_core::SourcesError, backtrace: Backtrace },
    #[snafu(display("invalid deck JSON"))]
    DeckJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("invalid template set {}", path.display()))]
    TemplateJson { path: PathBuf, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("transaction refused"))]
    Op { source: slider_core::TransactionError, backtrace: Backtrace },
    #[snafu(display("rendering failed"))]
    Render { source: slider_render::RenderError, backtrace: Backtrace },
    #[snafu(display("rendering unavailable"))]
    NotRendered { backtrace: Backtrace },
    #[snafu(display("no file: choose where to save"))]
    NoPath { backtrace: Backtrace },
    #[snafu(display("slide not found: {id}"))]
    NoSlide { id: String, backtrace: Backtrace },
    #[snafu(display("not a chart: {id}"))]
    NoChart { id: String, backtrace: Backtrace },
    #[snafu(display("chart conversion failed"))]
    Chart { source: slider_chart::ChartError, backtrace: Backtrace },
    #[snafu(display("template set not found: {id}"))]
    NoTemplateSet { id: String, backtrace: Backtrace },
    #[snafu(display("built-in template set {id} cannot be deleted"))]
    BuiltinTemplateSet { id: String, backtrace: Backtrace },
    #[snafu(display("no folder for user templates"))]
    NoTemplatesDir { backtrace: Backtrace },
    #[snafu(display("finish editing the templates first"))]
    EditingTemplates { backtrace: Backtrace },
    #[snafu(display("no template set is being edited"))]
    NotEditingTemplates { backtrace: Backtrace },
    #[snafu(display("the response could not be written as JSON"))]
    Response { source: serde_json::Error, backtrace: Backtrace },
}

impl SessionError {
    pub fn kind(&self) -> SessionErrorKind {
        use SessionInner::*;
        match *self.0 {
            Read { .. } | Write { .. } => SessionErrorKind::File,
            NotOkuSlides { .. } => SessionErrorKind::NotOkuSlides,
            Pdf { .. } | Sources { .. } | DeckJson { .. } | TemplateJson { .. } => SessionErrorKind::InvalidDocument,
            Op { .. } => SessionErrorKind::Op,
            Render { .. } | NotRendered { .. } => SessionErrorKind::Render,
            NoPath { .. } => SessionErrorKind::NoPath,
            NoSlide { .. } => SessionErrorKind::NoSlide,
            NoChart { .. } => SessionErrorKind::NoChart,
            Chart { .. } => SessionErrorKind::Chart,
            NoTemplateSet { .. } => SessionErrorKind::NoTemplateSet,
            BuiltinTemplateSet { .. } => SessionErrorKind::BuiltinTemplateSet,
            NoTemplatesDir { .. } => SessionErrorKind::NoTemplatesDir,
            EditingTemplates { .. } => SessionErrorKind::EditingTemplates,
            NotEditingTemplates { .. } => SessionErrorKind::NotEditingTemplates,
            Response { .. } => SessionErrorKind::Response,
        }
    }

    /// L'erreur de transaction, si une opération a été refusée.
    pub fn transaction(&self) -> Option<&slider_core::TransactionError> {
        match &*self.0 {
            SessionInner::Op { source, .. } => Some(source),
            _ => None,
        }
    }

    /// L'erreur et ses causes, sur une ligne.
    pub fn one_line(&self) -> String {
        slider_platform::errors::one_line(self)
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
            TemplateJson { path, .. } => l.tr_with("session-invalid-template-set", &path_arg(path)),
            Op { source, .. } => source.localize(l),
            Render { source, .. } => source.localize(l),
            NotRendered { .. } => l.tr("session-not-rendered"),
            NoPath { .. } => l.tr("session-no-path"),
            NoSlide { id, .. } => l.tr_with("session-no-slide", &[("id", id.as_str().into())]),
            NoChart { id, .. } => l.tr_with("op-not-a-chart", &[("subject", id.as_str().into())]),
            Chart { source, .. } => source.localize(l),
            NoTemplateSet { id, .. } => l.tr_with("session-no-template-set", &[("id", id.as_str().into())]),
            BuiltinTemplateSet { .. } => l.tr("session-builtin-template-set"),
            NoTemplatesDir { .. } => l.tr("session-no-templates-dir"),
            EditingTemplates { .. } => l.tr("session-editing-templates"),
            NotEditingTemplates { .. } => l.tr("session-not-editing-templates"),
            Response { .. } => l.tr("session-response"),
        }
    }
}
