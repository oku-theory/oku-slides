use thiserror::Error;

/// Erreurs de validation, de normalisation et de rendu.
///
/// Les messages sont destinés à l'utilisateur et au LLM : ils disent quoi
/// corriger, pas seulement ce qui a échoué.
#[derive(Debug, Error)]
pub enum ChartError {
    /// Options hors du sous-ensemble v1 (champ inconnu, type invalide…).
    #[error("options invalides : {0}")]
    InvalidOption(String),

    /// Options valides syntaxiquement mais incohérentes.
    #[error("graphique incohérent : {0}")]
    Inconsistent(String),

    /// Opération sur les données impossible (index hors limites…).
    #[error("opération sur les données impossible : {0}")]
    DataOp(String),

    /// Le patch touche `dataset` : il faut passer par `set_chart_data`.
    #[error("le patch d'options ne peut pas modifier `dataset` : utiliser set_chart_data")]
    PatchTouchesDataset,

    /// Échec de charts-rs.
    #[error("rendu impossible : {0}")]
    Render(String),
}

impl From<serde_json::Error> for ChartError {
    fn from(e: serde_json::Error) -> Self {
        ChartError::InvalidOption(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ChartError>;
