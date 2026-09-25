//! Modèles Whisper au format CTranslate2 (celui de faster-whisper),
//! téléchargés à la demande depuis Hugging Face.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

const HF: &str = "https://huggingface.co";

/// Marqueur écrit quand tous les fichiers d'un modèle sont là.
const COMPLETE: &str = ".complete";

/// Modèles proposés. Tous sont multilingues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// large-v3-turbo quantifié int8 : très bon compromis qualité / vitesse.
    #[default]
    Turbo,
    /// large-v3 complet : la meilleure qualité, lourd.
    Large,
    /// base : léger et rapide, pour les petites machines.
    Base,
}

impl Model {
    pub const ALL: [Model; 3] = [Model::Turbo, Model::Large, Model::Base];

    /// Dépôt Hugging Face du modèle.
    pub fn repo(self) -> &'static str {
        match self {
            Model::Turbo => "Zoont/faster-whisper-large-v3-turbo-int8-ct2",
            Model::Large => "Systran/faster-whisper-large-v3",
            Model::Base => "Systran/faster-whisper-base",
        }
    }

    /// Fichiers à récupérer : (dépôt, fichier). Le dépôt `base` n'a pas de
    /// `preprocessor_config.json` : on prend celui d'OpenAI.
    fn files(self) -> Vec<(&'static str, &'static str)> {
        let repo = self.repo();
        // Le vocabulaire est en .txt dans le dépôt `base`, en .json ailleurs.
        let vocabulary = if self == Model::Base { "vocabulary.txt" } else { "vocabulary.json" };
        let preprocessor = match self {
            Model::Base => ("openai/whisper-base", "preprocessor_config.json"),
            _ => (repo, "preprocessor_config.json"),
        };
        vec![(repo, "config.json"), (repo, "tokenizer.json"), (repo, vocabulary), preprocessor, (repo, "model.bin")]
    }

    /// Taille totale attendue (octets), pour la progression.
    pub fn size(self) -> u64 {
        match self {
            Model::Turbo => 817_887_676,
            Model::Large => 3_090_835_702,
            Model::Base => 148_067_931,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Model::Turbo => "Large v3 Turbo (multilingue, 818 Mo)",
            Model::Large => "Large v3 (multilingue, 3,1 Go)",
            Model::Base => "Base (multilingue, 148 Mo)",
        }
    }

    /// Dossier du modèle.
    pub fn path(self, dir: &Path) -> PathBuf {
        dir.join(self.repo().replace('/', "--"))
    }

    pub fn is_installed(self, dir: &Path) -> bool {
        self.path(dir).join(COMPLETE).exists()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("téléchargement impossible : {0}")]
    Http(String),
    #[error("écriture du modèle : {0}")]
    Io(#[from] std::io::Error),
    #[error("fichier incomplet : {got} octets sur {expected}")]
    Incomplete { got: u64, expected: u64 },
    #[error("téléchargement annulé")]
    Cancelled,
}

/// Télécharge un modèle dans `dir` (chaque fichier en `.part` puis renommé ;
/// les fichiers déjà là sont gardés, un téléchargement interrompu reprend).
/// `progress(reçu, total)` est appelé régulièrement ; s'il renvoie `false`, on annule.
pub fn download(
    model: Model,
    dir: &Path,
    mut progress: impl FnMut(u64, u64) -> bool,
) -> Result<PathBuf, DownloadError> {
    let dest = model.path(dir);
    std::fs::create_dir_all(&dest)?;
    let agent: ureq::Agent = ureq::Agent::config_builder().proxy(ureq::Proxy::try_from_env()).build().into();
    let total = model.size();
    let mut done = 0u64;
    for (repo, file) in model.files() {
        let path = dest.join(file);
        if let Ok(meta) = std::fs::metadata(&path) {
            done += meta.len();
            continue;
        }
        let url = format!("{HF}/{repo}/resolve/main/{file}");
        // Hugging Face limite parfois le débit (429) : on réessaie en patientant.
        let mut attempt = 0;
        let mut resp = loop {
            match agent.get(&url).call() {
                Ok(r) => break r,
                Err(ureq::Error::StatusCode(429 | 500 | 502 | 503)) if attempt < 5 => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_secs(4 * attempt));
                }
                Err(e) => return Err(DownloadError::Http(format!("{file} : {e}"))),
            }
        };
        let expected: Option<u64> =
            resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
        let part = dest.join(format!("{file}.part"));
        let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
        let mut out = std::fs::File::create(&part)?;
        let mut buf = vec![0u8; 1 << 20];
        let mut got = 0u64;
        loop {
            let n = reader.read(&mut buf).map_err(|e| DownloadError::Http(e.to_string()))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            got += n as u64;
            if !progress((done + got).min(total), total) {
                drop(out);
                std::fs::remove_file(&part).ok();
                return Err(DownloadError::Cancelled);
            }
        }
        out.sync_all()?;
        if let Some(expected) = expected
            && got != expected
        {
            std::fs::remove_file(&part).ok();
            return Err(DownloadError::Incomplete { got, expected });
        }
        std::fs::rename(&part, &path)?;
        done += got;
    }
    std::fs::write(dest.join(COMPLETE), b"")?;
    progress(total, total);
    Ok(dest)
}
