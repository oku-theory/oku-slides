//! État du moteur de l'agent local de repli : machine, modèle téléchargé, réglages.
//!
//! [`local_engine`] est la seule décision : l'hôte et le processus fils s'y fient
//! pour savoir si l'agent local peut tourner.

use std::path::PathBuf;

use crate::config::{LocalEngine, Settings};
use crate::models::PinnedFile;
use crate::storage::Storage;

/// Le modèle Qwen3 4B Instruct 2507 (Q4_K_M), épinglé à une révision auditée.
pub const AGENT_MODEL: PinnedFile = PinnedFile {
    repo: "unsloth/Qwen3-4B-Instruct-2507-GGUF",
    revision: "a06e946bb6b655725eafa393f4a9745d460374c9",
    name: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
    sha256: Some("3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597"),
};

/// Taille du fichier du modèle, en octets.
pub const AGENT_MODEL_SIZE: u64 = 2_497_281_120;

/// Les fichiers à télécharger pour l'agent local (une tranche `'static`).
pub const AGENT_MODEL_FILES: [PinnedFile; 1] = [AGENT_MODEL];

/// Dossier du modèle de l'agent : `<modèles>/agent`.
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
}

/// Le moteur pour ce réglage, cette machine et l'état du modèle.
pub fn resolve(
    engine: LocalEngine,
    target: Target,
    installed: bool,
    model: PathBuf,
) -> Result<ReadyEngine, Unavailable> {
    match engine {
        #[cfg(debug_assertions)]
        LocalEngine::Fake => Ok(ReadyEngine::Fake),
        // Le lot 4 ajoute Apple : `auto` le préférera quand il est disponible.
        LocalEngine::Auto | LocalEngine::Gguf => match target {
            Target::IntelMac => Err(Unavailable::IntelMac),
            Target::Other => Err(Unavailable::UnsupportedOs),
            Target::AppleSilicon if !installed => Err(Unavailable::NeedsDownload { size: AGENT_MODEL_SIZE }),
            Target::AppleSilicon => Ok(ReadyEngine::Gguf { model }),
        },
    }
}

/// Le moteur de l'agent local d'après les réglages et le modèle sur disque.
pub fn local_engine(settings: &Settings, storage: &Storage) -> Result<ReadyEngine, Unavailable> {
    let dir = agent_model_dir(settings);
    let installed = crate::models::is_complete(storage, &dir);
    resolve(settings.local_agent.engine, Target::current(), installed, dir.join(AGENT_MODEL.name))
}

#[cfg(test)]
#[path = "local_agent_tests.rs"]
mod local_agent_tests;
