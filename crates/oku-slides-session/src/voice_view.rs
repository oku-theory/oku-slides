//! Ce que montre la boîte des réglages de la dictée et de l'agent.
//!
//! Aucun texte traduit : la boîte le traduit à chaque construction.
//! Les modèles et les agents sont des chaînes exprès : la session ne dépend pas de `oku-slides-stt`.

use serde_json::{Map, Value};

/// Les langues proposées à la dictée : code et nom écrit dans sa langue.
pub const DICTATION_LANGUAGES: [(&str, &str); 7] = [
    ("fr", "Français"),
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("nl", "Nederlands"),
];

/// D'où vient un modèle de dictée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// La reconnaissance du système.
    Apple,
    /// Un modèle Whisper sur cet ordinateur.
    Local,
    /// Un service distant compatible OpenAI.
    Api,
}

/// Un modèle de dictée ; son id est celui de `oku-slides-stt` (`Model::id`).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelView {
    /// L'id du modèle : `apple`, `turbo`, `large`, `base` ou `api`.
    pub id: String,
    /// D'où il vient.
    pub kind: ModelKind,
    /// Sa taille à télécharger, en octets ; zéro sans fichier.
    pub size: u64,
    /// Ses fichiers sont sur l'ordinateur.
    pub installed: bool,
    /// Il comprend la langue choisie.
    pub available: bool,
}

/// Settings of the remote transcription service.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiView {
    /// The service address, normalized.
    pub url: String,
    /// The model asked of the service; the deployment for Azure.
    pub model: String,
    /// Name of the environment variable that holds the key.
    pub key_env: String,
    /// Azure OpenAI `api-version`; empty: the OpenAI API.
    pub api_version: String,
    /// Custom headers in one line: `Name: value; Name: $VARIABLE` (variable names only).
    pub headers: String,
    /// Every variable is set; their values are never read here.
    pub key_found: bool,
    /// The first variable not set, named in the hint.
    pub missing: Option<String>,
}

/// Le téléchargement d'un modèle de dictée en cours.
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadView {
    /// L'id du modèle téléchargé.
    pub model: String,
    /// Les octets reçus.
    pub received: u64,
    /// Les octets attendus.
    pub total: u64,
}

/// La dictée telle que la montre la boîte.
#[derive(Debug, Clone, PartialEq)]
pub struct DictationView {
    /// L'id du modèle choisi.
    pub model: String,
    /// Les modèles connus, dans l'ordre d'affichage.
    pub models: Vec<ModelView>,
    /// Le code de la langue fixée ; `None` pour la détection.
    pub language: Option<String>,
    /// Le code de la langue détectée par le système, s'il y en a une.
    pub detected: Option<String>,
    /// La dictée s'arrête seule après un silence.
    pub auto_stop: bool,
    /// La durée du silence, en secondes.
    pub silence: f32,
    /// Les réglages du service distant.
    pub api: ApiView,
    /// Le téléchargement en cours, s'il y en a un.
    pub download: Option<DownloadView>,
    /// Aucune dictée n'écoute ni ne transcrit.
    pub idle: bool,
}

/// Pourquoi le téléchargement du modèle de l'agent local a échoué.
#[derive(Debug, Clone, PartialEq)]
pub enum LocalFailure {
    /// Le téléchargement a échoué.
    Failed,
    /// Le fichier reçu était endommagé.
    Integrity,
    /// Le fichier reçu est trop court.
    Incomplete {
        /// Les octets reçus.
        got: u64,
        /// Les octets attendus.
        expected: u64,
    },
    /// L'utilisateur l'a annulé.
    Cancelled,
}

/// L'état du modèle de l'agent local.
#[derive(Debug, Clone, PartialEq)]
pub enum LocalModelView {
    /// Le modèle manque : il faut le télécharger.
    NeedsDownload {
        /// Sa taille, en octets.
        size: u64,
    },
    /// Le téléchargement tourne.
    Running {
        /// L'avancement, de 0 à 100.
        percent: u8,
    },
    /// Le dernier téléchargement a échoué.
    Failed {
        /// La taille du modèle, en octets.
        size: u64,
        /// La cause.
        failure: LocalFailure,
    },
}

/// Un agent proposé au choix.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentPresetView {
    /// Son id.
    pub id: String,
    /// Son nom.
    pub name: String,
    /// Sa commande, pour un agent externe.
    pub hint: Option<String>,
}

/// A model the active agent offers.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentModelView {
    /// Its value id, sent back to the agent.
    pub id: String,
    /// Its name, shown as the agent gives it.
    pub name: String,
    /// Its description from the agent, if any.
    pub hint: Option<String>,
}

/// The active agent's models, as the box shows them.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentModelsView {
    /// No session of this agent has told its models yet.
    Unknown {
        /// The model the app asks for, if any.
        chosen: Option<String>,
    },
    /// The agent offers these models.
    Offered {
        /// The offered models, in the agent's order.
        models: Vec<AgentModelView>,
        /// The model of the agent's last session.
        current: String,
        /// The model the app asks for, if any; it may not be offered.
        chosen: Option<String>,
    },
}

