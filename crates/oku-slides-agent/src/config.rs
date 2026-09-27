//! Comment lancer l'agent (un seul sous-processus pour l'app).

use std::path::PathBuf;
use std::time::Duration;

/// Délai pour qu'un agent lancé soit prêt : large, `npx` peut d'abord le télécharger.
pub const READY_WITHIN: Duration = Duration::from_secs(120);

/// Lancement de l'agent ; les outils MCP sont propres à chaque onglet
/// (voir `ToolsAccess`).
#[derive(Debug, Clone)]
pub struct Launch {
    pub command: String,
    pub args: Vec<String>,
    /// Dossier de travail d'une session quand son document n'est pas enregistré.
    pub fallback_cwd: PathBuf,
    /// Au-delà, le lancement est abandonné (sous-processus tué) : l'agent est arrêté.
    pub ready_within: Duration,
}
