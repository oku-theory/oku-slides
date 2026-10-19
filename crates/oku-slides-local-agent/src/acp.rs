//! Serveur ACP de l'agent local : poignée de main, sessions, tours, annulation.

use std::sync::{Arc, Mutex, PoisonError};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    McpCapabilities, McpServer, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, SessionId,
    SessionNotification, SessionUpdate, StopReason, TextContent, ToolCall, ToolCallStatus, ToolCallUpdate,
    ToolCallUpdateFields,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Responder, Stdio};
use serde_json::Value;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;

use crate::engine::Engine;
use crate::error::{AcpSnafu, LocalAgentError};
use crate::prompt::read_prompt;
use crate::sessions::{McpAccess, SessionState, Sessions};
use crate::turn::{TurnEnd, Updates, run_turn};

/// Le serveur MCP HTTP de l'onglet et son en-tête `Authorization`.
fn http_access(servers: &[McpServer]) -> Option<McpAccess> {
    servers.iter().find_map(|s| match s {
        McpServer::Http(h) => {
            let authorization = h.headers.iter().find(|x| x.name == "Authorization")?.value.clone();
            Some(McpAccess { url: h.url.clone(), authorization })
        }
        _ => None,
    })
}

/// Progression d'un tour envoyée au client ACP, avec le titre du dernier `report`.
struct AcpUpdates {
    cx: ConnectionTo<Client>,
    session: SessionId,
    /// Numéro du tour : préfixe les identifiants d'appels, uniques dans la session.
    turn: usize,
    title: Mutex<Option<String>>,
}

impl AcpUpdates {
    fn send(&self, update: SessionUpdate) {
        if let Err(e) = self.cx.send_notification(SessionNotification::new(self.session.clone(), update)) {
            tracing::warn!(error = %e, "mise à jour de session de l'agent local impossible");
        }
    }

    fn wire_id(&self, id: &str) -> String {
        format!("t{}-{id}", self.turn)
    }
}

impl Updates for AcpUpdates {
    fn tool_started(&self, id: &str, name: &str, args: &Value) {
        if name == "report" {
            let title = args.get("title").and_then(Value::as_str).map(str::to_owned);
            *self.title.lock().unwrap_or_else(PoisonError::into_inner) = title;
        }
        let call =
            ToolCall::new(self.wire_id(id), name.to_owned()).status(ToolCallStatus::InProgress).raw_input(args.clone());
        self.send(SessionUpdate::ToolCall(call));
    }

    fn tool_finished(&self, id: &str, ok: bool) {
        let status = if ok { ToolCallStatus::Completed } else { ToolCallStatus::Failed };
        let update = ToolCallUpdate::new(self.wire_id(id), ToolCallUpdateFields::new().status(status));
        self.send(SessionUpdate::ToolCallUpdate(update));
    }

    fn note(&self, text: &str) {
        let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new(text.to_owned())));
        self.send(SessionUpdate::AgentMessageChunk(chunk));
    }
}

/// Un tour commencé : sa session, son numéro et son jeton d'annulation.
struct Started {
    session: Arc<SessionState>,
    turn: usize,
    cancel: CancellationToken,
}

/// Joue un tour de la session puis répond au prompt.
async fn answer(
    engine: Arc<dyn Engine>,
    started: Started,
    req: PromptRequest,
    responder: Responder<PromptResponse>,
    cx: ConnectionTo<Client>,
) -> agent_client_protocol::Result<()> {
    let Started { session, turn, cancel } = started;
    let tools = match session.tools().await {
        Ok(tools) => tools,
        Err(error) => {
            tracing::warn!(error = %snafu::Report::from_error(&error), "connexion MCP de l'agent local impossible");
            return responder.respond_with_internal_error("the local agent cannot reach the tab's tools");
        }
    };
    let prompt = read_prompt(&req.prompt);
    let updates = AcpUpdates { cx, session: req.session_id, turn, title: Mutex::new(None) };
    let end = run_turn(engine.as_ref(), tools, &updates, &prompt, &session.summaries(), &cancel).await;
    let outcome = match end {
        TurnEnd::Reported => updates.title.lock().unwrap_or_else(PoisonError::into_inner).clone(),
        TurnEnd::Failed(kind) => Some(kind.key().to_owned()),
        TurnEnd::Cancelled => None,
    };
    if let Some(outcome) = outcome {
        session.push_summary(format!("{} → {outcome}", prompt.request));
    }
    let stop = if end == TurnEnd::Cancelled { StopReason::Cancelled } else { StopReason::EndTurn };
    responder.respond(PromptResponse::new(stop))
}

/// Sert ACP sur stdin/stdout jusqu'à la fermeture du tube.
pub(crate) async fn serve(engine: Arc<dyn Engine>) -> Result<(), LocalAgentError> {
    let sessions = Arc::new(Sessions::default());
    let (for_new, for_prompt, for_cancel) = (sessions.clone(), sessions.clone(), sessions);
    Agent
        .builder()
        .name("oku-slides-local-agent")
        .on_receive_request(
            async move |req: InitializeRequest, responder, _cx: ConnectionTo<Client>| {
                responder.respond(
                    InitializeResponse::new(req.protocol_version).agent_capabilities(
                        AgentCapabilities::new().mcp_capabilities(McpCapabilities::new().http(true)),
                    ),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: NewSessionRequest, responder, _cx: ConnectionTo<Client>| match http_access(
                &req.mcp_servers,
            ) {
                Some(access) => responder.respond(NewSessionResponse::new(for_new.open(access))),
                None => responder.respond_with_internal_error("no http mcp server"),
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: PromptRequest, responder, cx: ConnectionTo<Client>| {
                let Some(session) = for_prompt.get(req.session_id.0.as_ref()) else {
                    return responder.respond_with_internal_error("unknown session");
                };
                // Avant `spawn` : un `session/cancel` qui suit vise bien ce tour-ci.
                let (turn, cancel) = session.begin_turn();
                let started = Started { session, turn, cancel };
                cx.spawn(answer(engine.clone(), started, req, responder, cx.clone()))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |n: CancelNotification, _cx: ConnectionTo<Client>| {
                if let Some(session) = for_cancel.get(n.session_id.0.as_ref()) {
                    session.cancel.lock().unwrap_or_else(PoisonError::into_inner).cancel();
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
        .context(AcpSnafu)
        .map_err(Into::into)
}
