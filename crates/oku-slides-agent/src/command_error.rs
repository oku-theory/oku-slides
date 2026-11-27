//! Commandes de l'interface refusées par la pile, dites dans la langue de l'utilisateur.
//!
//! Le message localisé ne cite aucune donnée de l'utilisateur ; l'erreur complète (tailles, agent, cause)
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
    #[snafu(display("agent {id:?} is not in the configuration"))]
    UnknownAgent { id: String, backtrace: Backtrace },
    #[snafu(display("draft already holds {max} images"))]
    TooManyImages { max: usize, backtrace: Backtrace },
    #[snafu(display("draft already holds {max} documents"))]
    TooManyDocuments { max: usize, backtrace: Backtrace },
    #[snafu(display("attachment data is not base64"))]
    AttachmentNotBase64 { source: base64::DecodeError, backtrace: Backtrace },
    #[snafu(display("attachment data of {len} base64 bytes passes the {max} allowed"))]
    AttachmentTooLarge { len: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("attachment {id} is no longer kept"))]
    AttachmentGone { id: String, backtrace: Backtrace },
    #[snafu(display("a message cites more than {max} attachments"))]
    TooManyAttachments { max: usize, backtrace: Backtrace },
    #[snafu(display("page report from a window that does not hold the tab"))]
    ReportElsewhere { backtrace: Backtrace },
    #[snafu(display("page report over its size limits"))]
    UiTooLarge { backtrace: Backtrace },
    #[snafu(display("page report number {seq} is out of range"))]
    SeqExhausted { seq: u64, backtrace: Backtrace },
    /// Boxed cause: the session error would make every `CommandError` larger.
    #[snafu(display("attachment refused by the session"))]
    PasteRefused {
        #[snafu(source(from(oku_slides_session::SessionError, Box::new)))]
        source: Box<oku_slides_session::SessionError>,
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
            CommandInner::TooManyImages { .. } => l.tr("agent-error-too-many-images"),
            CommandInner::TooManyDocuments { max, .. } => {
                l.tr_with("agent-error-too-many-documents", &[("max", oku_slides_core::num::count_u64(*max).into())])
            }
            CommandInner::AttachmentNotBase64 { .. } => l.tr("agent-error-attachment-not-base64"),
            CommandInner::AttachmentTooLarge { .. } => l.tr("agent-error-attachment-too-large"),
            CommandInner::AttachmentGone { .. } => l.tr("agent-error-attachment-gone"),
            CommandInner::TooManyAttachments { max, .. } => {
                l.tr_with("agent-error-too-many-attachments", &[("max", oku_slides_core::num::count_u64(*max).into())])
            }
            CommandInner::ReportElsewhere { .. } => l.tr("agent-error-report-elsewhere"),
            CommandInner::UiTooLarge { .. } => l.tr("agent-error-ui-too-large"),
            CommandInner::SeqExhausted { .. } => l.tr("agent-error-seq-exhausted"),
            CommandInner::PasteRefused { source, .. } => source.localize(l),
        }
    }
}

#[cfg(test)]
#[path = "command_error_tests.rs"]
mod command_error_tests;
