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
}

impl Localize for AgentError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            AgentInner::NotFound { command, .. } | AgentInner::Spawn { command, .. } => {
                l.tr_with("agent-not-found", &[("command", command.as_str().into())])
            }
            AgentInner::NoHttp { .. } => l.tr("agent-no-http"),
            AgentInner::Stopped { .. } => l.tr("agent-stopped"),
            AgentInner::Request { message, .. } => l.tr_with("agent-failed", &[("message", message.as_str().into())]),
        }
    }
}
