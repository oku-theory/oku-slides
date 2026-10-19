//! Sessions ACP de l'agent local : une par onglet, avec son serveur MCP.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use crate::mcp::McpTools;
use crate::tool_error::ToolError;

/// Nombre de résumés de tours gardés par session.
const KEPT_SUMMARIES: usize = 3;

/// URL et en-tête `Authorization` du serveur MCP d'une session.
#[derive(Clone)]
pub(crate) struct McpAccess {
    pub(crate) url: String,
    pub(crate) authorization: String,
}

/// Le jeton ne doit jamais apparaître dans les journaux.
impl std::fmt::Debug for McpAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpAccess").field("url", &self.url).field("authorization", &"<redacted>").finish()
    }
}

/// État d'une session : son accès MCP, son annulation et ses derniers résumés.
#[derive(Debug)]
pub(crate) struct SessionState {
    pub(crate) mcp: McpAccess,
    /// Jeton du tour en cours ; `session/cancel` l'annule.
    pub(crate) cancel: Mutex<CancellationToken>,
    /// Résumés des derniers tours, relus par le tour suivant.
    pub(crate) summaries: Mutex<VecDeque<String>>,
    /// Client MCP, connecté au premier prompt puis réutilisé.
    tools: OnceCell<McpTools>,
    /// Tours commencés, pour des identifiants d'appels uniques dans la session.
    turns: AtomicUsize,
}

impl SessionState {
    fn new(mcp: McpAccess) -> Self {
        Self {
            mcp,
            cancel: Mutex::new(CancellationToken::new()),
            summaries: Mutex::new(VecDeque::new()),
            tools: OnceCell::new(),
            turns: AtomicUsize::new(0),
        }
    }

    /// Commence un tour : annule le précédent, rend son numéro et son nouveau jeton.
    pub(crate) fn begin_turn(&self) -> (usize, CancellationToken) {
        let fresh = CancellationToken::new();
        let previous =
            std::mem::replace(&mut *self.cancel.lock().unwrap_or_else(PoisonError::into_inner), fresh.clone());
        previous.cancel();
        (self.turns.fetch_add(1, Ordering::Relaxed) + 1, fresh)
    }

    /// Les résumés des derniers tours, du plus ancien au plus récent.
    pub(crate) fn summaries(&self) -> Vec<String> {
        self.summaries.lock().unwrap_or_else(PoisonError::into_inner).iter().cloned().collect()
    }

    /// Ajoute le résumé d'un tour, en ne gardant que les trois derniers.
    pub(crate) fn push_summary(&self, summary: String) {
        let mut kept = self.summaries.lock().unwrap_or_else(PoisonError::into_inner);
        kept.push_back(summary);
        while kept.len() > KEPT_SUMMARIES {
            kept.pop_front();
        }
    }

    /// Le client MCP de la session, connecté à la première demande.
    pub(crate) async fn tools(&self) -> Result<&McpTools, ToolError> {
        self.tools.get_or_try_init(|| McpTools::connect(&self.mcp)).await
    }
}

/// Table des sessions ouvertes, numérotées `local-1`, `local-2`…
#[derive(Debug, Default)]
pub(crate) struct Sessions {
    count: AtomicUsize,
    map: Mutex<HashMap<String, Arc<SessionState>>>,
}

impl Sessions {
    /// Ouvre une session sur ce serveur MCP et renvoie son identifiant.
    pub(crate) fn open(&self, mcp: McpAccess) -> String {
        let id = format!("local-{}", self.count.fetch_add(1, Ordering::Relaxed) + 1);
        let state = Arc::new(SessionState::new(mcp));
        self.map.lock().unwrap_or_else(PoisonError::into_inner).insert(id.clone(), state);
        id
    }

    /// La session `id`, si elle est ouverte.
    pub(crate) fn get(&self, id: &str) -> Option<Arc<SessionState>> {
        self.map.lock().unwrap_or_else(PoisonError::into_inner).get(id).cloned()
    }
}

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod sessions_tests;
