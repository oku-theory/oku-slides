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

#[path = "config_agent.rs"]
mod agent;

pub use agent::{AgentPreset, AgentSettings, AgentSettingsError, Builtin};

const DEFAULTS: &str = include_str!("config.default.toml");

/// Réglages illisibles (fichier ou variable d'environnement invalide).
#[derive(Debug, Snafu)]
#[snafu(display("invalid settings"))]
pub struct ConfigError {
    #[snafu(source(from(config::ConfigError, Box::new)))]
    source: Box<config::ConfigError>,
    backtrace: Backtrace,
}

/// All the app settings, one section per table of `config.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Settings {
    /// Data, model, pack and deck folders.
    pub paths: PathSettings,
    /// Depth of the undo history.
    pub history: HistorySettings,
    /// State restored on relaunch.
    pub resume: ResumeSettings,
    /// Loro sources embedded in the decks.
    pub sources: SourcesSettings,
    /// Typst rendering and widget pack limits.
    pub render: RenderSettings,
    /// Import of PPTX presentations.
    pub import: ImportSettings,
    /// Pasting images and files into the deck.
    pub paste: PasteSettings,
    /// `oku-slides serve` development server.
    pub serve: ServeSettings,
    /// Language of the messages.
    pub i18n: I18nSettings,
    /// Light or dark scheme of the UI.
    pub appearance: AppearanceSettings,
    /// Dictation pacing and thresholds.
    pub stt: SttSettings,
    /// Model downloads.
    pub models: ModelsSettings,
    /// Offered ACP agents and the active agent.
    pub agent: AgentSettings,
    /// Local fallback agent, without network.
    pub local_agent: LocalAgentSettings,
    /// App update checks.
    pub updates: UpdateSettings,
    /// Author recorded in the deck history.
    #[serde(default)]
    pub author: AuthorSettings,
    /// Agent token usage kept in each deck.
    pub usage: UsageSettings,
}

/// Mises à jour de l'app depuis les releases GitHub.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct UpdateSettings {
    /// Chercher seul au lancement puis à intervalle régulier.
    pub check_automatically: bool,
    /// Heures entre deux recherches automatiques.
    pub check_interval_hours: std::num::NonZeroU64,
}

impl UpdateSettings {
    /// Délai entre deux recherches automatiques.
    #[must_use]
    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.check_interval_hours.get().saturating_mul(3600))
    }

    /// Recherche automatique permise : réglage actif, et jamais en développement.
    #[must_use]
    pub fn auto_check(&self, debug_build: bool) -> bool {
        self.check_automatically && !debug_build
    }
}

/// Dossiers de l'app ; un dossier absent se déduit du dossier de données
/// (les decks, du dossier Documents).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[non_exhaustive]
pub struct PathSettings {
    /// App data folder; absent: the system one.
    pub data: Option<PathBuf>,
    /// Downloaded models (dictation, local agent).
    pub models: Option<PathBuf>,
    /// Packs de widgets de l'utilisateur (la bibliothèque).
    pub widgets: Option<PathBuf>,
    /// Packs de layouts de l'utilisateur (la bibliothèque).
    pub layouts: Option<PathBuf>,
    /// The user's theme packs (the library).
    pub themes: Option<PathBuf>,
    /// Polices supplémentaires (TTF/OTF).
    pub fonts: Option<PathBuf>,
    /// Où s'enregistre de lui-même un deck qui n'a pas encore de fichier.
    pub decks: Option<PathBuf>,
}

/// Undo history of the decks.
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

/// Secondes dans un jour.
const SECS_PER_DAY: i64 = 86_400;

/// Sources du deck (replica Loro).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct SourcesSettings {
    /// Maximum size of a read history, in MiB.
    ///
    /// It applies to a history read from a PDF or from the resume state.
    ///
    /// Also roughly bounds the ops produced by an import.
    pub max_replica_mib: usize,
    /// Maximum decompressed size of the sources zip read from a PDF (MiB).
    ///
    /// History, assets and fonts included; also roughly bounds the
    /// output of an import (an early estimate, not a guarantee of reopening).
    pub max_sources_mib: usize,
    /// Délai de compaction des nœuds supprimés (jours) ; 0 : jamais.
    pub compact_after_days: u32,
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

