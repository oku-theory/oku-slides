//! Modèles de dictée : Whisper au format CTranslate2 (celui de faster-whisper),
//! téléchargés à la demande depuis Hugging Face ; Apple et l'API n'ont rien à télécharger.

use std::path::{Path, PathBuf};

use oku_slides_i18n::Localizer;
use oku_slides_platform::config::ModelsSettings;
use oku_slides_platform::models::{self, PinnedFile};
use oku_slides_platform::storage::Storage;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::api::ApiConfig;
use crate::error::{DownloadSnafu, NotDownloadableSnafu, SttError};

/// Modèles proposés. Tous sont multilingues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// Moteur d'Apple (macOS 26+) : en continu, langue installée par le système.
    Apple,
    /// large-v3-turbo quantifié int8 : très bon compromis qualité / vitesse.
    #[default]
    Turbo,
    /// large-v3 complet : la meilleure qualité, lourd.
    Large,
    /// base : léger et rapide, pour les petites machines.
    Base,
    /// API de transcription compatible OpenAI : phrase par phrase, sans aperçu.
    Api,
}

impl Model {
    /// Tous les modèles, dans l'ordre affiché.
    pub const ALL: [Model; 5] = [Model::Apple, Model::Turbo, Model::Large, Model::Base, Model::Api];

    /// Identifiant du modèle, le même que son nom serde.
    pub fn id(self) -> &'static str {
        match self {
            Model::Apple => "apple",
            Model::Turbo => "turbo",
            Model::Large => "large",
            Model::Base => "base",
            Model::Api => "api",
        }
    }

    /// Le modèle d'identifiant `id` ; `None` s'il est inconnu.
    pub fn from_id(id: &str) -> Option<Model> {
        Model::ALL.into_iter().find(|m| m.id() == id)
    }

    /// Dépôt Hugging Face du modèle.
    pub fn repo(self) -> &'static str {
        match self {
            Model::Apple => "apple",
            Model::Turbo => "Zoont/faster-whisper-large-v3-turbo-int8-ct2",
            Model::Large => "Systran/faster-whisper-large-v3",
            Model::Base => "Systran/faster-whisper-base",
            Model::Api => "api",
        }
    }

    /// Fichiers à récupérer, non épinglés (branche `main`). Le dépôt `base` n'a pas de
    /// `preprocessor_config.json` : on prend celui d'OpenAI.
    fn files(self) -> Vec<PinnedFile> {
        if matches!(self, Model::Apple | Model::Api) {
            return Vec::new();
        }
        let repo = self.repo();
        // Le vocabulaire est en .txt dans le dépôt `base`, en .json ailleurs.
        let vocabulary = if self == Model::Base { "vocabulary.txt" } else { "vocabulary.json" };
        let preprocessor_repo = if self == Model::Base { "openai/whisper-base" } else { repo };
        let file = |repo, name| PinnedFile { repo, revision: "main", name, sha256: None };
        vec![
            file(repo, "config.json"),
            file(repo, "tokenizer.json"),
            file(repo, vocabulary),
            file(preprocessor_repo, "preprocessor_config.json"),
            file(repo, "model.bin"),
        ]
    }

    /// Taille totale attendue (octets), pour la progression.
    pub fn size(self) -> u64 {
        match self {
            Model::Apple | Model::Api => 0,
            Model::Turbo => 817_887_676,
            Model::Large => 3_090_835_702,
            Model::Base => 148_067_931,
        }
    }

    /// Nom affiché, dans la langue de l'utilisateur ; l'API montre son modèle.
    pub fn label(self, l: &Localizer, api: &ApiConfig) -> String {
        match self {
            Model::Apple => l.tr("stt-model-apple"),
            Model::Turbo => l.tr("stt-model-turbo"),
            Model::Large => l.tr("stt-model-large"),
            Model::Base => l.tr("stt-model-base"),
            Model::Api => l.tr_with("stt-model-api", &[("model", api.model.as_str().into())]),
        }
    }

    /// Vrai si le moteur transcrit des aperçus de la phrase en cours.
    pub(crate) fn previews(self) -> bool {
        // Un aperçu par l'API coûterait une requête de plus par pas d'aperçu.
        self != Model::Api
    }

    /// Dossier du modèle.
    pub fn path(self, dir: &Path) -> PathBuf {
        dir.join(self.repo().replace('/', "--"))
    }

    /// Vrai si les fichiers du modèle sont sur le disque (jamais pour Apple ni l'API).
    pub fn is_installed(self, storage: &Storage, dir: &Path) -> bool {
        !matches!(self, Model::Apple | Model::Api) && models::is_complete(storage, &self.path(dir))
    }
}

/// Télécharge un modèle dans `dir` (chaque fichier en `.part` puis renommé ;
/// les fichiers déjà là sont gardés, un téléchargement interrompu reprend).
/// `progress(reçu, total)` est appelé régulièrement ; s'il renvoie `false`, on annule.
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
