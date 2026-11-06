//! Outils MCP d'Oku Slides pour l'agent, servis en HTTP streamable sur
//! `127.0.0.1` avec un jeton par lancement.
//!
//! - [`ToolServer`] : démarre le serveur (port choisi par le système).
//! - [`Tools`] : les outils (lecture, `apply`, surfaces, confirmations).
//! - [`SurfaceHub`] : surfaces ouvertes, réponses attendues, évènements
//!   vers l'interface ([`McpEvent`]).
//! - [`SettingsPort`] : réglages lus et écrits, fournis par l'hôte.
//! - [`VoicePort`] : la dictée et l'agent de l'app, pour la boîte des réglages.
//! - [`TabPort`]: another tab's draft, page and fine-tuning turn, for the host to act on.
//! - [`Lexicon`]: the spoken command words of the user's languages, matched whole.
//! - [`HeardUtterance`]: a hands-free utterance, as the host voice router hands it on.
//!
//! Dépend de `oku-slides-session` ; utilisé par `oku-slides-agent`.

mod a2ui_tools;
mod app_tools;
mod diagnose_tools;
mod diagnosis;
mod draft;
mod edit_tools;
mod error;
mod guarded_tools;
mod handler;
mod heard;
mod history_page;
mod host_tools;
mod hub;
mod import_tools;
mod lexicon;
mod open_tools;
mod paths;
mod read_tools;
mod server;
mod settings;
mod surface_tools;
mod tab_gate;
mod tab_port;
mod tab_tools;
mod tool_error;
mod tool_kinds;
mod tools;
mod undo_tools;
mod usage_tools;
mod voice;
mod voice_tools;
mod widget_tools;
mod workspace_tools;

pub use app_tools::{FoldedChange, InterfaceChange, PresentCommand, PresentMode, SchemeChoice};
pub use diagnosis::{AgentDiagnosis, Diagnosis, DictationDiagnosis};
pub use draft::MAX_REQUEST_BYTES;
pub use error::McpError;
pub use heard::HeardUtterance;
pub use hub::{
    ARMING, AfterTurn, Answer, AnswerOutcome, CLOSE_DECISION_TTL, CloseDecision, CloseTarget, Freeze, HubError,
    HubErrorKind, McpEvent, STATIC_PER_TURN, ShowOutcome, SurfaceAction, SurfaceHub, THREAD_BYTES,
};
pub use lexicon::{ConfirmAction, ConfirmAsk, ConfirmVerb, Lexicon, Nav, YesNo};
pub use server::ToolServer;
pub use settings::{Applied, SettingsPort};
#[cfg(any(test, feature = "test-support"))]
pub use tab_port::NoTabs;
pub use tab_port::{Prefill, TabPort, TuneRefusal};
pub use tools::{HostPorts, Tools, tool_descriptions_for, tool_names, tool_names_for};
#[cfg(any(test, feature = "test-support"))]
pub use voice::NoVoice;
pub use voice::{ApiTarget, Checked, Origin, VoiceApplied, VoiceError, VoiceErrorKind, VoicePort};
