//! Modèles Whisper (format ggml de whisper.cpp), téléchargés à la demande.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Modèles proposés. Tous sont multilingues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// large-v3-turbo quantifié : très bon compromis qualité / vitesse.
    #[default]
    Turbo,
    /// large-v3 complet : la meilleure qualité, lourd.
    Large,
    /// base : léger et rapide, pour les petites machines.
    Base,
}

impl Model {
    pub const ALL: [Model; 3] = [Model::Turbo, Model::Large, Model::Base];

    pub fn file_name(self) -> &'static str {
        match self {
            Model::Turbo => "ggml-large-v3-turbo-q5_0.bin",
            Model::Large => "ggml-large-v3.bin",
            Model::Base => "ggml-base.bin",
        }
    }

    pub fn url(self) -> String {
        format!("{BASE_URL}/{}", self.file_name())
    }

    /// Taille attendue du fichier, pour vérifier le téléchargement.
    pub fn size(self) -> u64 {
        match self {
            Model::Turbo => 574_041_195,
            Model::Large => 3_095_033_483,
            Model::Base => 147_951_465,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Model::Turbo => "Large v3 Turbo (multilingue, 574 Mo)",
            Model::Large => "Large v3 (multilingue, 3,1 Go)",
            Model::Base => "Base (multilingue, 148 Mo)",
        }
    }

    pub fn path(self, dir: &Path) -> PathBuf {
        dir.join(self.file_name())
    }

    pub fn is_installed(self, dir: &Path) -> bool {
        std::fs::metadata(self.path(dir)).is_ok_and(|m| m.len() == self.size())
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

/// Télécharge un modèle dans `dir` (fichier `.part` puis renommage).
/// `progress(reçu, total)` est appelé régulièrement ; s'il renvoie `false`, on annule.
pub fn download(
    model: Model,
    dir: &Path,
    mut progress: impl FnMut(u64, u64) -> bool,
) -> Result<PathBuf, DownloadError> {
    std::fs::create_dir_all(dir)?;
    let dest = model.path(dir);
    let part = dir.join(format!("{}.part", model.file_name()));

    let agent: ureq::Agent = ureq::Agent::config_builder().proxy(ureq::Proxy::try_from_env()).build().into();
    let mut resp = agent.get(&model.url()).call().map_err(|e| DownloadError::Http(e.to_string()))?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(model.size());

    let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
    let mut file = std::fs::File::create(&part)?;
    let mut buf = vec![0u8; 1 << 20];
    let mut got = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| DownloadError::Http(e.to_string()))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        got += n as u64;
        if !progress(got, total) {
            drop(file);
            std::fs::remove_file(&part).ok();
            return Err(DownloadError::Cancelled);
        }
    }
    file.sync_all()?;
    if got != model.size() {
        std::fs::remove_file(&part).ok();
        return Err(DownloadError::Incomplete { got, expected: model.size() });
    }
    std::fs::rename(&part, &dest)?;
    Ok(dest)
}

// ---- Modèles CTranslate2 (faster-whisper), pour la comparaison ------------------

const HF: &str = "https://huggingface.co";

impl Model {
    /// Dépôt Hugging Face du même modèle au format CTranslate2.
    pub fn ct2_repo(self) -> &'static str {
        match self {
            Model::Turbo => "deepdml/faster-whisper-large-v3-turbo-ct2",
            Model::Large => "Systran/faster-whisper-large-v3",
            Model::Base => "Systran/faster-whisper-base",
        }
    }

    /// Fichiers à récupérer : (dépôt, fichier). Le dépôt `base` n'a pas de
    /// `preprocessor_config.json` : on prend celui d'OpenAI.
    fn ct2_files(self) -> Vec<(&'static str, &'static str)> {
        let repo = self.ct2_repo();
        // Le vocabulaire est en .txt dans le dépôt `base`, en .json ailleurs.
        let vocabulary = if self == Model::Base { "vocabulary.txt" } else { "vocabulary.json" };
        let mut files = vec![(repo, "config.json"), (repo, "tokenizer.json"), (repo, vocabulary), (repo, "model.bin")];
        files.push(match self {
            Model::Base => ("openai/whisper-base", "preprocessor_config.json"),
            _ => (repo, "preprocessor_config.json"),
        });
        files
    }

    pub fn ct2_dir(self, dir: &Path) -> PathBuf {
        dir.join("ct2").join(self.ct2_repo().replace('/', "--"))
    }
}

/// Télécharge (si besoin) le modèle CTranslate2 et renvoie son dossier.
pub fn download_ct2(
    model: Model,
    dir: &Path,
    mut progress: impl FnMut(&str, u64, u64),
) -> Result<PathBuf, DownloadError> {
    let dest = model.ct2_dir(dir);
    std::fs::create_dir_all(&dest)?;
    let agent: ureq::Agent = ureq::Agent::config_builder().proxy(ureq::Proxy::try_from_env()).build().into();
    for (repo, file) in model.ct2_files() {
        let path = dest.join(file);
        if path.exists() {
            continue;
        }
        let url = format!("{HF}/{repo}/resolve/main/{file}");
        // Hugging Face limite parfois le débit (429) : on réessaie en patientant.
        let mut attempt = 0;
        let mut resp = loop {
            match agent.get(&url).call() {
                Ok(r) => break r,
                Err(ureq::Error::StatusCode(429 | 502 | 503)) if attempt < 5 => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_secs(5 * attempt));
                }
                Err(e) => return Err(DownloadError::Http(format!("{url} : {e}"))),
            }
        };
        let total = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
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
            progress(file, got, total);
        }
        if total > 0 && got != total {
            std::fs::remove_file(&part).ok();
            return Err(DownloadError::Incomplete { got, expected: total });
        }
        std::fs::rename(&part, &path)?;
    }
    Ok(dest)
}
