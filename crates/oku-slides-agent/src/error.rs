//! Agent errors: not found, no app binary, no HTTP, stopped, request refused, no working folder.

use std::path::PathBuf;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::local_agent::Unavailable;
use oku_slides_platform::storage::StorageError;
use snafu::{Backtrace, Snafu};

/// L'agent n'a pas pu servir la consigne.
#[derive(Debug, Snafu)]
pub struct AgentError(pub(crate) AgentInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum AgentInner {
    #[snafu(display("agent command not found: {command}"))]
    NotFound { command: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("built-in agent without the app binary to run"))]
    NoProgram { backtrace: Backtrace },
    #[snafu(display("agent could not start: {command}"))]
    Spawn { command: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("the agent cannot reach MCP servers over HTTP"))]
    NoHttp { backtrace: Backtrace },
    #[snafu(display("the agent stopped: {message}"))]
    Stopped { message: String, backtrace: Backtrace },
    #[snafu(display("the agent refused the request: {message}"))]
    Request { message: String, backtrace: Backtrace },
    #[snafu(display("no working folder for the tab's agent"))]
    WorkDir { source: oku_slides_session::SessionError, backtrace: Backtrace },
    #[snafu(display("the tab's agent working folder was lost"))]
    WorkDirTask { source: tokio::task::JoinError, backtrace: Backtrace },
    #[snafu(display("cannot empty the agent process folder {}", path.display()))]
    ProcessDir { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("the agent process folder task was lost"))]
    ProcessDirTask { source: tokio::task::JoinError, backtrace: Backtrace },
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
            AgentInner::NoProgram { .. } => Some(Unusable::NoProgram),
            AgentInner::Stopped { .. }
            | AgentInner::Request { .. }
            | AgentInner::WorkDir { .. }
            | AgentInner::WorkDirTask { .. }
            | AgentInner::ProcessDir { .. }
            | AgentInner::ProcessDirTask { .. } => None,
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
    /// Agent local : le modèle de `size` octets reste à télécharger.
    LocalNeedsDownload { size: u64 },
    /// Agent local : il faut un Mac avec puce Apple.
    LocalIntelMac,
    /// Agent local : pas encore disponible sur ce système.
    LocalUnsupportedOs,
    /// Local agent: Apple Intelligence is turned off.
    LocalAppleOff,
    /// Local agent: Apple's model is still getting ready.
    LocalAppleNotReady,
    /// Local agent: Apple's model cannot run on this Mac (device or macOS).
    LocalAppleUnsupported,
}

impl From<Unavailable> for Unusable {
    fn from(unavailable: Unavailable) -> Self {
        match unavailable {
            Unavailable::IntelMac => Self::LocalIntelMac,
            Unavailable::UnsupportedOs => Self::LocalUnsupportedOs,
            Unavailable::NeedsDownload { size } => Self::LocalNeedsDownload { size },
            Unavailable::AppleIntelligenceOff => Self::LocalAppleOff,
            Unavailable::AppleModelNotReady => Self::LocalAppleNotReady,
            Unavailable::AppleUnsupported => Self::LocalAppleUnsupported,
        }
    }
}

impl Localize for Unusable {
    fn localize(&self, l: &Localizer) -> String {
        match self {
            Self::NotFound { command } => l.tr_with("agent-not-found", &[("command", command.as_str().into())]),
            Self::Spawn { command } => l.tr_with("agent-spawn-failed", &[("command", command.as_str().into())]),
            Self::NoHttp => l.tr("agent-no-http"),
            Self::NoProgram => l.tr("agent-local-no-program"),
            Self::LocalNeedsDownload { size } => l.tr_with(
                "agent-local-needs-download",
                &[("gigabytes", oku_slides_session::gigabytes_text(*size, l).into())],
            ),
            Self::LocalIntelMac => l.tr("agent-local-intel-mac"),
            Self::LocalUnsupportedOs => l.tr("agent-local-unsupported-os"),
            Self::LocalAppleOff => l.tr("agent-local-apple-off"),
            Self::LocalAppleNotReady => l.tr("agent-local-apple-not-ready"),
            Self::LocalAppleUnsupported => l.tr("agent-local-apple-unsupported"),
        }
    }
}

/// Why the selected agent cannot serve, localized at each use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reason {
    /// One of the reasons an agent is unusable.
    Unusable(Unusable),
    /// Its preset is missing from the settings (checked at load: logged as a bug).
    UnknownAgent { id: String },
}

impl From<Unusable> for Reason {
    fn from(unusable: Unusable) -> Self {
        Self::Unusable(unusable)
    }
}

impl Localize for Reason {
    fn localize(&self, l: &Localizer) -> String {
        match self {
            Self::Unusable(u) => u.localize(l),
            Self::UnknownAgent { id } => {
                let e = crate::command_error::UnknownAgentSnafu { id: id.clone() }.build();
                crate::command_error::CommandError::from(e).localize(l)
            }
        }
    }
}

impl Localize for AgentError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            AgentInner::NotFound { command, .. } => Unusable::NotFound { command: command.clone() }.localize(l),
            AgentInner::Spawn { command, .. } => Unusable::Spawn { command: command.clone() }.localize(l),
            AgentInner::NoHttp { .. } => Unusable::NoHttp.localize(l),
            AgentInner::NoProgram { .. } => Unusable::NoProgram.localize(l),
            AgentInner::Stopped { .. } => l.tr("agent-stopped"),
            AgentInner::Request { message, .. } => l.tr_with("agent-failed", &[("message", message.as_str().into())]),
            // The path stays in Display, for the logs: never quoted to the user.
            AgentInner::WorkDir { .. }
            | AgentInner::WorkDirTask { .. }
            | AgentInner::ProcessDir { .. }
            | AgentInner::ProcessDirTask { .. } => l.tr("agent-error-work-dir"),
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
