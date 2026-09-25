//! Dossiers de l'app et fichiers : où vivent les données, lire un document,
//! écrire sans risque. L'app et le serveur de développement partagent tout.

use std::path::{Path, PathBuf};

use slider_core::SourceBundle;

use crate::{Result, SessionError};

/// Identifiant de l'app : même dossier que `app_data_dir` de Tauri.
pub const APP_ID: &str = "me.dartiguenave.echo";

/// Extension d'un document Écho.
pub const EXTENSION: &str = ".echo.pdf";

/// Dossier de données de l'app (`ECHO_DATA` le remplace).
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ECHO_DATA") {
        return dir.into();
    }
    let home =
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData/Roaming"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))
    };
    base.join(APP_ID)
}

/// Ensembles de modèles de slides de l'utilisateur.
pub fn templates_dir() -> PathBuf {
    data_dir().join("templates")
}

/// Modèles de dictée (`ECHO_MODELS` le remplace) : un modèle n'est
/// téléchargé qu'une fois pour l'app et le serveur de développement.
pub fn models_dir() -> PathBuf {
    std::env::var_os("ECHO_MODELS").map_or_else(|| data_dir().join("models"), PathBuf::from)
}

/// Écrit dans un fichier temporaire à côté, puis renomme.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let tmp = dir.join(format!(".{}.tmp", path.file_name().unwrap_or_default().to_string_lossy()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Un document lu sur disque.
#[derive(Debug)]
pub struct Opened {
    pub sources: SourceBundle,
    /// Lu dans un `.echo.pdf` (sinon, un deck JSON de développement).
    pub pdf: bool,
    /// Le PDF a été modifié par un autre outil après Écho.
    pub modified_elsewhere: bool,
}

/// Lit un `.echo.pdf` (reconnu à son contenu) ou un deck JSON.
pub fn read_document(path: &Path) -> Result<Opened> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"%PDF") {
        let found = slider_pdf::extract_sources(&bytes)?.ok_or(SessionError::NotSlider)?;
        let sources = SourceBundle::from_zip(&found.zip)?;
        return Ok(Opened { sources, pdf: true, modified_elsewhere: found.modified_elsewhere() });
    }
    let deck =
        serde_json::from_slice(&bytes).map_err(|e| SessionError::BadRequest(format!("deck JSON invalide : {e}")))?;
    Ok(Opened { sources: SourceBundle::new(deck), pdf: false, modified_elsewhere: false })
}
