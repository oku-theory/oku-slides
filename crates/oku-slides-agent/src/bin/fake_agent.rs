//! Faux agent ACP : suit un scénario JSON (`--script <fichier>`), note chaque
//! message reçu (`--log <fichier>`, une ligne JSON par message) et peut
//! appeler les outils MCP que le client lui injecte.
//!
//! Scénario : `{ "http": true, "turns": [ { "plan": [{"content","status"}],
//! "tool_calls": [{"id","title"}], "mcp": [{"tool","args"}], "chunks": ["…"],
//! "wait_cancel": false, "crash": false, "delay_ms": 0, "error": null } ],
//! "sessions": { "<n>": [ <turns> ] }, "crash_on_session": null }` ; le
//! n-ième prompt d'une session joue le tour n de cette session (modulo le
//! nombre de tours de `sessions.<n>` si présent pour son numéro, sinon de
//! `turns`). `<n>` est le numéro de session (même numérotation que
//! `crash_on_session`, 1 = la première créée), en clé JSON (une chaîne).
//! Chaque session garde par elle-même : son serveur MCP (reçu à son
//! `session/new`), son compteur de tours et son annulation (`session/cancel`
//! ne réveille que cette session). `delay_ms` fait attendre un tour avant sa
//! réponse ; `crash_on_session` (1 = la première session créée) fait quitter
//! le processus au premier prompt de la n-ième session, comme `crash`.
//! `error` fait répondre au prompt par une erreur JSON-RPC avec ce message
//! (l'agent continue). Le résultat de chaque appel MCP est noté
//! (`mcp/result` : `session`, `tool`, `out`). `"hang_initialize": true` (au niveau du scénario) ne
//! répond jamais à `initialize` (démarrage bloqué).

use std::collections::HashMap;
use std::io::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    McpCapabilities, McpServer, NewSessionRequest, NewSessionResponse, Plan, PlanEntry, PlanEntryPriority,
    PlanEntryStatus, PromptRequest, PromptResponse, SessionId, SessionNotification, SessionUpdate, StopReason,
    TextContent, ToolCall, ToolCallStatus,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Stdio};
use async_trait::async_trait;
use rust_mcp_sdk::mcp_client::{ClientHandler, client_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams, ProtocolVersion,
};
use rust_mcp_sdk::{McpClient, RequestOptions, StreamableTransportOptions};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Notify;

