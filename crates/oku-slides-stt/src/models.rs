//! Dictation models: Parakeet TDT 0.6B v3 in ONNX (int8), downloaded on demand from
//! Hugging Face, pinned by commit and SHA-256; Apple and the API have nothing to download.

use std::path::{Path, PathBuf};

use oku_slides_i18n::Localizer;
use oku_slides_platform::config::ModelsSettings;
use oku_slides_platform::models::{self, PinnedFile};
use oku_slides_platform::storage::Storage;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::api::ApiConfig;
use crate::error::{DownloadSnafu, NotDownloadableSnafu, SttError};

/// Hugging Face repository of the Parakeet ONNX export.
const PARAKEET_REPO: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";

/// Commit of [`PARAKEET_REPO`] the files are pinned to.
const PARAKEET_REVISION: &str = "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce";

/// Languages Parakeet v3 transcribes (ISO 639-1); it detects which one on its own.
pub const PARAKEET_LANGUAGES: [&str; 25] = [
    "bg", "cs", "da", "de", "el", "en", "es", "et", "fi", "fr", "hr", "hu", "it", "lt", "lv", "mt", "nl", "pl", "pt",
    "ro", "ru", "sk", "sl", "sv", "uk",
];

/// True if Parakeet transcribes `language`, a BCP 47 tag matched on its primary subtag.
pub(crate) fn parakeet_supports(language: &str) -> bool {
    let primary = language.split(['-', '_']).next().unwrap_or_default();
    PARAKEET_LANGUAGES.iter().any(|l| l.eq_ignore_ascii_case(primary))
}

/// Offered models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// Apple's engine (macOS 26+): streaming, language installed by the system.
    Apple,
    /// Parakeet TDT 0.6B v3, int8: 25 European languages, on the CPU.
    #[default]
    Parakeet,
    /// OpenAI-compatible transcription API: phrase by phrase, no preview.
    Api,
}

impl Model {
    /// Every model, in display order.
    pub const ALL: [Model; 3] = [Model::Apple, Model::Parakeet, Model::Api];

    /// The model's id, the same as its serde name.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Model::Apple => "apple",
            Model::Parakeet => "parakeet",
            Model::Api => "api",
        }
    }

    /// The model with id `id`; `None` if unknown.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Model> {
        Model::ALL.into_iter().find(|m| m.id() == id)
    }

    /// The model's Hugging Face repository.
    #[must_use]
    pub fn repo(self) -> &'static str {
        match self {
            Model::Apple => "apple",
            Model::Parakeet => PARAKEET_REPO,
            Model::Api => "api",
        }
    }

    /// Files to fetch, pinned by commit, size and SHA-256.
    fn files(self) -> Vec<PinnedFile> {
        if self != Model::Parakeet {
            return Vec::new();
        }
        let file = |name, sha256, size| PinnedFile {
            repo: PARAKEET_REPO,
            revision: PARAKEET_REVISION,
            name,
            sha256: Some(sha256),
            size,
        };
        vec![
            file(
                "encoder-model.int8.onnx",
                "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
                652_183_999,
            ),
            file(
                "decoder_joint-model.int8.onnx",
                "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
                18_202_004,
            ),
            file("vocab.txt", "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d", 93_939),
        ]
    }

    /// Expected total size in bytes, for the progress bar.
    #[must_use]
    pub fn size(self) -> u64 {
        match self {
            Model::Apple | Model::Api => 0,
            Model::Parakeet => 670_479_942,
        }
    }

    /// Display name, in the user's language; the API shows its model.
    pub fn label(self, l: &Localizer, api: &ApiConfig) -> String {
        match self {
            Model::Apple => l.tr("stt-model-apple"),
            Model::Parakeet => l.tr("stt-model-parakeet"),
            Model::Api => l.tr_with("stt-model-api", &[("model", api.model.as_str().into())]),
        }
    }

    /// True if the engine transcribes previews of the current phrase.
    pub(crate) fn previews(self) -> bool {
        // A preview through the API would cost one more request per preview step.
        self != Model::Api
    }

    /// The model's folder.
    #[must_use]
    pub fn path(self, dir: &Path) -> PathBuf {
        dir.join(self.repo().replace('/', "--"))
    }

    /// True if the model's files are on disk (never for Apple or the API).
    #[must_use]
    pub fn is_installed(self, storage: &Storage, dir: &Path) -> bool {
        !matches!(self, Model::Apple | Model::Api) && models::is_complete(storage, &self.path(dir))
    }
}

/// Downloads a model into `dir`.
///
/// Each file is written as `.part` then renamed; files already there are kept,
/// and an interrupted download resumes.
/// `progress(received, total)` is called regularly; if it returns `false`, the download is cancelled.
///
/// # Errors
///
/// For Apple and the API (nothing to download), on a network or disk failure, a file
/// with an unexpected fingerprint, or a cancellation by `progress`.
pub fn download(
    model: Model,
    dir: &Path,
    settings: &ModelsSettings,
    storage: &Storage,
    progress: impl FnMut(u64, u64) -> bool,
) -> Result<PathBuf, SttError> {
    ensure!(!matches!(model, Model::Apple | Model::Api), NotDownloadableSnafu);
    let dest = model.path(dir);
    models::download(&model.files(), &dest, model.size(), settings, storage, progress).context(DownloadSnafu)?;
    Ok(dest)
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
