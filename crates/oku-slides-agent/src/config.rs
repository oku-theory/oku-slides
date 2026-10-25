//! Comment lancer l'agent (un seul sous-processus pour l'app).

use std::path::{Path, PathBuf};
use std::time::Duration;

use oku_slides_platform::config::{AgentPreset, Builtin};

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
    /// Variables ajoutées à l'environnement de l'agent (tests : moteur factice de l'agent local).
    pub env: Vec<(String, String)>,
    /// Dossier de travail d'une session quand son document n'est pas enregistré.
    pub fallback_cwd: PathBuf,
    /// Au-delà, le lancement est abandonné (sous-processus tué) : l'agent est arrêté.
    pub ready_within: Duration,
}

impl Launch {
    /// Le lancement de `preset` ; un agent intégré relance `program` avec son argument.
    ///
    /// `None` : agent intégré et pas de binaire de l'app connu (tests, outils).
    #[must_use]
    pub fn for_preset(preset: &AgentPreset, program: Option<&Path>, fallback_cwd: PathBuf) -> Option<Self> {
        let (command, args) = match preset.builtin {
            None => (preset.command.clone(), preset.args.clone()),
            Some(Builtin::Local) => {
                (program?.to_string_lossy().into_owned(), vec![oku_slides_platform::LOCAL_AGENT_FLAG.to_owned()])
            }
        };
        Some(Self { command, args, env: Vec::new(), fallback_cwd, ready_within: READY_WITHIN })
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