#[derive(Debug, Deserialize)]
struct Script {
    #[serde(default = "yes")]
    http: bool,
    #[serde(default)]
    turns: Vec<Turn>,
    /// Numéro (1 = la première créée) de la session dont le premier prompt fait crasher le processus.
    #[serde(default)]
    crash_on_session: Option<usize>,
    /// Scénario propre à une session (clé : son numéro, comme `crash_on_session`, en chaîne) ;
    /// remplace `turns` pour elle si présent.
    #[serde(default)]
    sessions: HashMap<String, Vec<Turn>>,
    /// Ne jamais répondre à `initialize` : un démarrage qui ne finit pas.
    #[serde(default)]
    hang_initialize: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Turn {
    #[serde(default)]
    plan: Vec<Step>,
    #[serde(default)]
    tool_calls: Vec<ToolStep>,
    #[serde(default)]
    mcp: Vec<McpStep>,
    #[serde(default)]
    chunks: Vec<String>,
    #[serde(default)]
    wait_cancel: bool,
    #[serde(default)]
    crash: bool,
    /// Délai avant de répondre au tour, après le plan et avant les appels MCP.
    #[serde(default)]
    delay_ms: u64,
    /// Répondre au prompt par une erreur JSON-RPC portant ce message.
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Step {
    content: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ToolStep {
    id: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct McpStep {
    tool: String,
    #[serde(default)]
    args: Value,
}

struct State {
    script: Script,
    log: Option<Mutex<std::fs::File>>,
    /// Nombre de sessions créées jusqu'ici (numérotation `sess-{n}`, 1 = la première).
    sessions: AtomicUsize,
    /// URL et en-tête `Authorization` du serveur MCP de chaque session, par `SessionId`.
    mcp: Mutex<HashMap<String, (String, String)>>,
    /// Compteur de tours joués, par session.
    turns: Mutex<HashMap<String, usize>>,
    /// Notification d'annulation par session ; `session/cancel` ne réveille que la sienne.
    wait_cancel: Mutex<HashMap<String, Arc<Notify>>>,
}

impl State {
    fn log(&self, method: &str, params: Value) {
        if let Some(file) = &self.log
            && let Ok(mut f) = file.lock()
        {
            let _ = writeln!(f, "{}", serde_json::json!({ "method": method, "params": params }));
        }
    }

    /// Notify de la session, créée au besoin.
    fn notify_for(&self, session: &str) -> Arc<Notify> {
        let Ok(mut map) = self.wait_cancel.lock() else { return Arc::new(Notify::new()) };
        map.entry(session.to_owned()).or_insert_with(|| Arc::new(Notify::new())).clone()
    }

    /// Numéro de la n-ième session créée (`sess-{n}` -> `n`), ou `0` si le format est inattendu.
    fn session_index(session: &str) -> usize {
        session.strip_prefix("sess-").and_then(|n| n.parse().ok()).unwrap_or(0)
    }
}

struct Noop;
#[async_trait]
impl ClientHandler for Noop {}

/// Appelle un outil MCP et renvoie son résultat en JSON (texte).
async fn call_tool(url: &str, auth: &str, tool: &str, args: &Value) -> String {
    let transport = StreamableTransportOptions {
        mcp_url: url.to_owned(),
        request_options: RequestOptions {
            custom_headers: Some(HashMap::from([("Authorization".to_owned(), auth.to_owned())])),
            ..RequestOptions::default()
        },
    };
    let details = InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "oku-slides-fake-agent".into(),
            version: "0.1.0".into(),
            title: None,
            description: None,
            icons: vec![],
            website_url: None,
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        meta: None,
    };
    let client = client_runtime::with_transport_options(details, transport, Noop, None, None, None);
    if let Err(e) = client.clone().start().await {
        return format!("mcp start failed: {e}");
    }
    let params = CallToolRequestParams::new(tool).with_arguments(args.as_object().cloned().unwrap_or_default());
    let out = match client.request_tool_call(params).await {
        Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
        Err(e) => format!("mcp call failed: {e}"),
    };
    let _ = client.shut_down().await;
    out
}

fn chunk(session: &SessionId, text: String) -> SessionNotification {
    SessionNotification::new(
        session.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(text)))),
    )
}

async fn play(
    state: &State,
    cx: &ConnectionTo<Client>,
    session: &SessionId,
    turn: Turn,
) -> agent_client_protocol::Result<StopReason> {
    if !turn.plan.is_empty() {
        let entries = turn
            .plan
            .iter()
            .map(|s| {
                let status = match s.status.as_str() {
                    "completed" => PlanEntryStatus::Completed,
                    "in_progress" => PlanEntryStatus::InProgress,
                    _ => PlanEntryStatus::Pending,
                };
                PlanEntry::new(s.content.clone(), PlanEntryPriority::Medium, status)
            })
            .collect();
        cx.send_notification(SessionNotification::new(session.clone(), SessionUpdate::Plan(Plan::new(entries))))?;
    }
    for t in &turn.tool_calls {
        let call = ToolCall::new(t.id.clone(), t.title.clone()).status(ToolCallStatus::Completed);
        cx.send_notification(SessionNotification::new(session.clone(), SessionUpdate::ToolCall(call)))?;
    }
    if turn.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(turn.delay_ms)).await;
    }
    let session_key = session.0.to_string();
    let mcp = state.mcp.lock().ok().and_then(|m| m.get(&session_key).cloned());
    for step in &turn.mcp {
        let out = match &mcp {
            Some((url, auth)) => call_tool(url, auth, &step.tool, &step.args).await,
            None => "no mcp server".to_owned(),
        };
        state.log("mcp/result", serde_json::json!({ "session": session_key, "tool": step.tool, "out": out }));
        cx.send_notification(chunk(session, format!("tool:{} {out}", step.tool)))?;
    }
    for text in &turn.chunks {
        cx.send_notification(chunk(session, text.clone()))?;
    }
    if turn.wait_cancel {
        state.notify_for(&session_key).notified().await;
        return Ok(StopReason::Cancelled);
    }
    Ok(StopReason::EndTurn)
}

