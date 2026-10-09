//! Erreurs des actions de surface ([`SurfaceHub::act`](super::SurfaceHub::act)) : l'interface
//! affiche le texte reçu, dit dans la langue de l'utilisateur.
//!
//! Le message localisé ne reprend que des arguments sûrs (le nom d'une action) ; l'erreur
//! complète (identifiant de surface, cause de la session) reste dans `Display`, pour les journaux.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::SessionError;
use snafu::{Backtrace, Snafu};

use crate::error::say_session;

/// Une action de surface refusée.
#[derive(Debug, Snafu)]
pub struct HubError(HubInner);

/// La famille d'une [`HubError`], pour les tests et les appelants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubErrorKind {
    /// La surface n'est plus ouverte (répondue, fermée, élaguée) ou n'a jamais existé.
    UnknownSurface,
    /// Les suites d'un tour passé ont expiré.
    Expired,
    /// Action que cette surface ne connaît pas.
    UnexpectedAction,
    /// Il manque à l'action l'index qu'elle désigne.
    Incomplete,
    /// Le réglage désigné n'existe pas.
    NoField,
    /// La variante désignée n'existe pas.
    NoVariant,
    /// La session a refusé l'action (valeur refusée, slide disparue, onglet fermé).
    Session,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub(super) enum HubInner {
    #[snafu(display("unknown surface {surface}"))]
    UnknownSurface { surface: String, backtrace: Backtrace },
    #[snafu(display("the actions of a past turn have expired"))]
    Expired { backtrace: Backtrace },
    #[snafu(display("unexpected action {action}"))]
    UnexpectedAction { action: String, backtrace: Backtrace },
    #[snafu(display("missing {key}"))]
    Incomplete { key: String, backtrace: Backtrace },
    #[snafu(display("no field {index}"))]
    NoField { index: usize, backtrace: Backtrace },
    #[snafu(display("no variant {index}"))]
    NoVariant { index: usize, backtrace: Backtrace },
    #[snafu(display("the session refused the action"))]
    Session { source: SessionError, backtrace: Backtrace },
}

impl HubError {
    /// La surface `surface` n'est pas (plus) ouverte : aussi pour l'onglet sans agent, côté hôte.
    pub fn unknown_surface(surface: impl Into<String>) -> Self {
        UnknownSurfaceSnafu { surface: surface.into() }.build().into()
    }

    pub fn kind(&self) -> HubErrorKind {
        match &self.0 {
            HubInner::UnknownSurface { .. } => HubErrorKind::UnknownSurface,
            HubInner::Expired { .. } => HubErrorKind::Expired,
            HubInner::UnexpectedAction { .. } => HubErrorKind::UnexpectedAction,
            HubInner::Incomplete { .. } => HubErrorKind::Incomplete,
            HubInner::NoField { .. } => HubErrorKind::NoField,
            HubInner::NoVariant { .. } => HubErrorKind::NoVariant,
            HubInner::Session { .. } => HubErrorKind::Session,
        }
    }
}

impl Localize for HubError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            HubInner::UnknownSurface { .. } => l.tr("agent-error-unknown-surface"),
            HubInner::Expired { .. } => l.tr("agent-error-expired"),
            HubInner::UnexpectedAction { action, .. } => {
                l.tr_with("agent-error-unexpected-action", &[("action", action.as_str().into())])
            }
            HubInner::Incomplete { .. } => l.tr("agent-error-incomplete"),
            HubInner::NoField { .. } => l.tr("agent-error-no-field"),
            HubInner::NoVariant { .. } => l.tr("agent-error-no-variant"),
            HubInner::Session { source, .. } => say_session(source, l),
        }
    }
}
