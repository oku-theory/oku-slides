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
    pub resume: ResumeSettings,
    pub sources: SourcesSettings,
    pub render: RenderSettings,
    pub serve: ServeSettings,
    pub i18n: I18nSettings,
    pub appearance: AppearanceSettings,
    pub stt: SttSettings,
    pub agent: AgentSettings,
    #[serde(default)]
    pub author: AuthorSettings,
}

/// Dossiers de l'app ; un dossier absent se déduit du dossier de données
/// (les decks, du dossier Documents).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[non_exhaustive]
pub struct PathSettings {
    pub data: Option<PathBuf>,
    pub models: Option<PathBuf>,
    pub templates: Option<PathBuf>,
    pub themes: Option<PathBuf>,
    /// Polices supplémentaires (TTF/OTF).
    pub fonts: Option<PathBuf>,
    /// Où s'enregistre de lui-même un deck qui n'a pas encore de fichier.
    pub decks: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct HistorySettings {
    /// Transactions gardées pour annuler.
    pub limit: usize,
}

/// Reprise de l'état au relancement.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ResumeSettings {
    /// Reprises mises de côté (`*.failed-*`) gardées au lancement, par dossier.
    pub keep_failed: usize,
}

/// Sources du deck (replica Loro).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct SourcesSettings {
    /// Taille maximale d'un historique lu dans un PDF ou dans la reprise (Mio).
    pub max_replica_mib: usize,
    /// Taille décompressée maximale du zip des sources lu dans un PDF (Mio) :
    /// historique, assets et polices compris.
    pub max_sources_mib: usize,
}

/// Bornes du processus de rendu.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RenderSettings {
    /// Durée maximale d'une compilation ou d'un export Typst (ms).
    pub timeout_ms: u64,
    /// Durée totale des sondes de widgets d'un appel (ms) ; le premier fichier
    /// exécuté passe toujours, les restants sont sondés à l'appel suivant.
    pub probe_budget_ms: u64,
    /// Mémoire maximale du processus de rendu (Mio).
    pub memory_mib: u64,
    /// Taille maximale d'un message échangé avec le processus de rendu (Mio).
    pub max_frame_mib: u64,
    /// Taille maximale d'un fichier de widget d'un pack (Kio).
    pub max_widget_kib: usize,
    /// Nombre maximal de fichiers de widgets d'une version de pack, `_commun` compris.
    pub max_widgets: usize,
}

impl SourcesSettings {
    /// [`Self::max_replica_mib`] en octets.
    pub fn max_replica_bytes(&self) -> usize {
        self.max_replica_mib.saturating_mul(1024 * 1024)
    }

    /// [`Self::max_sources_mib`] en octets.
    pub fn max_sources_bytes(&self) -> usize {
        self.max_sources_mib.saturating_mul(1024 * 1024)
    }
}

impl RenderSettings {
    /// [`Self::timeout_ms`] en [`std::time::Duration`].
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms)
    }

    /// [`Self::probe_budget_ms`] en [`std::time::Duration`].
    pub fn probe_budget(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.probe_budget_ms)
    }

    /// [`Self::memory_mib`] en octets.
    pub fn memory_bytes(&self) -> u64 {
        self.memory_mib.saturating_mul(1024 * 1024)
    }

    /// [`Self::max_frame_mib`] en octets.
    pub fn max_frame_bytes(&self) -> usize {
        (self.max_frame_mib as usize).saturating_mul(1024 * 1024)
    }

    /// [`Self::max_widget_kib`] en octets.
    pub fn max_widget_bytes(&self) -> usize {
        self.max_widget_kib.saturating_mul(1024)
    }
}

/// Auteur des modifications, inscrit dans l'historique du deck.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[non_exhaustive]
pub struct AuthorSettings {
    #[serde(default)]
    pub name: Option<String>,
}

impl AuthorSettings {
    /// Le nom réglé, sinon l'utilisateur du système ; blanc = aucun.
    pub fn resolved(&self) -> Option<String> {
        let blank = |s: &String| s.trim().is_empty();
        self.name
            .clone()
            .filter(|n| !blank(n))
            .or_else(|| std::env::var("USER").ok().filter(|n| !blank(n)))
            .or_else(|| std::env::var("USERNAME").ok().filter(|n| !blank(n)))
            .map(|n| n.trim().to_owned())
    }
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

/// Apparence de l'interface.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[non_exhaustive]
pub struct AppearanceSettings {
    /// Schéma imposé (`light`, `dark`) ; `None` : celui du système.
    pub scheme: Option<String>,
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
    /// Proposer le moteur de dictée d'Apple (macOS 26+).
    pub apple: bool,
    /// Attente maximale du texte final du moteur d'Apple après l'arrêt (s).
    pub apple_finish_timeout_secs: f32,
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

/// Agent ACP de la dictée vers l'agent (⌘⇧D).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentSettings {
    /// Programme lancé (cherché dans le `PATH`).
    pub command: String,
    pub args: Vec<String>,
}

impl Settings {
    /// Réglages de l'app : défauts, `config.toml` du dossier de données, environnement.
    pub fn load() -> Result<Self, ConfigError> {
        let env = Self::environment();
        let data = std::env::var_os("OKU_SLIDES_PATHS__DATA").map_or_else(default_data_dir, PathBuf::from);
        Self::from_sources(Some(&data.join("config.toml")), env)
    }

    /// Variables `OKU_SLIDES_<SECTION>__<CLÉ>` ; `agent.args` se découpe sur les espaces.
    pub(crate) fn environment() -> config::Environment {
        config::Environment::with_prefix("OKU_SLIDES")
            .prefix_separator("_")
            .separator("__")
            .try_parsing(true)
            .list_separator(" ")
            .with_list_parse_key("agent.args")
    }

    /// `config.toml` du dossier de données (facultatif, écrit par `set_setting`).
    pub fn config_file(&self) -> PathBuf {
        self.paths.data().join("config.toml")
    }

    /// Les seuls défauts embarqués (tests, outils).
    pub fn defaults() -> Result<Self, ConfigError> {
        Self::from_sources(None, Self::environment().source(Some(HashMap::new())))
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

    /// Thèmes de l'utilisateur.
    pub fn themes(&self) -> PathBuf {
        self.themes.clone().unwrap_or_else(|| self.data().join("themes"))
    }

    /// Decks enregistrés d'eux-mêmes : `Documents/Oku Slides` par défaut.
    pub fn decks(&self) -> PathBuf {
        self.decks.clone().unwrap_or_else(|| home_dir().join("Documents").join("Oku Slides"))
    }
}

/// Dossier personnel de l'utilisateur ; vide s'il est inconnu.
fn home_dir() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default()
}

/// Dossier de données de la plateforme pour [`APP_ID`].
fn default_data_dir() -> PathBuf {
    let home = home_dir();
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
