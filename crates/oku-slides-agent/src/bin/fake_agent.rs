//! Faux agent ACP : suit un scénario JSON (`--script <fichier>`), note chaque
//! message reçu (`--log <fichier>`, une ligne JSON par message) et peut
//! appeler les outils MCP que le client lui injecte.
//!
//! Scénario : `{ "http": true, "turns": [ { "plan": [{"content","status"}],
//! "tool_calls": [{"id","title","input","output","status","before"}], "mcp": [{"tool","args"}], "chunks": ["…"],
//! "wait_cancel": false, "crash": false, "delay_ms": 0, "error": null, "parallel": false } ],
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
//! (l'agent continue). `parallel` lance les appels MCP du tour en même temps
//! (questions parallèles) ; leurs résultats sont notés dans l'ordre du scénario. Le résultat de chaque appel MCP est noté
//! (`mcp/result` : `session`, `tool`, `out`), jamais raconté : comme un vrai agent, le texte du tour
//! n'est que ses `chunks` (le reçu de repli en prend la première phrase). `"hang_initialize": true` (au niveau du scénario) ne
//! répond jamais à `initialize` (démarrage bloqué) ; `init_delay_ms` en retarde la réponse. `load_session` / `resume`
//! (bool) annoncent et acceptent `session/load` / `session/resume` ;
//! `unknown_sessions` (bool) les fait échouer ; `replay` (liste de textes) est
//! rejoué en `agent_message_chunk` par `session/load` ; `by_prompt`
//! (`{ "<texte>": Turn }`) joue ce tour quand le prompt est exactement ce texte,
//! avant le choix par numéro, sans avancer la numérotation des autres tours.
//! Ces deux requêtes sont journalisées.
//! `new_session_delay_ms` fait attendre la réponse à `session/new` (ouverture lente),
//! `load_delay_ms` celle à `session/load`. `late_replay_ms` (agent non conforme) envoie
//! le rejeu de `session/load` après sa réponse, au bout de ce délai.
//! `models` (`[{ "id", "name" }]`) offers these models in a `model` select option of every
//! session reply (the first one in use); `session/set_config_option` is logged, switches the
//! session's model and answers its options, or fails for a value not offered.
//! `no_options_on_reopen` leaves the options out of the `session/load` and `session/resume` replies.

use std::collections::HashMap;
use std::io::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    LoadSessionRequest, LoadSessionResponse, McpCapabilities, McpServer, NewSessionRequest, NewSessionResponse, Plan,
    PlanEntry, PlanEntryPriority, PlanEntryStatus, PromptCapabilities, PromptRequest, PromptResponse,
    ResumeSessionRequest, ResumeSessionResponse, SessionCapabilities, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOption, SessionId, SessionNotification, SessionResumeCapabilities, SessionUpdate,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, StopReason, TextContent, ToolCall, ToolCallStatus,
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
#[expect(clippy::struct_excessive_bools, reason = "independent flags of the JSON scenario, read as they are")]
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
    /// Annonce `promptCapabilities.image` (blocs image dans un prompt).
    #[serde(default)]
    prompt_image: bool,
    /// Ne jamais répondre à `initialize` : un démarrage qui ne finit pas.
    #[serde(default)]
    hang_initialize: bool,
    /// Attente avant de répondre à `initialize`, en millisecondes (démarrage lent).
    #[serde(default)]
    init_delay_ms: Option<u64>,
    /// Annonce `session/load` et le sait faire.
    #[serde(default)]
    load_session: bool,
    /// Annonce `session/resume` et le sait faire.
    #[serde(default)]
    resume: bool,
    /// `session/load` et `session/resume` répondent par une erreur (session inconnue).
    #[serde(default)]
    unknown_sessions: bool,
    /// Textes rejoués (en `agent_message_chunk`) par `session/load`.
    #[serde(default)]
    replay: Vec<String>,
    /// Tour joué quand le texte du prompt est exactement cette clé (avant le choix par numéro).
    #[serde(default)]
    by_prompt: HashMap<String, Turn>,
    /// Attente avant de répondre à `session/new`, en millisecondes.
    #[serde(default)]
    new_session_delay_ms: u64,
    /// Attente avant de répondre à `session/load`, en millisecondes.
    #[serde(default)]
    load_delay_ms: u64,
    /// Rejeu envoyé après la réponse à `session/load`, au bout de ce délai (millisecondes).
    #[serde(default)]
    late_replay_ms: Option<u64>,
    /// Models offered in every session's `model` option; empty: no config option.
    #[serde(default)]
    models: Vec<FakeModel>,
    /// `session/load` and `session/resume` replies carry no config options (they are optional).
    #[serde(default)]
    no_options_on_reopen: bool,
}

