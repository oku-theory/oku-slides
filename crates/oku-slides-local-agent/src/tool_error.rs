//! Erreurs des outils MCP : appel à réparer par le modèle, ou serveur injoignable.
//!
//! Elles ne vont que dans les journaux et dans le texte `error: …` rendu au modèle.

use rust_mcp_sdk::error::McpSdkError;
use snafu::{Backtrace, IntoError, Snafu};

/// Un appel d'outil ou une liste d'outils a échoué.
#[derive(Debug, Snafu)]
pub(crate) struct ToolError(ToolInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ToolInner {
    /// Le serveur refuse l'appel (résultat en erreur, erreur JSON-RPC, outil inconnu, arguments) : le modèle peut le réparer.
    #[snafu(display("{message}"))]
    Rejected { message: String, backtrace: Backtrace },
    /// La connexion au serveur d'outils a échoué : le tour échoue.
    #[snafu(display("tool server unreachable"))]
    Transport {
        #[snafu(source(from(McpSdkError, Box::new)))]
        source: Box<McpSdkError>,
        backtrace: Backtrace,
    },
    /// Un schéma d'outil n'est pas du JSON valide.
    #[snafu(display("invalid schema for tool {name}"))]
    Schema { name: String, source: serde_json::Error, backtrace: Backtrace },
}

impl ToolError {
    /// Vrai si le modèle peut réparer l'appel ; faux si le tour doit échouer.
    pub(crate) fn is_rejected(&self) -> bool {
        matches!(self.0, ToolInner::Rejected { .. })
    }

    /// Un appel refusé, avec le texte à montrer au modèle.
    pub(crate) fn rejected(message: impl Into<String>) -> Self {
        RejectedSnafu { message: message.into() }.build().into()
    }

    /// Une erreur de transport, sans serveur à joindre.
    #[cfg(all(test, debug_assertions))]
    pub(crate) fn transport_for_test() -> Self {
        TransportSnafu.into_error(McpSdkError::Internal { description: "down".into() }).into()
    }
}

/// Une erreur JSON-RPC du serveur se répare ; tout le reste est un échec de transport.
impl From<McpSdkError> for ToolError {
    fn from(error: McpSdkError) -> Self {
        match error {
            McpSdkError::RpcError(rpc) => Self::rejected(rpc.message),
            other => TransportSnafu.into_error(other).into(),
        }
    }
}

#[cfg(test)]
#[path = "tool_error_tests.rs"]
mod tool_error_tests;
