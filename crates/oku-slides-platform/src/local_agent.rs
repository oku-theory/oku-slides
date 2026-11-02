//! The local fallback agent's engine state: machine, Apple's model, downloaded model, settings.
//!
//! [`local_engine`] is the one decision: the host and the child process both rely on it
//! to know whether the local agent can run, and on which engine.

use std::path::PathBuf;

pub use oku_slides_apple_model::AppleModel;

use crate::config::{LocalEngine, Settings};
use crate::models::PinnedFile;
use crate::storage::Storage;

/// Size of the model file in bytes, checked against the hub (`X-Linked-Size`).
pub const AGENT_MODEL_SIZE: u64 = 2_497_281_120;

/// The Qwen3 4B Instruct 2507 model (`Q4_K_M`), pinned to an audited revision.
pub const AGENT_MODEL: PinnedFile = PinnedFile {
    repo: "unsloth/Qwen3-4B-Instruct-2507-GGUF",
    revision: "a06e946bb6b655725eafa393f4a9745d460374c9",
    name: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
    sha256: Some("3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597"),
    size: AGENT_MODEL_SIZE,
};

/// Les fichiers à télécharger pour l'agent local (une tranche `'static`).
pub const AGENT_MODEL_FILES: [PinnedFile; 1] = [AGENT_MODEL];

/// Dossier du modèle de l'agent : `<modèles>/agent`.
#[must_use]
pub fn agent_model_dir(settings: &Settings) -> PathBuf {
    settings.paths.models().join("agent")
}

/// La machine sur laquelle tourne l'app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Mac à puce Apple : llama.cpp avec Metal.
    AppleSilicon,
    /// Mac Intel : moteur local non pris en charge.
    IntelMac,
    /// Tout système autre que macOS.
    Other,
}

impl Target {
    /// La machine de ce binaire.
    #[must_use]
    pub fn current() -> Self {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Self::AppleSilicon
        } else if cfg!(target_os = "macos") {
            Self::IntelMac
        } else {
            Self::Other
        }
    }
}

/// Un moteur prêt à être lancé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadyEngine {
    /// Apple's on-device model, nothing to download.
    Apple,
    /// llama.cpp avec ce fichier GGUF.
    Gguf {
        /// Chemin du fichier GGUF à charger.
        model: PathBuf,
    },
    /// Script rejoué (builds de débogage).
    #[cfg(debug_assertions)]
    Fake,
}

/// Pourquoi l'agent local ne peut pas tourner (encore).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// Mac Intel : aucun moteur local disponible.
    IntelMac,
    /// Système autre que macOS : aucun moteur local disponible.
    UnsupportedOs,
    /// Le modèle manque : il reste à le télécharger.
    NeedsDownload {
        /// Octets à télécharger.
        size: u64,
    },
    /// Apple Intelligence is turned off in System Settings.
    AppleIntelligenceOff,
    /// Apple's model is still downloading or preparing.
    AppleModelNotReady,
    /// Apple's model cannot run here: device not eligible or macOS older than 26.
    AppleUnsupported,
}

/// The engine for this setting, this machine, Apple's model and the downloaded model.
///
/// `apple` is asked only when the engine is `auto` or `apple`, and not outside macOS.
/// `auto` takes Apple's model when ready, else Gguf; on a machine without Gguf it gives
/// Apple's reason when the user can fix it (turned off, not ready yet).
///
/// # Errors
///
/// When the chosen engine cannot run here, with its reason; nothing falls back silently.
pub fn resolve(
    engine: LocalEngine,
    target: Target,
    apple: impl FnOnce() -> AppleModel,
    installed: bool,
    model: PathBuf,
) -> Result<ReadyEngine, Unavailable> {
    let gguf = || match target {
        Target::IntelMac => Err(Unavailable::IntelMac),
        Target::Other => Err(Unavailable::UnsupportedOs),
        Target::AppleSilicon if !installed => Err(Unavailable::NeedsDownload { size: AGENT_MODEL_SIZE }),
        Target::AppleSilicon => Ok(ReadyEngine::Gguf { model }),
    };
    match engine {
        #[cfg(debug_assertions)]
        LocalEngine::Fake => Ok(ReadyEngine::Fake),
        LocalEngine::Gguf => gguf(),
        _ if target == Target::Other => Err(Unavailable::UnsupportedOs),
        LocalEngine::Apple => match apple() {
            AppleModel::Ready => Ok(ReadyEngine::Apple),
            AppleModel::IntelligenceOff => Err(Unavailable::AppleIntelligenceOff),
            AppleModel::ModelNotReady => Err(Unavailable::AppleModelNotReady),
            AppleModel::DeviceNotEligible | AppleModel::MacOsTooOld | AppleModel::Unknown => {
                Err(Unavailable::AppleUnsupported)
            }
        },
        LocalEngine::Auto => match (apple(), target) {
            (AppleModel::Ready, _) => Ok(ReadyEngine::Apple),
            (AppleModel::IntelligenceOff, Target::IntelMac) => Err(Unavailable::AppleIntelligenceOff),
            (AppleModel::ModelNotReady, Target::IntelMac) => Err(Unavailable::AppleModelNotReady),
            _ => gguf(),
        },
    }
}

/// The local agent's engine, from the settings, Apple's model and the model on disk.
///
/// # Errors
///
/// Like [`resolve`]: unsupported machine, Apple's model unavailable, or missing model.
pub fn local_engine(settings: &Settings, storage: &Storage) -> Result<ReadyEngine, Unavailable> {
    let dir = agent_model_dir(settings);
    let installed = crate::models::is_complete(storage, &dir);
    let model = dir.join(AGENT_MODEL.name);
    resolve(settings.local_agent.engine, Target::current(), oku_slides_apple_model::availability, installed, model)
}

#[cfg(test)]
#[path = "local_agent_tests.rs"]
mod local_agent_tests;
