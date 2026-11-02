//! The local agent's ACP sessions: one per tab, with its MCP server.
//!
//! A tab's MCP URL names it: a `session/new` or `session/load` on a URL drops the other
//! sessions of that URL, so their MCP clients close. A loaded session keeps its summaries.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use crate::mcp::McpTools;
use crate::tool_error::ToolError;

/// Nombre de résumés de tours gardés par session.
const KEPT_SUMMARIES: usize = 3;

/// Characters of the request kept in a turn summary.
pub(crate) const SUMMARY_CHARS: usize = 160;

/// One line telling a turn: its request on one line, cut to `SUMMARY_CHARS`, and its outcome.
pub(crate) fn turn_summary(request: &str, outcome: &str) -> String {
    let line = request.split_whitespace().collect::<Vec<_>>().join(" ");
    let request = match line.char_indices().nth(SUMMARY_CHARS) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line,
    };
    format!("{request} → {outcome}")
}

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

    /// The same session on `mcp`: its summaries and turn count kept, its MCP client renewed.
    fn reopened(&self, mcp: McpAccess) -> Self {
        let summaries = self.summaries.lock().unwrap_or_else(PoisonError::into_inner).clone();
        Self {
            summaries: Mutex::new(summaries),
            turns: AtomicUsize::new(self.turns.load(Ordering::Relaxed)),
            ..Self::new(mcp)
        }
    }

    /// Cancels the running turn, if any.
    pub(crate) fn cancel_turn(&self) {
        self.cancel.lock().unwrap_or_else(PoisonError::into_inner).cancel();
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

/// The open sessions, numbered `local-<nonce>-1`, `local-<nonce>-2`…
///
/// The nonce is random per table, so an id saved by an earlier `--local-agent` process never
/// names a session of this one: the host reopens saved ids after every relaunch.
#[derive(Debug)]
pub(crate) struct Sessions {
    nonce: String,
    count: AtomicUsize,
    map: Mutex<HashMap<String, Arc<SessionState>>>,
}

impl Default for Sessions {
    fn default() -> Self {
        Self { nonce: uuid::Uuid::new_v4().simple().to_string(), count: AtomicUsize::new(0), map: Mutex::default() }
    }
}

impl Sessions {
    /// Opens a session on this MCP server, dropping the server's previous ones; returns its id.
    pub(crate) fn open(&self, mcp: McpAccess) -> String {
        let id = format!("local-{}-{}", self.nonce, self.count.fetch_add(1, Ordering::Relaxed) + 1);
        self.install(id.clone(), SessionState::new(mcp));
        id
    }

    /// Loads session `id` on `mcp`, dropping the server's other sessions.
    ///
    /// `false` if `id` is unknown or was opened for another MCP URL (another tab): a tab never
    /// takes over another tab's session, its deck or its summaries.
    pub(crate) fn load(&self, id: &str, mcp: McpAccess) -> bool {
        let Some(previous) = self.get(id) else { return false };
        if previous.mcp.url != mcp.url {
            return false;
        }
        previous.cancel_turn();
        self.install(id.to_owned(), previous.reopened(mcp));
        true
    }

    /// Drops every session (the ACP connection closed), cancelling their turns.
    pub(crate) fn clear(&self) {
        let gone = std::mem::take(&mut *self.map.lock().unwrap_or_else(PoisonError::into_inner));
        for session in gone.values() {
            session.cancel_turn();
        }
    }

    /// Puts `state` under `id`, removing the sessions on the same MCP URL; their turns are cancelled.
    fn install(&self, id: String, state: SessionState) {
        let mut map = self.map.lock().unwrap_or_else(PoisonError::into_inner);
        let url = state.mcp.url.clone();
        let replaced: Vec<String> = map.iter().filter(|(_, s)| s.mcp.url == url).map(|(k, _)| k.clone()).collect();
        for key in replaced {
            if let Some(old) = map.remove(&key) {
                old.cancel_turn();
            }
        }
        map.insert(id, Arc::new(state));
    }

    /// Number of open sessions.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.lock().unwrap_or_else(PoisonError::into_inner).len()
    }

    /// La session `id`, si elle est ouverte.
    pub(crate) fn get(&self, id: &str) -> Option<Arc<SessionState>> {
        self.map.lock().unwrap_or_else(PoisonError::into_inner).get(id).cloned()
    }
}

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod sessions_tests;
