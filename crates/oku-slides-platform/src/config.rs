//! Réglages d'Oku Slides, lus une fois au démarrage.
//!
//! Trois couches, la dernière l'emporte : les défauts embarqués
//! (`config.default.toml`), `config.toml` dans le dossier de données de
//! l'app (facultatif), puis les variables d'environnement
//! `OKU_SLIDES_<SECTION>__<CLÉ>` (double soulignement pour descendre d'un niveau :
//! `OKU_SLIDES_STT__VAD__PAUSE=0.9`).

use std::collections::{BTreeMap, HashMap};
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
    pub import: ImportSettings,
    pub serve: ServeSettings,
    pub i18n: I18nSettings,
    pub appearance: AppearanceSettings,
    pub stt: SttSettings,
    pub models: ModelsSettings,
    pub agent: AgentSettings,
    pub local_agent: LocalAgentSettings,
    pub updates: UpdateSettings,
    #[serde(default)]
    pub author: AuthorSettings,
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
    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.check_interval_hours.get().saturating_mul(3600))
    }

    /// Recherche automatique permise : réglage actif, et jamais en développement.
    pub fn auto_check(&self, debug_build: bool) -> bool {
        self.check_automatically && !debug_build
    }
}

/// Dossiers de l'app ; un dossier absent se déduit du dossier de données
/// (les decks, du dossier Documents).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[non_exhaustive]
pub struct PathSettings {
    pub data: Option<PathBuf>,
    pub models: Option<PathBuf>,
    /// Packs de widgets de l'utilisateur (la bibliothèque).
    pub widgets: Option<PathBuf>,
    /// Packs de layouts de l'utilisateur (la bibliothèque).
    pub layouts: Option<PathBuf>,
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

/// Secondes dans un jour.
const SECS_PER_DAY: i64 = 86_400;

/// Sources du deck (replica Loro).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct SourcesSettings {
    /// Taille maximale d'un historique lu dans un PDF ou dans la reprise (Mio).
    ///
    /// Borne aussi, approximativement, les ops produites par un import.
    pub max_replica_mib: usize,
    /// Taille décompressée maximale du zip des sources lu dans un PDF (Mio).
    ///
    /// Historique, assets et polices compris ; borne aussi, approximativement, la
    /// sortie d'un import (estimation précoce, pas une garantie de réouverture).
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
    /// Hi-fi : écart (ΔE OKLab ×100) d'une région qui fait signaler sa slide.
    ///
    /// Comparé à la pire région non suspecte de chaque slide, contre le rendu
    /// de référence ; fini et positif (la CLI refuse un autre).
    pub score_threshold: f64,
    /// Durée maximale d'une tranche du rendu de référence (ms), dans son processus de rendu.
    pub reference_timeout_ms: u64,
    /// Témoin LibreOffice (`--reference soffice`) : programmes `soffice` essayés
    /// dans l'ordre, un nom seul cherché dans `PATH`.
    pub soffice: Vec<PathBuf>,
    /// Délai de LibreOffice pour une tranche (ms) ; passé lui, il est tué.
    pub soffice_timeout_ms: u64,
    /// Essais par slide de l'ajustement par l'agent.
    ///
    /// Un essai : une note d'une slide dont le raster a changé.
    pub tune_iterations: std::num::NonZeroU32,
    /// Essais de suite sans gain avant d'arrêter une slide de l'ajustement.
    ///
    /// Le gain se mesure sur l'excès total.
    pub tune_stall: std::num::NonZeroU32,
    /// Essais du deck entier (thème, pack) de l'ajustement par l'agent.
    ///
    /// Un par état du deck noté.
    pub tune_global: std::num::NonZeroU32,
    /// Taille maximale d'un `.pptx` importé (Mio).
    ///
    /// Vérifiée avant toute lecture : un fichier plus gros n'entre jamais en mémoire.
    pub max_input_mib: std::num::NonZeroU64,
    /// Témoin de la référence d'un import hi-fi dans l'app (t-905).
    ///
    /// Aussi celui de `oku-slides import` sans `--reference`.
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
    pub fn max_input_bytes(&self) -> u64 {
        self.max_input_mib.get().saturating_mul(1024 * 1024)
    }
}

impl SourcesSettings {
    /// [`Self::compact_after_days`] en secondes.
    pub fn compact_after_secs(&self) -> i64 {
        i64::from(self.compact_after_days).saturating_mul(SECS_PER_DAY)
    }

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