/// Import d'une présentation (`oku-slides import`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ImportSettings {
    /// Hi-fi: difference (OKLab ΔE ×100) of a region that gets its slide flagged.
    ///
    /// Compared with the worst non-suspect region of each slide, against the reference
    /// render; finite and positive (the CLI rejects anything else).
    pub score_threshold: f64,
    /// Durée maximale d'une tranche du rendu de référence (ms), dans son processus de rendu.
    pub reference_timeout_ms: u64,
    /// Témoin LibreOffice (`--reference soffice`) : programmes `soffice` essayés
    /// dans l'ordre, un nom seul cherché dans `PATH`.
    pub soffice: Vec<PathBuf>,
    /// Délai de LibreOffice pour une tranche (ms) ; passé lui, il est tué.
    pub soffice_timeout_ms: u64,
    /// Attempts per slide of the agent's adjustment.
    ///
    /// One attempt: a score of a slide whose raster changed.
    pub tune_iterations: std::num::NonZeroU32,
    /// Consecutive attempts without gain before a slide leaves the adjustment.
    ///
    /// The gain is measured on the total excess.
    pub tune_stall: std::num::NonZeroU32,
    /// Attempts on the whole deck (theme, pack) of the agent's adjustment.
    ///
    /// One per scored deck state.
    pub tune_global: std::num::NonZeroU32,
    /// Maximum size of an imported `.pptx` (MiB).
    ///
    /// Checked before any read: a larger file never enters memory.
    pub max_input_mib: std::num::NonZeroU64,
    /// Witness for the reference of a hi-fi import in the app (t-905).
    ///
    /// Also the one for `oku-slides import` without `--reference`.
    pub witness: ImportWitness,
}

/// Le témoin qui rend la référence d'un import hi-fi (`[import] witness`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportWitness {
    /// office2pdf, dans le processus de rendu (défaut).
    Office2pdf,
    /// LibreOffice, le premier programme de `[import] soffice` trouvé.
    Soffice,
}

impl ImportSettings {
    /// [`Self::max_input_mib`] en octets.
    #[must_use]
    pub fn max_input_bytes(&self) -> u64 {
        self.max_input_mib.get().saturating_mul(1024 * 1024)
    }
}

impl SourcesSettings {
    /// [`Self::compact_after_days`] en secondes.
    #[must_use]
    pub fn compact_after_secs(&self) -> i64 {
        i64::from(self.compact_after_days).saturating_mul(SECS_PER_DAY)
    }

    /// [`Self::max_replica_mib`] en octets.
    #[must_use]
    pub fn max_replica_bytes(&self) -> usize {
        self.max_replica_mib.saturating_mul(1024 * 1024)
    }

    /// [`Self::max_sources_mib`] en octets.
    #[must_use]
    pub fn max_sources_bytes(&self) -> usize {
        self.max_sources_mib.saturating_mul(1024 * 1024)
    }
}

/// Bornes des images collées dans le brouillon de l'agent (`[paste]`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct PasteSettings {
    /// Taille maximale d'une image (Mio).
    pub max_image_mib: usize,
    /// Pixels au plus d'une image, lus dans son en-tête.
    pub max_image_pixels: u64,
    /// Images au plus dans un brouillon.
    pub max_draft_images: usize,
    /// Taille cumulée des images gardées par onglet (Mio).
    pub max_kept_mib: usize,
    /// Plus grande image envoyée à l'agent en bloc image (Mio).
    pub prompt_image_mib: usize,
    /// Plus grand côté (pixels) d'une image envoyée à l'agent en bloc image.
    pub prompt_image_max_side: u32,
    /// Taille encodée cumulée des images d'une même consigne envoyées à l'agent (Mio).
    pub prompt_total_mib: usize,
}

/// Les bornes des blocs image envoyés à l'agent avec une consigne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptImageLimits {
    /// Taille encodée (base64) maximale d'une image, en octets.
    pub max_bytes: usize,
    /// Plus grand côté d'une image, en pixels.
    pub max_side: u32,
    /// Taille encodée cumulée des images d'une consigne, en octets.
    pub total_bytes: usize,
}

impl PasteSettings {
    /// [`Self::max_image_mib`] en octets.
    #[must_use]
    pub fn max_image_bytes(&self) -> usize {
        self.max_image_mib.saturating_mul(1024 * 1024)
    }

    /// [`Self::max_kept_mib`] en octets.
    #[must_use]
    pub fn max_kept_bytes(&self) -> usize {
        self.max_kept_mib.saturating_mul(1024 * 1024)
    }

    /// Les bornes des images montrées à l'agent, en octets et en pixels.
    #[must_use]
    pub fn prompt_limits(&self) -> PromptImageLimits {
        PromptImageLimits {
            max_bytes: self.prompt_image_mib.saturating_mul(1024 * 1024),
            max_side: self.prompt_image_max_side,
            total_bytes: self.prompt_total_mib.saturating_mul(1024 * 1024),
        }
    }
}

