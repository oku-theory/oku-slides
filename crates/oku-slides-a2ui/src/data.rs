//! Composants de données d'un graphique ou d'un tableau (`ChartData`, `TableData`), hôte seul.
//!
//! La session les construit depuis le deck ; l'interface écrit une modification au chemin
//! `edit` du modèle, puis envoie l'action, que la session applique en une op annulable.

use serde::{Deserialize, Serialize};

/// Une série qu'on peut colorer : son nom (en-tête de sa colonne) et sa couleur propre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesSwatch {
    /// Le nom de la série, lu dans l'en-tête de sa colonne.
    pub name: String,
    /// Couleur `#rrggbb` propre à la série ; absente : la palette du thème.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