    /// [`Self::max_frame_mib`] en octets, saturé à `usize::MAX` (cibles 32 bits).
    pub fn max_frame_bytes(&self) -> usize {
        usize::try_from(self.max_frame_mib).unwrap_or(usize::MAX).saturating_mul(1024 * 1024)
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
    pub fn header_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.header_timeout_ms.get())
    }
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
    pub tick_ms: u64,
    pub nothing_heard_secs: f32,
    pub preview_step_secs: f32,
    pub min_phrase_secs: f32,
    pub countdown_after_secs: f32,
    /// Rejoue ce fichier WAV au lieu du micro.
    pub fake_mic: Option<PathBuf>,
    /// Proposer le moteur de dictée d'Apple (macOS 26+).
    pub apple: bool,
    /// Attente maximale du texte final du moteur d'Apple après l'arrêt (s).
    pub apple_finish_timeout_secs: f32,
    /// Attente maximale d'une réponse de l'API de dictée (s).
    pub api_timeout_secs: f32,
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

/// Agents ACP proposés et celui qui est lancé par ⌘⇧D.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentSettings {
    /// L'agent actif : une clé de `presets` (vérifiée au chargement).
    pub selected: String,
    /// Les agents proposés, par identifiant (ordre d'affichage : celui des clés).
    pub presets: BTreeMap<String, AgentPreset>,
    /// `OKU_SLIDES_AGENT__SELECTED` impose le choix : le panneau ne le change pas.
    #[serde(skip)]
    pub locked: bool,
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

/// Un agent fourni par Oku Slides lui-même, sans programme externe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    /// L'agent local de repli (`oku-slides-local-agent`), lancé par ce binaire.
    Local,
}

/// Un agent proposé : son nom affiché et comment le lancer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentPreset {
    /// Nom propre affiché tel quel (pas traduit).
    pub name: String,
    /// Programme lancé (cherché dans le `PATH`) ; vide pour un agent intégré.
    #[serde(default)]
    pub command: String,
    /// Arguments du programme.
    #[serde(default)]
    pub args: Vec<String>,
    /// Agent intégré : exclusif de `command`.
    #[serde(default)]
    pub builtin: Option<Builtin>,
}

impl AgentPreset {
    /// Un agent proposé (tests, outils).
    pub fn new(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self { name: name.into(), command: command.into(), args, builtin: None }
    }

    /// Un agent intégré à Oku Slides.
    pub fn builtin(name: impl Into<String>, builtin: Builtin) -> Self {
        Self { name: name.into(), command: String::new(), args: Vec::new(), builtin: Some(builtin) }
    }
}

impl AgentSettings {
    /// L'agent actif. `selected` est vérifié au chargement : l'absence est un invariant violé.
    pub fn active(&self) -> &AgentPreset {
        self.presets.get(&self.selected).unwrap_or_else(|| panic!("agent.selected = {:?} non vérifié", self.selected))
    }

    /// `selected` désigne-t-il un agent de la liste ? Sinon, le message de l'erreur.
    fn check(&self) -> Result<(), String> {
        for (id, preset) in &self.presets {
            if preset.command.is_empty() == preset.builtin.is_none() {
                return Err(format!("[agent.presets.{id}] needs exactly one of command or builtin"));
            }
        }
        if self.presets.contains_key(&self.selected) {
            return Ok(());
        }
        let known: Vec<&str> = self.presets.keys().map(String::as_str).collect();
        Err(format!(
            "agent.selected = {:?} names no [agent.presets] entry (known: {})",
            self.selected,
            known.join(", ")
        ))
    }
}

impl Settings {
    /// Réglages de l'app : défauts, `config.toml` du dossier de données, environnement.
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
        let settings: Self =
            builder.add_source(env).build().and_then(config::Config::try_deserialize).context(ConfigSnafu)?;
        settings.agent.check().map_err(config::ConfigError::Message).context(ConfigSnafu)?;
        Ok(settings)
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

    /// Packs de widgets de l'utilisateur.
    pub fn widgets(&self) -> PathBuf {
        self.widgets.clone().unwrap_or_else(|| self.data().join("widgets"))
    }

    /// Packs de layouts de l'utilisateur.
    pub fn layouts(&self) -> PathBuf {
        self.layouts.clone().unwrap_or_else(|| self.data().join("layouts"))
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
