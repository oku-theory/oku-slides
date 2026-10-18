//! Moteur de langage abstrait : une complétion en entrée, une réponse du modèle en sortie.

use std::fmt::Debug;

use async_trait::async_trait;
use snafu::{Backtrace, Snafu};
use tokio_util::sync::CancellationToken;

use crate::compact::ToolSpec;

/// Étape de la boucle de tour que la complétion sert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Choix des familles d'outils.
    Route,
    /// Appel d'un outil ou réponse finale.
    Act,
}

/// Un message de la conversation présentée au modèle.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Message {
    /// Texte de l'utilisateur.
    User(String),
    /// Texte du modèle.
    Assistant(String),
    /// Résultat d'un appel d'outil.
    ToolResult {
        /// Nom de l'outil appelé.
        name: String,
        /// Sortie de l'outil.
        output: String,
    },
}

/// Ce que l'on demande au moteur : instructions, conversation et outils exposés.
#[derive(Clone, Debug)]
#[cfg_attr(not(debug_assertions), expect(dead_code, reason = "lu par un moteur ; seul le factice existe, en débogage"))]
pub(crate) struct Completion {
    /// Étape servie.
    pub step: Step,
    /// Instructions système.
    #[cfg_attr(
        all(debug_assertions, not(test)),
        expect(dead_code, reason = "lu par les vrais moteurs (Gguf, Apple), pas par le factice")
    )]
    pub system: String,
    /// Conversation jusqu'ici.
    pub messages: Vec<Message>,
    /// Outils que le modèle peut appeler à cette étape.
    pub tools: Vec<ToolSpec>,
}

/// Réponse du modèle.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    not(debug_assertions),
    expect(dead_code, reason = "construit par un moteur ; seul le factice existe, en débogage")
)]
pub(crate) enum Reply {
    /// Texte libre.
    Text(String),
    /// Appel d'un outil.
    Call {
        /// Nom de l'outil.
        name: String,
        /// Arguments JSON.
        args: serde_json::Value,
    },
}

/// Le moteur n'a pas pu répondre.
#[derive(Debug, Snafu)]
pub(crate) struct EngineError(EngineInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum EngineInner {
    #[snafu(display("engine cancelled"))]
    Cancelled { backtrace: Backtrace },
    #[snafu(display("engine failed: {message}"))]
    Failed { message: String, backtrace: Backtrace },
    #[snafu(display("engine script exhausted"))]
    ScriptExhausted { backtrace: Backtrace },
}

impl EngineError {
    /// Vrai si l'annulation du tour a interrompu le moteur.
    pub(crate) fn is_cancelled(&self) -> bool {
        matches!(self.0, EngineInner::Cancelled { .. })
    }
}

#[cfg(all(test, debug_assertions))]
impl EngineError {
    pub(crate) fn is_exhausted(&self) -> bool {
        matches!(self.0, EngineInner::ScriptExhausted { .. })
    }
}

/// Implement this trait to add a local model engine (Gguf, Apple).
#[async_trait]
pub(crate) trait Engine: Send + Sync + Debug {
    /// Produit la prochaine réponse ; rend `Cancelled` dès que `cancel` se déclenche.
    async fn complete(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError>;
}