impl RenderSettings {
    /// [`Self::timeout_ms`] en [`std::time::Duration`].
    #[must_use]
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms)
    }

    /// [`Self::probe_budget_ms`] en [`std::time::Duration`].
    #[must_use]
    pub fn probe_budget(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.probe_budget_ms)
    }

    /// [`Self::memory_mib`] en octets.
    #[must_use]
    pub fn memory_bytes(&self) -> u64 {
        self.memory_mib.saturating_mul(1024 * 1024)
    }

    /// [`Self::max_frame_mib`] en octets, saturé à `usize::MAX` (cibles 32 bits).
    #[must_use]
    pub fn max_frame_bytes(&self) -> usize {
        usize::try_from(self.max_frame_mib).unwrap_or(usize::MAX).saturating_mul(1024 * 1024)
    }

    /// [`Self::max_widget_kib`] en octets.
    #[must_use]
    pub fn max_widget_bytes(&self) -> usize {
        self.max_widget_kib.saturating_mul(1024)
    }
}

/// Auteur des modifications, inscrit dans l'historique du deck.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[non_exhaustive]
pub struct AuthorSettings {
    /// Recorded name; absent or blank: the system user.
    #[serde(default)]
    pub name: Option<String>,
}

impl AuthorSettings {
    /// Le nom réglé, sinon l'utilisateur du système ; blanc = aucun.
    #[must_use]
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

/// Agent token usage, recorded per turn in each deck file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct UsageSettings {
    /// Records each turn's tokens, model and cost in its deck.
    ///
    /// Off: none recorded nor saved.
    pub store_in_deck: bool,
}

/// `oku-slides serve` development server, without authentication.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ServeSettings {
    /// Listen address; outside loopback, the whole network is served.
    pub host: String,
    /// Listen port.
    pub port: u16,
    /// Délai pour recevoir les en-têtes d'une requête (ms) ; au-delà, la
    /// connexion est fermée. Zéro est refusé au chargement (aucune requête
    /// ne passerait).
    pub header_timeout_ms: std::num::NonZeroU64,
    /// Connexions simultanées au plus ; au-delà, une nouvelle connexion est
    /// fermée aussitôt. Zéro est refusé au chargement.
    pub max_connections: std::num::NonZeroUsize,
}

impl ServeSettings {
    /// [`Self::header_timeout_ms`] en [`std::time::Duration`].
    #[must_use]
    pub fn header_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.header_timeout_ms.get())
    }
}

/// Language of the app messages.
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

/// Téléchargement des modèles (dictée, agent local).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ModelsSettings {
    /// Dépôt des modèles, sans barre finale.
    pub hub_url: String,
    /// Tentatives de plus quand le dépôt est occupé (429, 5xx).
    pub download_retries: u32,
    /// Attente avant la première nouvelle tentative (s), multipliée à chaque suivante.
    pub download_backoff_secs: u64,
    /// Attente maximale (s) d'une réponse ou d'octets du dépôt avant d'abandonner.
    pub read_timeout_secs: u64,
}

/// Rythmes et seuils de la dictée.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct SttSettings {
    /// Pace of the listening loop (ms).
    pub tick_ms: u64,
    /// Without any speech for this long (s), listening stops.
    pub nothing_heard_secs: f32,
    /// New preview when the sentence has grown by at least this much (s).
    pub preview_step_secs: f32,
    /// Shorter than this (s), a "sentence" is ignored noise.
    pub min_phrase_secs: f32,
    /// Silence (s) after which the stop countdown is shown.
    pub countdown_after_secs: f32,
    /// Rejoue ce fichier WAV au lieu du micro.
    pub fake_mic: Option<PathBuf>,
    /// Proposer le moteur de dictée d'Apple (macOS 26+).
    pub apple: bool,
    /// Attente maximale du texte final du moteur d'Apple après l'arrêt (s).
    pub apple_finish_timeout_secs: f32,
    /// Attente maximale d'une réponse de l'API de dictée (s).
    pub api_timeout_secs: f32,
    /// Splitting of the voice into sentences.
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
    /// Audio kept before speech starts (s).
    pub pre_roll: f32,
    /// Audio kept after speech ends (s).
    pub post_roll: f32,
    /// Longueur maximale d'une phrase (s).
    pub max_phrase: f32,
}

/// Le moteur de l'agent local de repli (`[local_agent] engine`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalEngine {
    /// Le meilleur moteur disponible sur cette machine.
    #[default]
    Auto,
    /// llama.cpp sur Apple silicon, avec le modèle Qwen3 téléchargé.
    Gguf,
    /// Rejoue un script (tests, builds de débogage).
    #[cfg(debug_assertions)]
    Fake,
}

