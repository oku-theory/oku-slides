//! Ce que l'agent fait, pour le fil de l'interface.

use agent_client_protocol::schema::v1::{ContentBlock, SessionUpdate};
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    Status {
        state: AgentState,
    },
    /// La consigne dite, qui ouvre le tour `turn`.
    UserText {
        turn: u32,
        text: String,
    },
    /// Un morceau de la réponse de l'agent.
    Chunk {
        text: String,
    },
    ToolCall {
        id: String,
        title: String,
        status: String,
    },
    ToolUpdate {
        id: String,
        title: Option<String>,
        status: Option<String>,
    },
    Plan {
        entries: Vec<PlanItem>,
    },
    /// Fin du tour : `end_turn`, `cancelled`, `max_tokens`…
    TurnEnded {
        stop: String,
    },
    /// Message dans la langue de l'utilisateur.
    Error {
        message: String,
    },
    /// Nouvelle conversation : le fil repart de zéro.
    Conversation,
}

/// Une valeur ACP sérialisée en chaîne (`"in_progress"`, un identifiant…).
pub(crate) fn as_text<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// L'évènement d'une mise à jour de session ; `None` pour ce que le fil n'affiche pas.
pub(crate) fn from_update(update: SessionUpdate) -> Option<AgentEvent> {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => match chunk.content {
            ContentBlock::Text(t) => Some(AgentEvent::Chunk { text: t.text }),
            _ => None,
        },
        SessionUpdate::ToolCall(tc) => {
            Some(AgentEvent::ToolCall { id: as_text(&tc.tool_call_id), title: tc.title, status: as_text(&tc.status) })
        }
        SessionUpdate::ToolCallUpdate(u) => Some(AgentEvent::ToolUpdate {
            id: as_text(&u.tool_call_id),
            title: u.fields.title,
            status: u.fields.status.as_ref().map(as_text),
        }),
        SessionUpdate::Plan(p) => Some(AgentEvent::Plan {
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
