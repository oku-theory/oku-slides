//! Réglages d'Oku Slides, lus une fois au démarrage.
//!
//! Trois couches, la dernière l'emporte : les défauts embarqués
//! (`config.default.toml`), `config.toml` dans le dossier de données de
//! l'app (facultatif), puis les variables d'environnement
//! `OKU_SLIDES_<SECTION>__<CLÉ>` (double soulignement pour descendre d'un niveau :
//! `OKU_SLIDES_STT__VAD__PAUSE=0.9`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use snafu::{Backtrace, ResultExt, Snafu};

/// Identifiant de l'app : même dossier que `app_data_dir` de Tauri.
pub const APP_ID: &str = "me.dartiguenave.oku-slides";

const DEFAULTS: &str = include_str!("config.default.toml");

/// Réglages illisibles (fichier ou variable d'environnement invalide).
#[derive(Debug, Snafu)]
#[snafu(display("invalid settings"))]
pub struct ConfigError {
    #[snafu(source(from(config::ConfigError, Box::new)))]
    source: Box<config::ConfigError>,
    backtrace: Backtrace,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Settings {
    pub paths: PathSettings,
    pub history: HistorySettings,
    pub serve: ServeSettings,
    pub i18n: I18nSettings,
    pub stt: SttSettings,
}

/// Dossiers de l'app ; un dossier absent se déduit du dossier de données.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[non_exhaustive]
pub struct PathSettings {
    pub data: Option<PathBuf>,
    pub models: Option<PathBuf>,
    pub templates: Option<PathBuf>,
    /// Polices supplémentaires (TTF/OTF).
    pub fonts: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct HistorySettings {
    /// Transactions gardées pour annuler.
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ServeSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[non_exhaustive]
pub struct I18nSettings {
    /// Langue imposée (`fr`, `en`…) ; `None` : celle du système.
    pub locale: Option<String>,
}

/// Rythmes et seuils de la dictée.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct SttSettings {
    pub tick_ms: u64,
    pub nothing_heard_secs: f32,
    pub preview_step_secs: f32,
    pub min_phrase_secs: f32,
    pub countdown_after_secs: f32,
    pub hub_url: String,
    pub download_retries: u32,
    pub download_backoff_secs: u64,
    /// Rejoue ce fichier WAV au lieu du micro.
    pub fake_mic: Option<PathBuf>,
    pub vad: VadSettings,
}

/// Découpage de la voix en phrases.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct VadSettings {
    /// Score à partir duquel une trame est parlée.
    pub threshold: f32,
    /// Trames parlées d'affilée pour ouvrir une phrase.
    pub onset_frames: usize,
    /// Pause qui clôt une phrase (s)…
    pub pause: f32,
    /// …si elle dure déjà au moins ceci (s) ; plus courte, elle attend la suite,
    pub min_phrase: f32,
    /// sauf après une pause plus longue que celle-ci (s).
    pub long_pause: f32,
    /// Audio gardé avant le début et après la fin de la parole (s).
    pub pre_roll: f32,
    pub post_roll: f32,
    /// Longueur maximale d'une phrase (s).
    pub max_phrase: f32,
}

impl Settings {
    /// Réglages de l'app : défauts, `config.toml` du dossier de données, environnement.
    pub fn load() -> Result<Self, ConfigError> {
        let env = config::Environment::with_prefix("OKU_SLIDES").prefix_separator("_").separator("__").try_parsing(true);
        let data = std::env::var_os("OKU_SLIDES_PATHS__DATA").map_or_else(default_data_dir, PathBuf::from);
        Self::from_sources(Some(&data.join("config.toml")), env)
    }

    /// Les seuls défauts embarqués (tests, outils).
    pub fn defaults() -> Result<Self, ConfigError> {
        Self::from_sources(None, config::Environment::default().source(Some(HashMap::new())))
    }

    /// Défauts, puis `file` s'il existe, puis `env`.
    pub fn from_sources(file: Option<&Path>, env: config::Environment) -> Result<Self, ConfigError> {
        let mut builder =
            config::Config::builder().add_source(config::File::from_str(DEFAULTS, config::FileFormat::Toml));
        if let Some(path) = file {
            builder = builder.add_source(config::File::from(path).format(config::FileFormat::Toml).required(false));
        }
        builder.add_source(env).build().and_then(config::Config::try_deserialize).context(ConfigSnafu)
    }
}

impl PathSettings {
    /// Dossier de données de l'app.
    pub fn data(&self) -> PathBuf {
        self.data.clone().unwrap_or_else(default_data_dir)
    }

    /// Modèles de dictée : un modèle n'est téléchargé qu'une fois pour l'app
    /// et le serveur de développement.
    pub fn models(&self) -> PathBuf {
        self.models.clone().unwrap_or_else(|| self.data().join("models"))
    }

    /// Ensembles de modèles de slides de l'utilisateur.
    pub fn templates(&self) -> PathBuf {
        self.templates.clone().unwrap_or_else(|| self.data().join("templates"))
    }
}

/// Dossier de données de la plateforme pour [`APP_ID`].
fn default_data_dir() -> PathBuf {
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

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