/// Les agents tels que les montre la boîte.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentsView {
    /// Les agents proposés.
    pub presets: Vec<AgentPresetView>,
    /// L'id de l'agent actif.
    pub active: String,
    /// Une réponse tourne : changer d'agent l'interromprait.
    pub busy: bool,
    /// L'environnement impose l'agent : pas de choix.
    pub locked: bool,
    /// Le modèle de l'agent local, quand il compte.
    pub local: Option<LocalModelView>,
    /// L'id de l'agent choisi, en attente de la fin des réponses.
    pub pending: Option<String>,
    /// The active agent's models; `None` when it offers no model choice.
    pub models: Option<AgentModelsView>,
}

/// Tout ce que montre la boîte.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoiceView {
    /// La dictée ; `None` sans moteur de dictée.
    pub dictation: Option<DictationView>,
    /// Les agents.
    pub agents: AgentsView,
}

/// La langue de la dictée demandée par un changement.
#[derive(Debug, Clone, PartialEq)]
pub enum DictationLanguage {
    /// Détection automatique.
    Auto,
    /// Un code de langue.
    Code(String),
}

/// Des réglages de dictée à changer ; ce qui reste `None` ne bouge pas.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DictationPatch {
    /// L'id du modèle.
    pub model: Option<String>,
    /// La langue.
    pub language: Option<DictationLanguage>,
    /// L'arrêt après un silence.
    pub auto_stop: Option<bool>,
    /// La durée du silence, en secondes.
    pub silence: Option<f32>,
    /// L'adresse du service distant.
    pub api_url: Option<String>,
    /// Le modèle du service distant.
    pub api_model: Option<String>,
    /// La variable d'environnement qui porte la clé.
    pub api_key_env: Option<String>,
    /// Azure OpenAI `api-version`; empty: the OpenAI API.
    pub api_version: Option<String>,
    /// Custom headers, in their one-line form.
    pub api_headers: Option<String>,
}

impl DictationPatch {
    /// Aucun réglage n'est demandé.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The change touches the address, version, a variable or the headers: where secrets go.
    #[must_use]
    pub fn touches_api_target(&self) -> bool {
        self.api_url.is_some() || self.api_key_env.is_some() || self.api_version.is_some() || self.api_headers.is_some()
    }
}

/// Une action de l'utilisateur dans la boîte.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceEvent {
    /// Changer des réglages de dictée.
    Dictation(DictationPatch),
    /// Télécharger un modèle de dictée.
    DownloadDictation {
        /// L'id du modèle.
        model: String,
    },
    /// Annuler le téléchargement d'un modèle de dictée.
    CancelDictationDownload,
    /// Choisir l'agent de l'app.
    SelectAgent {
        /// L'id de l'agent.
        id: String,
    },
    /// Télécharger le modèle de l'agent local.
    DownloadAgentModel,
    /// Annuler le téléchargement du modèle de l'agent local.
    CancelAgentModelDownload,
    /// Choose the model the active agent's sessions use.
    SelectAgentModel {
        /// The model's value id, offered by the agent.
        id: String,
    },
}

/// L'évènement d'une action de la boîte et de son contexte ; `None` si mal formé.
#[must_use]
pub fn voice_event(name: &str, context: &Map<String, Value>) -> Option<VoiceEvent> {
    let text = |key: &str| context.get(key).and_then(Value::as_str).map(str::to_owned);
    let patch = |p: DictationPatch| Some(VoiceEvent::Dictation(p));
    match name {
        "model" => patch(DictationPatch { model: Some(text("id")?), ..Default::default() }),
        "language" => {
            let code = text("code")?;
            let language = if code.is_empty() { DictationLanguage::Auto } else { DictationLanguage::Code(code) };
            patch(DictationPatch { language: Some(language), ..Default::default() })
        }
        "silence" => match text("value")?.as_str() {
            "off" => patch(DictationPatch { auto_stop: Some(false), ..Default::default() }),
            v => {
                let secs = v.parse::<f32>().ok().filter(|s| s.is_finite() && *s > 0.0)?;
                patch(DictationPatch { auto_stop: Some(true), silence: Some(secs), ..Default::default() })
            }
        },
        "api" => {
            let value = text("value")?;
            let mut p = DictationPatch::default();
            match text("field")?.as_str() {
                "url" => p.api_url = Some(value),
                "model" => p.api_model = Some(value),
                "key_env" => p.api_key_env = Some(value),
                "api_version" => p.api_version = Some(value),
                "headers" => p.api_headers = Some(value),
                _ => return None,
            }
            patch(p)
        }
        "download" => Some(VoiceEvent::DownloadDictation { model: text("model")? }),
        "cancel_download" => Some(VoiceEvent::CancelDictationDownload),
        "select" => Some(VoiceEvent::SelectAgent { id: text("id")? }),
        "download_agent_model" => Some(VoiceEvent::DownloadAgentModel),
        "cancel_agent_model_download" => Some(VoiceEvent::CancelAgentModelDownload),
        "agent_model" => Some(VoiceEvent::SelectAgentModel { id: text("id").filter(|id| !id.is_empty())? }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "voice_view_tests.rs"]
mod voice_view_tests;
