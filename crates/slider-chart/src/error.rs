//! Erreurs des graphiques : validation, normalisation, données et rendu.
//!
//! Les raisons sont destinées à l'utilisateur et au LLM : elles disent quoi
//! corriger, pas seulement ce qui a échoué.

use snafu::{Backtrace, Snafu};

/// Un graphique refusé ou impossible à rendre.
#[derive(Debug, Snafu)]
pub struct ChartError(pub(crate) ChartInner);

/// Nature d'une [`ChartError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChartErrorKind {
    /// Options hors du sous-ensemble v1 (champ inconnu, type invalide…).
    InvalidOption,
    /// Options valides mais incohérentes.
    Inconsistent,
    /// Opération sur les données impossible (index hors limites…).
    DataOp,
    /// Le patch touche `dataset` : il faut passer par `set_chart_data`.
    PatchTouchesDataset,
    /// Échec de charts-rs.
    Render,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ChartInner {
    #[snafu(display("invalid options"))]
    InvalidOption { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("invalid options: {reason}"))]
    InvalidPatch { reason: &'static str, backtrace: Backtrace },
    #[snafu(display("inconsistent chart: {reason}"))]
    Inconsistent { reason: String, backtrace: Backtrace },
    #[snafu(display("impossible data operation: {reason}"))]
    DataOp { reason: String, backtrace: Backtrace },
    #[snafu(display("an option patch cannot change `dataset`: use set_chart_data"))]
    PatchTouchesDataset { backtrace: Backtrace },
    #[snafu(display("chart rendering failed"))]
    Render { source: charts_rs::Error, backtrace: Backtrace },
}

impl ChartError {
    pub fn kind(&self) -> ChartErrorKind {
        match self.0 {
            ChartInner::InvalidOption { .. } | ChartInner::InvalidPatch { .. } => ChartErrorKind::InvalidOption,
            ChartInner::Inconsistent { .. } => ChartErrorKind::Inconsistent,
            ChartInner::DataOp { .. } => ChartErrorKind::DataOp,
            ChartInner::PatchTouchesDataset { .. } => ChartErrorKind::PatchTouchesDataset,
            ChartInner::Render { .. } => ChartErrorKind::Render,
        }
    }

    /// Ce qu'il faut corriger, pour l'utilisateur ou le LLM (champ fautif,
    /// raison de l'incohérence…).
    pub fn reason(&self) -> String {
        match &self.0 {
            ChartInner::InvalidOption { source, .. } => source.to_string(),
            ChartInner::InvalidPatch { reason, .. } => (*reason).to_owned(),
            ChartInner::Inconsistent { reason, .. } | ChartInner::DataOp { reason, .. } => reason.clone(),
            ChartInner::PatchTouchesDataset { .. } => self.to_string(),
            ChartInner::Render { source, .. } => source.to_string(),
        }
    }
}

pub type Result<T, E = ChartError> = std::result::Result<T, E>;