#[tokio::main]
async fn main() -> agent_client_protocol::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let script: Script = flag("--script")
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Script {
            http: true,
            turns: vec![Turn::default()],
            crash_on_session: None,
            sessions: HashMap::new(),
            hang_initialize: false,
        });
    let log =
        flag("--log").and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok()).map(Mutex::new);
    let state = Arc::new(State {
        script,
        log,
        sessions: AtomicUsize::new(0),
        mcp: Mutex::new(HashMap::new()),
        turns: Mutex::new(HashMap::new()),
        wait_cancel: Mutex::new(HashMap::new()),
    });
    let (s1, s2, s3, s4) = (state.clone(), state.clone(), state.clone(), state.clone());
    Agent
        .builder()
        .name("oku-slides-fake-agent")
        .on_receive_request(
            async move |req: InitializeRequest, responder, _cx: ConnectionTo<Client>| {
                s1.log("initialize", serde_json::to_value(&req).unwrap_or_default());
                s1.log("pid", serde_json::json!(std::process::id()));
                if s1.script.hang_initialize {
                    std::future::pending::<()>().await;
                }
                responder.respond(InitializeResponse::new(req.protocol_version).agent_capabilities(
                    AgentCapabilities::new().mcp_capabilities(McpCapabilities::new().http(s1.script.http)),
                ))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: NewSessionRequest, responder, _cx: ConnectionTo<Client>| {
                s2.log("session/new", serde_json::to_value(&req).unwrap_or_default());
                let http = req.mcp_servers.iter().find_map(|s| match s {
                    McpServer::Http(h) => {
                        let auth = h.headers.iter().find(|x| x.name == "Authorization").map(|x| x.value.clone())?;
                        Some((h.url.clone(), auth))
                    }
                    _ => None,
                });
                let n = s2.sessions.fetch_add(1, Ordering::Relaxed) + 1;
                let id = format!("sess-{n}");
                if let (Some(http), Ok(mut m)) = (http, s2.mcp.lock()) {
                    m.insert(id.clone(), http);
                }
                responder.respond(NewSessionResponse::new(id))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: PromptRequest, responder, cx: ConnectionTo<Client>| {
                s3.log("session/prompt", serde_json::to_value(&req).unwrap_or_default());
                let session_key = req.session_id.0.to_string();
                let index_in_session = if let Ok(mut turns) = s3.turns.lock() {
                    let count = turns.entry(session_key.clone()).or_insert(0);
                    let i = *count;
                    *count += 1;
                    i
                } else {
                    0
                };
                let rank = State::session_index(&session_key);
                if index_in_session == 0 && s3.script.crash_on_session == Some(rank) {
                    std::process::exit(3);
                }
                let turns_script = s3.script.sessions.get(&rank.to_string()).unwrap_or(&s3.script.turns);
                let turn = match turns_script.len() {
                    0 => Turn::default(),
                    n => turns_script[index_in_session % n].clone(),
                };
                if let Some(message) = &turn.error {
                    return responder.respond_with_internal_error(message);
                }
                if turn.crash && turn.mcp.is_empty() {
                    std::process::exit(3);
                }
                let state = s3.clone();
                cx.spawn({
                    let cx = cx.clone();
                    async move {
                        let stop = play(&state, &cx, &req.session_id, turn.clone()).await?;
                        if turn.crash {
                            std::process::exit(3);
                        }
                        responder.respond(PromptResponse::new(stop))
                    }
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |n: CancelNotification, _cx: ConnectionTo<Client>| {
                s4.log("session/cancel", serde_json::to_value(&n).unwrap_or_default());
                s4.notify_for(n.session_id.0.as_ref()).notify_one();
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}
