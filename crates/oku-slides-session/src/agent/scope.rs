//! La portée d'une conversation de l'agent.
//!
//! Les slides, l'atelier d'un pack, ou un widget d'un pack : chacune a son fil et sa session ACP.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Où l'utilisateur parle à l'agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentScope {
    /// Le deck (et son historique).
    #[default]
    Slides,
    /// Le mode pack du pack `pack` : « Modèles » et « Thème ».
    Atelier { pack: Uuid },
    /// Le sous-onglet du widget `name` du pack `pack`.
    Widget { pack: Uuid, name: String },
}

impl AgentScope {
    /// Nom du fichier de reprise ; longueur fixe, quel que soit le nom du widget.
    pub fn file_name(&self) -> String {
        match self {
            Self::Slides => "slides.json".to_owned(),
            Self::Atelier { pack } => format!("atelier.{}.json", pack.simple()),
            Self::Widget { pack, name } => {
                let digest = hex::encode(Sha256::digest(name.as_bytes()));
                format!("widget.{}.{}.json", pack.simple(), &digest[..32])
            }
        }
    }
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod scope_tests;
