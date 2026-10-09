//! Ce que l'agent fait, pour le fil de l'interface.

use agent_client_protocol::schema::v1::{ContentBlock, SessionUpdate};
use oku_slides_session::agent::AgentScope;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Starting,
    Working,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanItem {
    pub content: String,
    /// `pending`, `in_progress`, `completed`.
    pub status: String,
}

/// Un évènement du fil (JSON `{"kind": …}`).
///
/// Tout ce qui arrive pendant un tour porte le numéro de ce tour (`turn`, donné par
/// `SurfaceHub::begin_turn`) : le réducteur des tours range chaque évènement dans le sien,
/// même si l'ouverture du tour (`UserText`) s'est perdue (relais en retard).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    /// L'état de l'agent, avec la portée qu'il sert quand il travaille.
    Status {
        state: AgentState,
        /// La portée servie ; absente au repos.
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<AgentScope>,
    },
    /// La consigne dite, qui ouvre le tour `turn` de la portée `scope`.
    UserText {
        turn: u32,
        /// La portée du tour.
        scope: AgentScope,
        text: String,
    },
    /// Un morceau de la réponse de l'agent.
    Chunk {
        turn: u32,
        text: String,
    },
    ToolCall {
        turn: u32,
        id: String,
        title: String,
        status: String,
    },
    ToolUpdate {
        turn: u32,
        id: String,
        title: Option<String>,
        status: Option<String>,
    },
    Plan {
        turn: u32,
        entries: Vec<PlanItem>,
    },
    /// Fin du tour : `end_turn`, `cancelled`, `max_tokens`…
    TurnEnded {
        turn: u32,
        stop: String,
    },
    /// Message dans la langue de l'utilisateur.
    Error {
        turn: u32,
        message: String,
    },
    /// Nouvelle conversation : le fil de la portée repart de zéro.
    Conversation {
        /// La portée dont le fil est oublié.
        scope: AgentScope,
    },
    /// Le brouillon de l'onglet (`None` : fermé), tel que l'hôte le tient pour toutes ses vues.
    Draft {
        text: Option<String>,
        /// La vue qui l'a écrit ; absente quand l'hôte le rejoue. Une vue ignore ses propres échos.
        origin: Option<String>,
    },
}

impl AgentEvent {
    /// L'agent au repos, sans portée.
    pub fn idle() -> Self {
        Self::Status { state: AgentState::Idle, scope: None }
    }
}

/// Une valeur ACP sérialisée en chaîne (`"in_progress"`, un identifiant…).
pub(crate) fn as_text<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// L'évènement d'une mise à jour de session du tour `turn` ; `None` pour ce que le fil n'affiche pas.
pub(crate) fn from_update(turn: u32, update: SessionUpdate) -> Option<AgentEvent> {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => match chunk.content {
            ContentBlock::Text(t) => Some(AgentEvent::Chunk { turn, text: t.text }),
            _ => None,
        },
        SessionUpdate::ToolCall(tc) => Some(AgentEvent::ToolCall {
            turn,
            id: as_text(&tc.tool_call_id),
            title: tc.title,
            status: as_text(&tc.status),
        }),
        SessionUpdate::ToolCallUpdate(u) => Some(AgentEvent::ToolUpdate {
            turn,
            id: as_text(&u.tool_call_id),
            title: u.fields.title,
            status: u.fields.status.as_ref().map(as_text),
        }),
        SessionUpdate::Plan(p) => Some(AgentEvent::Plan {
            turn,
            entries: p
                .entries
                .iter()
                .map(|e| PlanItem { content: e.content.clone(), status: as_text(&e.status) })
                .collect(),
        }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod events_tests;
