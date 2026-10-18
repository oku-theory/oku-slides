//! Erreurs de l'agent : introuvable, sans HTTP, arrêté, requête refusée.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// L'agent n'a pas pu servir la consigne.
#[derive(Debug, Snafu)]
pub struct AgentError(pub(crate) AgentInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum AgentInner {
    #[snafu(display("agent command not found: {command}"))]
    NotFound { command: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("agent could not start: {command}"))]
    Spawn { command: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("the agent cannot reach MCP servers over HTTP"))]
    NoHttp { backtrace: Backtrace },
    #[snafu(display("the agent stopped: {message}"))]
    Stopped { message: String, backtrace: Backtrace },
    #[snafu(display("the agent refused the request: {message}"))]
    Request { message: String, backtrace: Backtrace },
}

impl AgentError {
    /// L'erreur et ses causes, sur une ligne (journaux).
    pub fn one_line(&self) -> String {
        oku_slides_platform::errors::one_line(self)
    }

    /// Pourquoi l'agent est inutilisable, si cette erreur le dit.
    ///
    /// Un arrêt (`Stopped`, annulation comprise) ou une requête refusée ne le rend pas inutilisable.
    pub(crate) fn unusable(&self) -> Option<Unusable> {
        match &self.0 {
            AgentInner::NotFound { command, .. } => Some(Unusable::NotFound { command: command.clone() }),
            AgentInner::Spawn { command, .. } => Some(Unusable::Spawn { command: command.clone() }),
            AgentInner::NoHttp { .. } => Some(Unusable::NoHttp),
            AgentInner::Stopped { .. } | AgentInner::Request { .. } => None,
        }
    }
}

/// Pourquoi l'agent choisi ne peut pas servir.
///
/// `Link` retient `Spawn` et `NoHttp` jusqu'au prochain succès, au plus `FAILURE_TTL`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unusable {
    /// Son programme est introuvable.
    NotFound { command: String },
    /// Son programme existe mais ne se lance pas (droits, format).
    Spawn { command: String },
    /// Il ne sait pas joindre les outils MCP en HTTP.
    NoHttp,
    /// Agent intégré, mais ce processus n'est pas l'app (tests, outils).
    NoProgram,
}

impl Localize for Unusable {
    fn localize(&self, l: &Localizer) -> String {
        match self {
            Self::NotFound { command } => l.tr_with("agent-not-found", &[("command", command.as_str().into())]),
            Self::Spawn { command } => l.tr_with("agent-spawn-failed", &[("command", command.as_str().into())]),
            Self::NoHttp => l.tr("agent-no-http"),
            Self::NoProgram => l.tr("agent-local-no-program"),
        }
    }
}

impl Localize for AgentError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            AgentInner::NotFound { command, .. } => Unusable::NotFound { command: command.clone() }.localize(l),
            AgentInner::Spawn { command, .. } => Unusable::Spawn { command: command.clone() }.localize(l),
            AgentInner::NoHttp { .. } => Unusable::NoHttp.localize(l),
            AgentInner::Stopped { .. } => l.tr("agent-stopped"),
            AgentInner::Request { message, .. } => l.tr_with("agent-failed", &[("message", message.as_str().into())]),
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
