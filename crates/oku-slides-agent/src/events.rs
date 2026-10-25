//! Ce que l'agent fait, pour le fil de l'interface.

use agent_client_protocol::schema::v1::{ContentBlock, SessionUpdate, ToolCallContent};
use oku_slides_session::agent::{AttachmentRef, DETAIL_CHARS, PromptImage, clip};
use serde::Serialize;
use serde_json::Value;

/// The state of a tab's agent, shown by the thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Idle: no turn is running.
    Idle,
    /// Starting: the ACP handshake is not finished.
    Starting,
    /// A turn is running.
    Working,
}

/// One step of the plan the agent announces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanItem {
    /// Text of the step, as the agent writes it.
    pub content: String,
    /// `pending`, `in_progress`, `completed`.
    pub status: String,
}

/// An event of the thread (JSON `{"kind": …}`).
///
/// Everything that happens during a turn carries that turn's number (`turn`, given by
/// `SurfaceHub::begin_turn`): the turn reducer files each event in its own turn, even if
/// the turn's opening (`UserText`) was lost (late relay).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    /// The agent's state.
    Status {
        /// Where the agent stands.
        state: AgentState,
    },
    /// The spoken instruction, which opens turn `turn`.
    UserText {
        /// Number of the opened turn.
        turn: u32,
        /// The instruction, as spoken or typed.
        text: String,
        /// The images pasted with the instruction.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        images: Vec<PromptImage>,
    },
    /// A chunk of the agent's answer.
    Chunk {
        /// Number of the turn.
        turn: u32,
        /// The received text, to append to the previous one.
        text: String,
    },
    /// The agent calls a tool.
    ToolCall {
        /// Number of the turn.
        turn: u32,
        /// ACP id of the call, which later updates refer to.
        id: String,
        /// Title of the call, as the agent gives it.
        title: String,
        /// ACP status of the call (`pending`, `in_progress`, `completed`, `failed`).
        status: String,
        /// Arguments as indented JSON, bounded ([`DETAIL_CHARS`]).
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<String>,
        /// Result as text, bounded.
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    /// An update to a tool call already shown.
    ToolUpdate {
        /// Number of the turn.
        turn: u32,
        /// ACP id of the updated call.
        id: String,
        /// New title; absent means unchanged.
        title: Option<String>,
        /// New status; absent means unchanged.
        status: Option<String>,
        /// New arguments, like those of `ToolCall`.
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<String>,
        /// New result, like that of `ToolCall`.
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    /// The agent's plan, replaced as a whole on each update.
    Plan {
        /// Number of the turn.
        turn: u32,
        /// The plan's steps, in order.
        entries: Vec<PlanItem>,
    },
    /// End of the turn: `end_turn`, `cancelled`, `max_tokens`…
    TurnEnded {
        /// Number of the ended turn.
        turn: u32,
        /// ACP stop reason.
        stop: String,
    },
    /// Message in the user's language.
    Error {
        /// Number of the failed turn.
        turn: u32,
        /// The localized message.
        message: String,
    },
    /// New conversation: the tab's thread starts over.
    Conversation,
    /// The tab's draft (`None`: closed), as the host holds it for all its views.
    Draft {
        /// The draft's text; `None` when it is closed.
        text: Option<String>,
        /// The view that wrote it; absent when the host replays it. A view ignores its own echoes.
        origin: Option<String>,
        /// The draft's pasted images; the host alone decides, every view takes them up.
        images: Vec<AttachmentRef>,
    },
    /// Is the chosen agent usable? `unusable`: why not, localized.
    Availability {
        /// Why the agent is unusable, localized; `None` when it is usable.
        unusable: Option<String>,
    },
    /// Dictation changed (settings, models, state): the interface reads its state again.
    Voice,
}

impl AgentEvent {
    /// The agent at rest.
    #[must_use]
    pub fn idle() -> Self {
        Self::Status { state: AgentState::Idle }
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