/// Réglages de l'agent local de repli.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct LocalAgentSettings {
    /// Moteur qui répond (llama.cpp ou script rejoué).
    pub engine: LocalEngine,
    /// Contexte du modèle (jetons).
    pub context: u32,
    /// Température d'échantillonnage des actions (0 : glouton).
    pub temperature: f32,
    /// Longueur maximale d'une réponse (jetons).
    pub max_reply_tokens: u32,
    /// Script rejoué par le moteur `fake`.
    pub fake_script: Option<PathBuf>,
    /// Journal des appels du moteur `fake`.
    pub fake_log: Option<PathBuf>,
}

impl Settings {
    /// App settings: defaults, the data folder's `config.toml`, environment.
    ///
    /// # Errors
    ///
    /// If `config.toml` or an environment variable is invalid, or if the agents are badly declared.
    pub fn load() -> Result<Self, ConfigError> {
        let env = Self::environment();
        let data = std::env::var_os("OKU_SLIDES_PATHS__DATA").map_or_else(default_data_dir, PathBuf::from);
        let mut settings = Self::from_sources(Some(&data.join("config.toml")), env)?;
        settings.agent.locked = std::env::var_os("OKU_SLIDES_AGENT__SELECTED").is_some();
        Ok(settings)
    }

    /// Variables `OKU_SLIDES_<SECTION>__<CLÉ>`.
    pub(crate) fn environment() -> config::Environment {
        config::Environment::with_prefix("OKU_SLIDES").prefix_separator("_").separator("__").try_parsing(true)
    }

    /// `config.toml` du dossier de données (facultatif, écrit par `set_setting`).
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.paths.data().join("config.toml")
    }

    /// The embedded defaults alone (tests, tools).
    ///
    /// # Errors
    ///
    /// If the embedded defaults cannot be read, which would be a bug.
    pub fn defaults() -> Result<Self, ConfigError> {
        Self::from_sources(None, Self::environment().source(Some(HashMap::new())))
    }

    /// Defaults, then `file` if it exists, then `env`.
    ///
    /// # Errors
    ///
    /// If `file` or `env` is invalid, or if the agents are badly declared.
    pub fn from_sources(file: Option<&Path>, env: config::Environment) -> Result<Self, ConfigError> {
        let mut builder =
            config::Config::builder().add_source(config::File::from_str(DEFAULTS, config::FileFormat::Toml));
        if let Some(path) = file {
            builder = builder.add_source(config::File::from(path).format(config::FileFormat::Toml).required(false));
        }
        let config = builder.add_source(env).build().context(ConfigSnafu)?;
        // `[agent]` is checked here first: inside `try_deserialize`, config-rs would append
        // " for key `agent`" to the message. A raw shape error is left to `try_deserialize`.
        if let Ok(raw) = config.get::<agent::RawAgentSettings>("agent") {
            raw.check().map_err(|e| config::ConfigError::Message(e.to_string())).context(ConfigSnafu)?;
        }
        config.try_deserialize().context(ConfigSnafu)
    }
}

impl PathSettings {
    /// Dossier de données de l'app.
    pub fn data(&self) -> PathBuf {
        self.data.clone().unwrap_or_else(default_data_dir)
    }

    /// Modèles de dictée : un modèle n'est téléchargé qu'une fois pour l'app
    /// et le serveur de développement.
    #[must_use]
    pub fn models(&self) -> PathBuf {
        self.models.clone().unwrap_or_else(|| self.data().join("models"))
    }

    /// Packs de widgets de l'utilisateur.
    #[must_use]
    pub fn widgets(&self) -> PathBuf {
        self.widgets.clone().unwrap_or_else(|| self.data().join("widgets"))
    }

    /// Packs de layouts de l'utilisateur.
    #[must_use]
    pub fn layouts(&self) -> PathBuf {
        self.layouts.clone().unwrap_or_else(|| self.data().join("layouts"))
    }

    /// Thèmes de l'utilisateur.
    #[must_use]
    pub fn themes(&self) -> PathBuf {
        self.themes.clone().unwrap_or_else(|| self.data().join("themes"))
    }

    /// Decks enregistrés d'eux-mêmes : `Documents/Oku Slides` par défaut.
    #[must_use]
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
        std::env::var_os("APPDATA").map_or_else(|| home.join("AppData/Roaming"), PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME").map_or_else(|| home.join(".local/share"), PathBuf::from)
    };
    base.join(APP_ID)
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