/// A model the fake agent offers.
#[derive(Debug, Deserialize)]
struct FakeModel {
    id: String,
    name: String,
}

impl Script {
    /// The scenario when `--script` is unreadable: an empty turn, HTTP MCP announced.
    fn fallback() -> Self {
        Script {
            http: true,
            turns: vec![Turn::default()],
            crash_on_session: None,
            sessions: HashMap::new(),
            prompt_image: false,
            hang_initialize: false,
            init_delay_ms: None,
            load_session: false,
            resume: false,
            unknown_sessions: false,
            replay: Vec::new(),
            by_prompt: HashMap::new(),
            new_session_delay_ms: 0,
            load_delay_ms: 0,
            late_replay_ms: None,
            models: Vec::new(),
            no_options_on_reopen: false,
        }
    }
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
    /// Lancer les appels MCP du tour en même temps (des questions parallèles), au lieu de l'un après l'autre.
    #[serde(default)]
    parallel: bool,
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
    /// Arguments envoyés en `raw_input`.
    #[serde(default)]
    input: Option<Value>,
    /// Résultat envoyé en `raw_output`.
    #[serde(default)]
    output: Option<Value>,
    /// `failed` : l'appel échoue ; sinon il réussit.
    #[serde(default)]
    status: Option<String>,
    /// Texte de l'agent envoyé juste avant l'appel (il devient une note de la timeline).
    #[serde(default)]
    before: Option<String>,
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
    /// The model each session uses, once changed by `session/set_config_option`.
    model: Mutex<HashMap<String, String>>,
}

