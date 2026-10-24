//! Ce que l'agent fait, pour le fil de l'interface.

use agent_client_protocol::schema::v1::{ContentBlock, SessionUpdate, ToolCallContent};
use oku_slides_session::agent::{AgentScope, AttachmentRef, DETAIL_CHARS, PromptImage, clip};
use serde::Serialize;
use serde_json::Value;

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
#[derive(Debug, Clone, PartialEq, Serialize)]
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
        /// Les images collées avec la consigne.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        images: Vec<PromptImage>,
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
        /// Arguments en JSON indenté, bornés ([`DETAIL_CHARS`]).
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<String>,
        /// Résultat en texte, borné.
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    ToolUpdate {
        turn: u32,
        id: String,
        title: Option<String>,
        status: Option<String>,
        /// Nouveaux arguments, comme ceux de `ToolCall`.
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<String>,
        /// Nouveau résultat, comme celui de `ToolCall`.
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
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
        /// Les images collées du brouillon ; l'hôte en est seul juge, chaque vue les reprend.
        images: Vec<AttachmentRef>,
    },
    /// L'agent choisi est-il utilisable ? `unusable` : pourquoi non, localisé.
    Availability {
        unusable: Option<String>,
    },
    /// La dictée a changé (réglages, modèles, état) : l'interface relit son état.
    Voice,
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

/// Les arguments d'un appel, en JSON indenté et bornés ; `None` s'il n'y a rien à montrer.
fn tool_input(raw: Option<&Value>) -> Option<String> {
    match raw? {
        Value::Null => None,
        Value::Object(m) if m.is_empty() => None,
        Value::Array(a) if a.is_empty() => None,
        Value::String(t) if t.trim().is_empty() => None,
        // Une chaîne se lit telle quelle, sans guillemets ni échappements.
        Value::String(t) => Some(clip(t, DETAIL_CHARS)),
        v => serde_json::to_string_pretty(v).ok().map(|t| clip(&t, DETAIL_CHARS)),
    }
}

/// Le résultat d'un appel en texte, borné ; `None` s'il est vide.
///
/// Le texte de son contenu (fichiers modifiés compris), sinon sa sortie brute : ses textes MCP,
/// ou son JSON.
fn tool_output(content: &[ToolCallContent], raw: Option<&Value>) -> Option<String> {
    let mut parts: Vec<String> = content
        .iter()
        .filter_map(|c| match c {
            ToolCallContent::Content(c) => match &c.content {
                // Un texte vide ne cache pas la sortie brute.
                ContentBlock::Text(t) if !t.text.trim().is_empty() => Some(t.text.clone()),
                _ => None,
            },
            ToolCallContent::Diff(d) => Some(d.path.display().to_string()),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        match raw {
            Some(Value::String(t)) => parts.push(t.clone()),
            // Une sortie MCP (chaque élément typé) : ses textes ; ses images et ressources n'ont rien à lire.
            Some(Value::Array(items)) if !items.is_empty() && items.iter().all(|i| i.get("type").is_some()) => {
                parts.extend(items.iter().filter_map(|i| i.get("text").and_then(Value::as_str).map(str::to_owned)));
            }
            Some(Value::Null) | None => {}
            Some(Value::Array(a)) if a.is_empty() => {}
            Some(Value::Object(m)) if m.is_empty() => {}
            Some(v) => parts.extend(serde_json::to_string_pretty(v).ok()),
        }
    }
    let text = parts.join("\n\n");
    let text = clip(text.trim(), DETAIL_CHARS);
    (!text.trim().is_empty()).then_some(text)
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
            input: tool_input(tc.raw_input.as_ref()),
            output: tool_output(&tc.content, tc.raw_output.as_ref()),
            title: tc.title,
            status: as_text(&tc.status),
        }),
        SessionUpdate::ToolCallUpdate(u) => Some(AgentEvent::ToolUpdate {
            turn,
            id: as_text(&u.tool_call_id),
            input: tool_input(u.fields.raw_input.as_ref()),
            output: tool_output(u.fields.content.as_deref().unwrap_or_default(), u.fields.raw_output.as_ref()),
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
