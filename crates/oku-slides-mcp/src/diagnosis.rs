//! What the host tells the `diagnose` tool: the app agent and the dictation.

use serde::Serialize;

/// The host's part of a `diagnose` reply, given by [`crate::VoicePort::diagnosis`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Diagnosis {
    /// The selected app agent.
    pub agent: AgentDiagnosis,
    /// The dictation; `None` when the host has none.
    pub dictation: Option<DictationDiagnosis>,
}

/// The selected app agent, and why it cannot be used.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AgentDiagnosis {
    /// Its preset id (an `[agent.presets]` key).
    pub selected: String,
    /// Its name, shown as is.
    pub name: String,
    /// True for the agent built into Oku Slides.
    pub builtin: bool,
    /// Why it is unusable, localized; `None` when it seems usable.
    pub unusable: Option<String>,
    /// The last lines (at most 40) it wrote to stderr since its last launch.
    pub stderr: Vec<String>,
}

/// The dictation's state, as the troubleshooting agent needs it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DictationDiagnosis {
    /// Microphone permission: `granted`, `denied`, `restricted`, `undetermined` or `unknown`.
    pub microphone: String,
    /// The chosen engine: `apple`, `turbo`, `large`, `base` or `api`.
    pub engine: String,
    /// The engine's files: `missing`, `partial` or `complete`.
    pub model: String,
    /// The dictation language code, or `auto`.
    pub language: String,
}
