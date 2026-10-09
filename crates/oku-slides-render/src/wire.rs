//! Erreurs de rendu et de registre, sérialisées pour traverser le tube du
//! processus de rendu (`oku-slides-render-worker`). Les backtraces restent
//! de leur côté ; nature, diagnostics et paramètres passent.

use crate::diagnostic::Diagnostic;
use crate::error::{
    BadPackIdSnafu, CompileSnafu, ImageSnafu, MissingSnafu, NoPageSnafu, PdfSnafu, RegistryCompileSnafu, RegistryInner,
    RemoteSchemaSnafu, RemoteSnafu, RenderInner,
};
use crate::{RegistryError, RenderError};

/// Une [`RenderError`] ou une [`RegistryError`] à travers le tube.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "error", rename_all = "kebab-case")]
pub enum WireError {
    Compile { diagnostics: Vec<Diagnostic> },
    Pdf { diagnostics: Vec<Diagnostic> },
    NoPage { index: usize },
    Image { message: String },
    Timeout { ms: u64 },
    OutOfMemory { mib: u64 },
    WorkerLost { message: String },
    Frame { message: String },
    Stale,
    PackDisabled { pack: String },
    Setup { message: String },
    RegistryBadPackId { id: String },
    RegistryCompile { diagnostics: Vec<Diagnostic> },
    RegistryMissing,
    RegistrySchema { message: String },
}

impl WireError {
    /// Une erreur de rendu, prête à être écrite dans le tube.
    pub fn from_render(e: &RenderError) -> Self {
        match &e.0 {
            RenderInner::Compile { diagnostics, .. } => Self::Compile { diagnostics: diagnostics.clone() },
            RenderInner::Pdf { diagnostics, .. } => Self::Pdf { diagnostics: diagnostics.clone() },
            RenderInner::NoPage { index, .. } => Self::NoPage { index: *index },
            RenderInner::Image { message, .. } => Self::Image { message: message.clone() },
            RenderInner::Timeout { ms, .. } => Self::Timeout { ms: *ms },
            RenderInner::OutOfMemory { mib, .. } => Self::OutOfMemory { mib: *mib },
            RenderInner::WorkerLost { message, .. } => Self::WorkerLost { message: message.clone() },
            RenderInner::Frame { message, .. } => Self::Frame { message: message.clone() },
            RenderInner::PageTooLarge { .. } => Self::Frame { message: first_line(&e.0) },
            RenderInner::Stale { .. } => Self::Stale,
            RenderInner::PackDisabled { pack, .. } => Self::PackDisabled { pack: pack.clone() },
            RenderInner::Remote { message, .. } => Self::Setup { message: message.clone() },
            other => Self::Setup { message: first_line(other) },
        }
    }

    /// Une erreur de registre, prête à être écrite dans le tube.
    pub fn from_registry(e: &RegistryError) -> Self {
        match &e.0 {
            RegistryInner::BadPackId { id, .. } => Self::RegistryBadPackId { id: id.clone() },
            RegistryInner::RegistryCompile { diagnostics, .. } => {
                Self::RegistryCompile { diagnostics: diagnostics.clone() }
            }
            RegistryInner::Missing { .. } => Self::RegistryMissing,
            RegistryInner::Schema { source, .. } => Self::RegistrySchema { message: source.to_string() },
            RegistryInner::RemoteSchema { message, .. } => Self::RegistrySchema { message: message.clone() },
            RegistryInner::Setup { source, .. } | RegistryInner::Worker { source, .. } => Self::from_render(source),
        }
    }

    /// L'erreur de rendu reconstruite de l'autre côté du tube.
    pub fn into_render(self) -> RenderError {
        match self {
            Self::Compile { diagnostics } => CompileSnafu { diagnostics }.build().into(),
            Self::Pdf { diagnostics } => PdfSnafu { diagnostics }.build().into(),
            Self::NoPage { index } => NoPageSnafu { index }.build().into(),
            Self::Image { message } => ImageSnafu { message }.build().into(),
            Self::Timeout { ms } => RenderError::timeout(ms),
            Self::OutOfMemory { mib } => RenderError::out_of_memory(mib),
            Self::WorkerLost { message } => RenderError::worker_lost(message),
            Self::Frame { message } => RenderError::frame(message),
            Self::Stale => RenderError::stale(),
            Self::PackDisabled { pack } => RenderError::pack_disabled(pack),
            Self::Setup { message } | Self::RegistrySchema { message } => RemoteSnafu { message }.build().into(),
            Self::RegistryBadPackId { id } => {
                RemoteSnafu { message: format!("invalid template pack identifier: {id}") }.build().into()
            }
            Self::RegistryCompile { diagnostics } => CompileSnafu { diagnostics }.build().into(),
            Self::RegistryMissing => {
                RemoteSnafu { message: "the widget probe emitted no registry".to_owned() }.build().into()
            }
        }
    }

    /// L'erreur de registre reconstruite de l'autre côté du tube.
    pub fn into_registry(self) -> RegistryError {
        match self {
            Self::RegistryBadPackId { id } => BadPackIdSnafu { id }.build().into(),
            Self::RegistryCompile { diagnostics } => RegistryCompileSnafu { diagnostics }.build().into(),
            Self::RegistryMissing => MissingSnafu.build().into(),
            Self::RegistrySchema { message } => RemoteSchemaSnafu { message }.build().into(),
            other => RegistryError::worker(other.into_render()),
        }
    }
}

/// Une ligne pour les variantes internes qui ne traversent pas le tube telles quelles.
fn first_line(inner: &RenderInner) -> String {
    inner.to_string().lines().next().unwrap_or_default().to_owned()
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod wire_tests;