impl State {
    /// The initial state, read from the command line.
    ///
    /// The scenario from `--script` (otherwise [`Script::fallback`]) and the log from `--log`.
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
        let script: Script = flag("--script")
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Script::fallback);
        let log = flag("--log")
            .and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok())
            .map(Mutex::new);
        Self {
            script,
            log,
            sessions: AtomicUsize::new(0),
            mcp: Mutex::new(HashMap::new()),
            turns: Mutex::new(HashMap::new()),
            wait_cancel: Mutex::new(HashMap::new()),
            model: Mutex::new(HashMap::new()),
        }
    }

    /// Records the `method` reopening of `session`, which keeps its MCP server.
    ///
    /// `method` is `session/load` or `session/resume`, with request `value`. False if the
    /// scenario refuses unknown sessions.
    fn reopen(&self, method: &str, value: &Value, session: &SessionId, mcp: &[McpServer]) -> bool {
        self.log(method, value);
        if self.script.unknown_sessions {
            return false;
        }
        self.remember_mcp(&session.0, mcp);
        true
    }

    /// Replays the scenario's `replay` texts into `session`, as `agent_message_chunk`.
    fn replay(&self, cx: &ConnectionTo<Client>, session: &SessionId) -> agent_client_protocol::Result<()> {
        for text in &self.script.replay {
            cx.send_notification(chunk(session, text.clone()))?;
        }
        Ok(())
    }

    /// Answers `initialize`: the scenario's capabilities, after its optional delay.
    async fn initialize(&self, req: InitializeRequest) -> InitializeResponse {
        self.log("initialize", &serde_json::to_value(&req).unwrap_or_default());
        self.log("pid", &serde_json::json!(std::process::id()));
        if self.script.hang_initialize {
            std::future::pending::<()>().await;
        }
        if let Some(ms) = self.script.init_delay_ms {
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }
        let sessions = if self.script.resume {
            SessionCapabilities::new().resume(SessionResumeCapabilities::new())
        } else {
            SessionCapabilities::new()
        };
        InitializeResponse::new(req.protocol_version).agent_capabilities(
            AgentCapabilities::new()
                .mcp_capabilities(McpCapabilities::new().http(self.script.http))
                .load_session(self.script.load_session)
                .prompt_capabilities(PromptCapabilities::new().image(self.script.prompt_image))
                .session_capabilities(sessions),
        )
    }

    /// The config options of `session`: its `model` select, `None` without scenario models.
    fn config_options(&self, session: &str) -> Option<Vec<SessionConfigOption>> {
        let first = self.script.models.first()?;
        let model = self.model.lock().unwrap_or_else(PoisonError::into_inner);
        let current = model.get(session).cloned().unwrap_or_else(|| first.id.clone());
        let values: Vec<_> =
            self.script.models.iter().map(|m| SessionConfigSelectOption::new(m.id.clone(), m.name.clone())).collect();
        let option =
            SessionConfigOption::select("model", "Model", current, values).category(SessionConfigOptionCategory::Model);
        Some(vec![option])
    }

    /// The config options of a reopened `session`; none if the scenario leaves them out.
    fn reopen_options(&self, session: &str) -> Option<Vec<SessionConfigOption>> {
        if self.script.no_options_on_reopen { None } else { self.config_options(session) }
    }

    /// Answers `session/set_config_option`: the session's new options, `None` for an unknown value.
    fn set_config_option(&self, req: &SetSessionConfigOptionRequest) -> Option<Vec<SessionConfigOption>> {
        self.log("session/set_config_option", &serde_json::to_value(req).unwrap_or_default());
        let value = req.value.as_value_id()?.0.to_string();
        if req.config_id.0.as_ref() != "model" || !self.script.models.iter().any(|m| m.id == value) {
            return None;
        }
        let session = req.session_id.0.to_string();
        self.model.lock().unwrap_or_else(PoisonError::into_inner).insert(session.clone(), value);
        self.config_options(&session)
    }

    /// Answers `session/new`: a numbered `sess-<n>` session, which keeps its MCP server.
    async fn new_session(&self, req: &NewSessionRequest) -> NewSessionResponse {
        self.log("session/new", &serde_json::to_value(req).unwrap_or_default());
        if self.script.new_session_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.script.new_session_delay_ms)).await;
        }
        let n = self.sessions.fetch_add(1, Ordering::Relaxed) + 1;
        let id = format!("sess-{n}");
        self.remember_mcp(&id, &req.mcp_servers);
        let options = self.config_options(&id);
        NewSessionResponse::new(id).config_options(options)
    }

    /// The turn that prompt `req` plays.
    ///
    /// Picked by its text (`by_prompt`), otherwise by its rank in the session. Exits the
    /// process if the scenario crashes this session.
    fn pick_turn(&self, req: &PromptRequest) -> Turn {
        let session_key = req.session_id.0.to_string();
        let said = req.prompt.iter().rev().find_map(|b| match b {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        });
        let by_prompt = said.and_then(|t| self.script.by_prompt.get(t)).cloned();
        // A turn picked by its text stays out of the other turns' numbering.
        let index_in_session = {
            let mut turns = self.turns.lock().unwrap_or_else(PoisonError::into_inner);
            let count = turns.entry(session_key.clone()).or_insert(0);
            let i = *count;
            if by_prompt.is_none() {
                *count += 1;
            }
            i
        };
        let rank = State::session_index(&session_key);
        if index_in_session == 0 && self.script.crash_on_session == Some(rank) {
            std::process::exit(3);
        }
        let turns_script = self.script.sessions.get(&rank.to_string()).unwrap_or(&self.script.turns);
        match by_prompt {
            Some(t) => t,
            None => match turns_script.len() {
                0 => Turn::default(),
                n => turns_script[index_in_session % n].clone(),
            },
        }
    }

    fn log(&self, method: &str, params: &Value) {
        if let Some(file) = &self.log {
            let mut f = file.lock().unwrap_or_else(PoisonError::into_inner);
            let _ = writeln!(f, "{}", serde_json::json!({ "method": method, "params": params }));
        }
    }

    /// Le résultat d'un appel d'outil MCP du script (`mcp/result`).
    fn log_result(&self, session: &str, step: &McpStep, out: &str) {
        self.log("mcp/result", &serde_json::json!({ "session": session, "tool": step.tool, "out": out }));
    }

    /// Notify de la session, créée au besoin.
    fn notify_for(&self, session: &str) -> Arc<Notify> {
        let mut map = self.wait_cancel.lock().unwrap_or_else(PoisonError::into_inner);
        map.entry(session.to_owned()).or_insert_with(|| Arc::new(Notify::new())).clone()
    }

    /// Range le serveur MCP HTTP de `session` (URL et `Authorization`).
    fn remember_mcp(&self, session: &str, servers: &[McpServer]) {
        let http = servers.iter().find_map(|s| match s {
            McpServer::Http(h) => {
                let auth = h.headers.iter().find(|x| x.name == "Authorization").map(|x| x.value.clone())?;
                Some((h.url.clone(), auth))
            }
            _ => None,
        });
        if let Some(http) = http {
            self.mcp.lock().unwrap_or_else(PoisonError::into_inner).insert(session.to_owned(), http);
        }
    }

    /// Numéro de la n-ième session créée (`sess-{n}` -> `n`), ou `0` si le format est inattendu.
    fn session_index(session: &str) -> usize {
        session.strip_prefix("sess-").and_then(|n| n.parse().ok()).unwrap_or(0)
    }
}

struct Noop;
#[async_trait]
impl ClientHandler for Noop {}

