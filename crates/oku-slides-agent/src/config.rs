//! How to launch the agent (one subprocess for the app).

use std::path::PathBuf;
use std::time::Duration;

use oku_slides_platform::ProgramPath;
use oku_slides_platform::config::AgentPreset;

/// The agent process's working folder, under the data folder.
pub(crate) const AGENT_PROCESS_DIR: &str = "agent-process";

/// Délai pour qu'un agent lancé soit prêt : large, `npx` peut d'abord le télécharger.
pub const READY_WITHIN: Duration = Duration::from_secs(120);

/// Lancement de l'agent ; les outils MCP sont propres à chaque onglet
/// (voir `ToolsAccess`).
#[derive(Debug, Clone)]
pub struct Launch {
    /// The ACP agent's program, looked up in `PATH` if it is not a path.
    pub command: String,
    /// Arguments passed to `command`, in order.
    pub args: Vec<String>,
    /// The `PATH` where `command` is looked up, also the agent's own; `None`: the process's.
    pub path: Option<ProgramPath>,
    /// The agent process's working folder: the app's own, `<data>/agent-process`.
    ///
    /// Emptied through the link's storage before each launch; a session's own folder is its tab's.
    pub process_dir: PathBuf,
    /// Au-delà, le lancement est abandonné (sous-processus tué) : l'agent est arrêté.
    pub ready_within: Duration,
}

impl Launch {
    /// Le lancement de `preset` : sa commande et ses arguments.
    #[must_use]
    pub fn for_preset(preset: &AgentPreset, process_dir: PathBuf) -> Self {
        Self {
            command: preset.command.clone(),
            args: preset.args.clone(),
            path: None,
            process_dir,
            ready_within: READY_WITHIN,
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
