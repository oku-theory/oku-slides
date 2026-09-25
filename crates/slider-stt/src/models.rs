//! Modèles Whisper au format CTranslate2 (celui de faster-whisper),
//! téléchargés à la demande depuis Hugging Face.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use slider_platform::config::SttSettings;
use slider_platform::storage::Storage;
use snafu::ResultExt;

use crate::error::{CancelledSnafu, HttpReadSnafu, HttpSnafu, IncompleteSnafu, ModelWriteSnafu, SttError};

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

    pub fn is_installed(self, storage: &Storage, dir: &Path) -> bool {
        storage.exists(&self.path(dir).join(COMPLETE)).unwrap_or(false)
    }
}

/// Télécharge un modèle dans `dir` (chaque fichier en `.part` puis renommé ;
/// les fichiers déjà là sont gardés, un téléchargement interrompu reprend).
/// `progress(reçu, total)` est appelé régulièrement ; s'il renvoie `false`, on annule.
pub fn download(
    model: Model,
    dir: &Path,
    settings: &SttSettings,
    storage: &Storage,
    mut progress: impl FnMut(u64, u64) -> bool,
) -> Result<PathBuf, SttError> {
    let dest = model.path(dir);
    let agent: ureq::Agent = ureq::Agent::config_builder().proxy(ureq::Proxy::try_from_env()).build().into();
    let total = model.size();
    let mut done = 0u64;
    for (repo, file) in model.files() {
        let path = dest.join(file);
        if let Ok(Some(len)) = storage.len(&path) {
            done += len;
            continue;
        }
        let url = format!("{}/{repo}/resolve/main/{file}", settings.hub_url.trim_end_matches('/'));
        // Hugging Face limite parfois le débit (429) : on réessaie en patientant.
        let mut attempt = 0;
        let mut resp = loop {
            match agent.get(&url).call() {
                Ok(r) => break r,
                Err(ureq::Error::StatusCode(429 | 500 | 502 | 503)) if attempt < settings.download_retries => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_secs(settings.download_backoff_secs * u64::from(attempt)));
                }
                Err(e) => return Err(Box::new(e)).context(HttpSnafu { file })?,
            }
        };
        let expected: Option<u64> =
            resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
        let part = dest.join(format!("{file}.part"));
        let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
        let mut out = storage.writer(&part).context(ModelWriteSnafu { path: &part })?;
        let mut buf = vec![0u8; 1 << 20];
        let mut got = 0u64;
        loop {
            let n = reader.read(&mut buf).context(HttpReadSnafu { file })?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).context(ModelWriteSnafu { path: &part })?;
            got += n as u64;
            if !progress((done + got).min(total), total) {
                drop(out);
                discard(storage, &part);
                return CancelledSnafu.fail()?;
            }
        }
        out.close().context(ModelWriteSnafu { path: &part })?;
        if let Some(expected) = expected
            && got != expected
        {
            discard(storage, &part);
            return IncompleteSnafu { got, expected }.fail()?;
        }
        storage.rename(&part, &path).context(ModelWriteSnafu { path: &path })?;
        done += got;
    }
    let marker = dest.join(COMPLETE);
    storage.write(&marker, Vec::new()).context(ModelWriteSnafu { path: &marker })?;
    progress(total, total);
    Ok(dest)
}

/// Retire un fichier partiel ; un échec ne change rien pour l'utilisateur
/// (le prochain téléchargement le réécrit), il est seulement signalé.
fn discard(storage: &Storage, part: &Path) {
    if let Err(e) = storage.delete(part) {
        tracing::warn!(path = %part.display(), error = %slider_platform::errors::one_line(&e), "fichier partiel");
    }
}