/// Un appel d'outil MCP du script, sur le serveur de la session (`mcp` : URL et `Authorization`) s'il y en a un.
async fn run_step(mcp: Option<&(String, String)>, step: &McpStep) -> String {
    match mcp {
        Some((url, auth)) => call_tool(url, auth, &step.tool, &step.args).await,
        None => "no mcp server".to_owned(),
    }
}

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
        if let Some(text) = &t.before {
            let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new(text.clone())));
            cx.send_notification(SessionNotification::new(session.clone(), SessionUpdate::AgentMessageChunk(chunk)))?;
        }
        let status =
            if t.status.as_deref() == Some("failed") { ToolCallStatus::Failed } else { ToolCallStatus::Completed };
        let call = ToolCall::new(t.id.clone(), t.title.clone())
            .status(status)
            .raw_input(t.input.clone())
            .raw_output(t.output.clone());
        cx.send_notification(SessionNotification::new(session.clone(), SessionUpdate::ToolCall(call)))?;
    }
    if turn.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(turn.delay_ms)).await;
    }
    let session_key = session.0.to_string();
    let mcp = state.mcp.lock().unwrap_or_else(PoisonError::into_inner).get(&session_key).cloned();
    // En séquence, chaque résultat est journalisé avant l'appel suivant : un appel qui attend
    // l'utilisateur (une question) ne retient pas le résultat des précédents.
    if turn.parallel {
        let calls: Vec<_> = turn
            .mcp
            .iter()
            .map(|step| {
                // `spawn` veut des valeurs possédées : chaque tâche a sa copie.
                let (mcp, step) = (mcp.clone(), step.clone());
                tokio::spawn(async move { run_step(mcp.as_ref(), &step).await })
            })
            .collect();
        for (step, call) in turn.mcp.iter().zip(calls) {
            let out = call.await.unwrap_or_else(|e| format!("mcp call failed: {e}"));
            state.log_result(&session_key, step, &out);
        }
    } else {
        for step in &turn.mcp {
            let out = run_step(mcp.as_ref(), step).await;
            state.log_result(&session_key, step, &out);
        }
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
    let state = Arc::new(State::from_args());
    let (s1, s2, s3, s4) = (state.clone(), state.clone(), state.clone(), state.clone());
    let (s5, s6, s7) = (state.clone(), state.clone(), state.clone());
    Agent
        .builder()
        .name("oku-slides-fake-agent")
        .on_receive_request(
            async move |req: InitializeRequest, responder, _cx: ConnectionTo<Client>| {
                responder.respond(s1.initialize(req).await)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: NewSessionRequest, responder, _cx: ConnectionTo<Client>| {
                responder.respond(s2.new_session(&req).await)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: PromptRequest, responder, cx: ConnectionTo<Client>| {
                s3.log("session/prompt", &serde_json::to_value(&req).unwrap_or_default());
                let turn = s3.pick_turn(&req);
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
        .on_receive_request(
            async move |req: LoadSessionRequest, responder, cx: ConnectionTo<Client>| {
                let value = serde_json::to_value(&req).unwrap_or_default();
                if !s5.reopen("session/load", &value, &req.session_id, &req.mcp_servers) {
                    return responder.respond_with_internal_error("unknown session");
                }
                if s5.script.load_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(s5.script.load_delay_ms)).await;
                }
                let loaded = LoadSessionResponse::new().config_options(s5.reopen_options(&req.session_id.0));
                let Some(late) = s5.script.late_replay_ms else {
                    s5.replay(&cx, &req.session_id)?;
                    return responder.respond(loaded);
                };
                responder.respond(loaded)?;
                let (state, session) = (s5.clone(), req.session_id.clone());
                cx.spawn({
                    let cx = cx.clone();
                    async move {
                        tokio::time::sleep(Duration::from_millis(late)).await;
                        state.replay(&cx, &session)
                    }
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: ResumeSessionRequest, responder, _cx: ConnectionTo<Client>| {
                let value = serde_json::to_value(&req).unwrap_or_default();
                if !s6.reopen("session/resume", &value, &req.session_id, &req.mcp_servers) {
                    return responder.respond_with_internal_error("unknown session");
                }
                responder.respond(ResumeSessionResponse::new().config_options(s6.reopen_options(&req.session_id.0)))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: SetSessionConfigOptionRequest, responder, _cx: ConnectionTo<Client>| match s7
                .set_config_option(&req)
            {
                Some(options) => responder.respond(SetSessionConfigOptionResponse::new(options)),
                None => responder.respond_with_internal_error("unknown config value"),
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |n: CancelNotification, _cx: ConnectionTo<Client>| {
                s4.log("session/cancel", &serde_json::to_value(&n).unwrap_or_default());
                s4.notify_for(n.session_id.0.as_ref()).notify_one();
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}
