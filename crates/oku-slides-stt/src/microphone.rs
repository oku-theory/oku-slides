//! Microphone permission and model files, as diagnosis reads them.

use oku_slides_platform::models::is_complete;
use serde::Serialize;

use crate::{Model, Status, Stt};

/// Whether the app may use the microphone; the permission is never requested here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Microphone {
    /// Never asked yet: the first dictation asks.
    Undetermined,
    /// Blocked by a policy the user cannot change.
    Restricted,
    /// Refused by the user (System Settings, Privacy).
    Denied,
    /// Allowed.
    Granted,
    /// Not known on this system (off macOS, or a status the bridge does not know).
    Unknown,
}

impl Microphone {
    /// Its name, the same as its serde name.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Undetermined => "undetermined",
            Self::Restricted => "restricted",
            Self::Denied => "denied",
            Self::Granted => "granted",
            Self::Unknown => "unknown",
        }
    }

    /// The permission of a bridge code: 0 undetermined, 1 restricted, 2 denied, 3 granted.
    ///
    /// The same table as `oku_speech_microphone` in the Swift bridge; any other code is `Unknown`.
    #[cfg(any(test, target_os = "macos"))]
    pub(crate) fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Undetermined,
            1 => Self::Restricted,
            2 => Self::Denied,
            3 => Self::Granted,
            _ => Self::Unknown,
        }
    }
}

/// Whether a dictation model's files are on the disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFiles {
    /// Nothing there, and no download of it running.
    Missing,
    /// A download of it runs, or a `.part` file of it is left.
    Partial,
    /// Ready: files verified, Apple language installed, or API key present.
    Complete,
}

impl ModelFiles {
    /// Its name, the same as its serde name.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Partial => "partial",
            Self::Complete => "complete",
        }
    }
}

impl Stt {
    /// The microphone permission, read without asking; `Unknown` off macOS.
    #[must_use]
    pub fn microphone(&self) -> Microphone {
        #[cfg(target_os = "macos")]
        {
            Microphone::from_code(crate::apple::microphone_code())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Microphone::Unknown
        }
    }

    /// The state of the chosen model's files in `status`. Reads the disk.
    ///
    /// Apple and the API have no files: Apple is `Complete` once its language is installed, the
    /// API once its key variable is set, `Missing` otherwise (never `Partial`).
    #[must_use]
    pub fn model_files(&self, status: &Status) -> ModelFiles {
        let model = status.config.model;
        let installed = status.models.iter().any(|m| m.id == model && m.installed);
        if installed
            || (!matches!(model, Model::Apple | Model::Api) && is_complete(&self.storage, &model.path(&self.dir)))
        {
            return ModelFiles::Complete;
        }
        if status.download.as_ref().is_some_and(|d| d.model == model) || self.has_part(model) {
            return ModelFiles::Partial;
        }
        ModelFiles::Missing
    }

    /// True if a `.part` file of `model` is left in its folder.
    fn has_part(&self, model: Model) -> bool {
        if matches!(model, Model::Apple | Model::Api) {
            return false;
        }
        match self.storage.list_files(&model.path(&self.dir)) {
            Ok(files) => files.iter().any(|f| f.extension().is_some_and(|e| e == "part")),
            Err(e) => {
                tracing::debug!(error = %e, "model folder not listed");
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "microphone_tests.rs"]
mod microphone_tests;
