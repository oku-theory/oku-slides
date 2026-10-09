//! Commandes de l'interface refusées par la pile, dites dans la langue de l'utilisateur.
//!
//! Le message localisé ne cite aucune donnée de l'utilisateur ; l'erreur complète (tailles)
//! reste dans `Display`, pour les journaux.

use oku_slides_i18n::{Localize, Localizer};
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
}

impl Localize for CommandError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            CommandInner::DraftTooLong { .. } => l.tr("agent-error-draft-too-long"),
            CommandInner::OriginTooLong { .. } => l.tr("agent-error-draft-origin"),
            CommandInner::NoAgent { .. } => l.tr("agent-error-draft-no-agent"),
        }
    }
}

#[cfg(test)]
#[path = "command_error_tests.rs"]
mod command_error_tests;
