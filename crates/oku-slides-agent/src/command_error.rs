//! Commandes de l'interface refusées par la pile, dites dans la langue de l'utilisateur.
//!
//! Le message localisé ne cite aucune donnée de l'utilisateur ; l'erreur complète (tailles, agent, cause)
//! reste dans `Display`, pour les journaux.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::config_write::ConfigWriteError;
use snafu::{Backtrace, Snafu};

/// Une commande de l'interface refusée.
#[derive(Debug, Snafu)]
pub struct CommandError(CommandInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum CommandInner {
    #[snafu(display("draft of {bytes} bytes over the {max} bytes limit"))]
    DraftTooLong { bytes: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("draft origin of {bytes} bytes over the {max} bytes limit"))]
    OriginTooLong { bytes: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("draft for a tab whose agent has not started"))]
    NoAgent { backtrace: Backtrace },
    #[snafu(display("agent {id:?} is not in the configuration"))]
    UnknownAgent { id: String, backtrace: Backtrace },
    #[snafu(display("agent choice is set by OKU_SLIDES_AGENT__SELECTED"))]
    AgentLocked { backtrace: Backtrace },
    /// Cause boxée : l'erreur d'écriture (chemin, cause) grossirait toute `CommandError`.
    #[snafu(display("agent choice could not be saved"))]
    AgentNotSaved {
        #[snafu(source(from(ConfigWriteError, Box::new)))]
        source: Box<ConfigWriteError>,
        backtrace: Backtrace,
    },
}

impl Localize for CommandError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            CommandInner::DraftTooLong { .. } => l.tr("agent-error-draft-too-long"),
            CommandInner::OriginTooLong { .. } => l.tr("agent-error-draft-origin"),
            CommandInner::NoAgent { .. } => l.tr("agent-error-draft-no-agent"),
            CommandInner::UnknownAgent { .. } => l.tr("agent-error-unknown-agent"),
            CommandInner::AgentLocked { .. } => l.tr("agent-error-agent-locked"),
            CommandInner::AgentNotSaved { .. } => l.tr("agent-error-agent-not-saved"),
        }
    }
}

#[cfg(test)]
#[path = "command_error_tests.rs"]
mod command_error_tests;
