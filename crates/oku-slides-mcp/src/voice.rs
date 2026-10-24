//! La dictée et l'agent de l'app, tels que la boîte et les outils les voient.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::{AgentPresetView, ModelView, VoiceEvent, VoiceView};
use snafu::{Backtrace, Snafu};

/// Qui demande un changement de la dictée ou de l'agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Un clic dans la boîte : appliqué tout de suite.
    User,
    /// Un outil de l'agent de cet onglet.
    Agent {
        /// L'onglet de l'agent qui demande.
        tab: String,
        /// L'utilisateur a confirmé ce que `check` demandait de confirmer.
        confirmed: bool,
    },
}

/// Quand un changement accepté prend effet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceApplied {
    /// Tout de suite.
    Now,
    /// Dès qu'aucune réponse ne tourne (changement d'agent).
    WhenIdle,
    /// Rien à changer.
    Unchanged,
}

/// L'API de dictée configurée par l'utilisateur, montrée avant de l'utiliser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiTarget {
    /// Hôte de l'adresse, montré seul.
    pub host: String,
    /// Adresse normalisée.
    pub url: String,
    /// Nom de la variable de la clé.
    pub key_env: String,
    /// Modèle de l'API qui sera utilisé.
    pub model: String,
}

/// Ce qu'une demande vérifiée changerait, pour la confirmer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Checked {
    /// La demande ne changerait rien.
    pub unchanged: bool,
    /// La dictée passerait par l'API : où elle irait.
    pub api: Option<ApiTarget>,
    /// Un modèle à télécharger, avec le modèle de l'API pour son nom.
    pub download: Option<(ModelView, String)>,
    /// L'agent choisi.
    pub agent: Option<AgentPresetView>,
    /// Taille du modèle de l'agent local à télécharger.
    pub agent_model_size: Option<u64>,
}

impl Checked {
    /// La demande attend une confirmation de l'utilisateur.
    pub fn needs_confirmation(&self) -> bool {
        self.api.is_some() || self.download.is_some() || self.agent.is_some() || self.agent_model_size.is_some()
    }
}

/// Pourquoi un changement de la dictée ou de l'agent est refusé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceErrorKind {
    /// L'hôte n'a pas de dictée.
    NoDictation,
    /// La dictée n'est pas au repos.
    Busy,
    /// Modèle de dictée inconnu.
    UnknownModel,
    /// Le modèle ne gère pas la langue choisie.
    Unavailable,
    /// Le modèle est déjà installé.
    Installed,
    /// Le modèle ne se télécharge pas.
    NotDownloadable,
    /// Rien à annuler.
    NothingToCancel,
    /// Silence hors de la plage acceptée.
    BadSilence,
    /// Langue de dictée inconnue.
    BadLanguage,
    /// Adresse d'API refusée.
    BadUrl,
    /// Nom de variable refusé.
    BadKeyEnv,
    /// Modèle d'API refusé.
    BadApiModel,
    /// Adresse et variable de l'API : réservées à l'utilisateur.
    ApiReservedToUser,
    /// L'état a changé depuis la question : il faudrait confirmer à nouveau.
    Changed,
    /// Rien à télécharger pour l'agent choisi.
    NothingToDownload,
    /// Agent absent de la configuration.
    UnknownAgent,
    /// Le choix de l'agent est imposé.
    AgentLocked,
    /// Le choix de l'agent n'a pas pu être enregistré.
    AgentNotSaved,
}

impl VoiceErrorKind {
    /// La clé Fluent du message montré à l'utilisateur.
    fn key(self) -> &'static str {
        match self {
            Self::NoDictation => "voice-error-no-dictation",
            Self::Busy => "voice-error-busy",
            Self::UnknownModel => "voice-error-unknown-model",
            Self::Unavailable => "voice-error-unavailable",
            Self::Installed => "voice-error-installed",
            Self::NotDownloadable => "voice-error-not-downloadable",
            Self::NothingToCancel => "voice-error-nothing-to-cancel",
            Self::BadSilence => "voice-error-bad-silence",
            Self::BadLanguage => "voice-error-bad-language",
            Self::BadUrl => "voice-error-bad-url",
            Self::BadKeyEnv => "voice-error-bad-key-env",
            Self::BadApiModel => "voice-error-bad-api-model",
            Self::ApiReservedToUser => "voice-error-api-reserved",
            Self::Changed => "voice-error-changed",
            Self::NothingToDownload => "voice-error-nothing-to-download",
            Self::UnknownAgent => "agent-error-unknown-agent",
            Self::AgentLocked => "agent-error-agent-locked",
            Self::AgentNotSaved => "agent-error-agent-not-saved",
        }
    }
}

/// Un changement de la dictée ou de l'agent, refusé.
#[derive(Debug, Snafu)]
#[snafu(display("voice change refused: {kind:?}"))]
pub struct VoiceError {
    kind: VoiceErrorKind,
    backtrace: Backtrace,
}

impl VoiceError {
    /// Un refus de sorte `kind`.
    pub fn new(kind: VoiceErrorKind) -> Self {
        VoiceSnafu { kind }.build()
    }

    /// La sorte du refus.
    pub fn kind(&self) -> VoiceErrorKind {
        self.kind
    }
}

impl Localize for VoiceError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(self.kind.key())
    }
}

/// Implémenter ce trait pour brancher la dictée et l'agent sur la boîte.
///
/// Les trois méthodes peuvent bloquer (disque) : jamais sur le fil async.
pub trait VoicePort: Send + Sync + std::fmt::Debug {
    /// L'état à montrer.
    fn view(&self) -> VoiceView;
    /// Vérifie une demande sans rien changer ; dit ce qu'il faudrait confirmer.
    fn check(&self, event: &VoiceEvent, origin: &Origin) -> Result<Checked, VoiceError>;
    /// Applique une demande, vérifiée à nouveau.
    fn apply(&self, event: VoiceEvent, origin: Origin) -> Result<VoiceApplied, VoiceError>;
}

/// Un port sans dictée ni agent, pour les tests qui n'y touchent pas.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct NoVoice;

#[cfg(any(test, feature = "test-support"))]
impl VoicePort for NoVoice {
    fn view(&self) -> VoiceView {
        VoiceView::default()
    }
    fn check(&self, _: &VoiceEvent, _: &Origin) -> Result<Checked, VoiceError> {
        Err(VoiceError::new(VoiceErrorKind::NoDictation))
    }
    fn apply(&self, _: VoiceEvent, _: Origin) -> Result<VoiceApplied, VoiceError> {
        Err(VoiceError::new(VoiceErrorKind::NoDictation))
    }
}

#[cfg(test)]
#[path = "voice_tests.rs"]
mod voice_tests;
