//! A small MCP client for the fake agent and tests: JSON-RPC over HTTP on one shared client.
//!
//! The rust-mcp-sdk client builds a `reqwest` client per transport and per request; with the system
//! roots unified in, each build reads the macOS keychain (~360 ms, t-1796). [`McpCaller`] speaks
//! only to the local tool server over plain HTTP and reuses one client. The app never uses it.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use rust_mcp_sdk::schema::{CallToolResult, ListToolsResult, ProtocolVersion};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};
use snafu::{Backtrace, OptionExt, ResultExt, Snafu, ensure};

/// The one HTTP client every caller shares: built once, never per call.
static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

const SESSION_HEADER: &str = "mcp-session-id";
const VERSION_HEADER: &str = "mcp-protocol-version";

/// A call to the tool server failed; its display says which request and why.
#[derive(Debug, Snafu)]
pub struct CallerError(CallerInner);

#[derive(Debug, Snafu)]
enum CallerInner {
    #[snafu(display("mcp http client could not be built: {source}"))]
    Client { source: reqwest::Error, backtrace: Backtrace },
    #[snafu(display("mcp request {method} failed: {source}"))]
    Http { method: String, source: reqwest::Error, backtrace: Backtrace },
    #[snafu(display("mcp request {method} answered http {status}"))]
    Status { method: String, status: u16, backtrace: Backtrace },
    #[snafu(display("mcp request {method} got no response"))]
    NoResponse { method: String, backtrace: Backtrace },
    #[snafu(display("mcp request {method} refused: {code} {message}"))]
    Rpc { method: String, code: i64, message: String, backtrace: Backtrace },
    #[snafu(display("mcp request {method} returned an unexpected result: {source}"))]
    Shape { method: String, source: serde_json::Error, backtrace: Backtrace },
}

/// A JSON-RPC error object, as the server sends it.
#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

/// An initialized MCP session on a tool server.
pub struct McpCaller {
    http: reqwest::Client,
    url: String,
    authorization: Option<String>,
    session: Option<String>,
    next_id: AtomicU64,
}

impl std::fmt::Debug for McpCaller {
    /// The authorization and the session id never appear.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpCaller").field("url", &self.url).finish_non_exhaustive()
    }
}

/// The shared client, built on first use; loopback needs no proxy (`system-proxy` is unified in).
fn shared_client() -> Result<reqwest::Client, CallerInner> {
    if let Some(client) = HTTP.get() {
        return Ok(client.clone());
    }
    let client = reqwest::Client::builder().no_proxy().build().context(ClientSnafu)?;
    Ok(HTTP.get_or_init(|| client).clone())
}

impl McpCaller {
    /// Opens a session on the server at `url`, sending `authorization` as the `Authorization` header.
    ///
    /// # Errors
    ///
    /// Fails if the server is unreachable, refuses the header or the initialization.
    pub async fn connect(url: &str, authorization: Option<String>) -> Result<Self, CallerError> {
        let http = shared_client()?;
        let mut caller = Self { http, url: url.to_owned(), authorization, session: None, next_id: AtomicU64::new(1) };
        let params = json!({
            "protocolVersion": ProtocolVersion::V2025_11_25.to_string(),
            "capabilities": {},
            "clientInfo": { "name": "oku-slides-caller", "version": env!("CARGO_PKG_VERSION") },
        });
        let (session, _) = caller.request("initialize", params).await?;
        caller.session = session;
        caller.notify("notifications/initialized").await?;
        Ok(caller)
    }

    /// Calls the tool `name` with `args`; a tool's own failure is an `is_error` result.
    ///
    /// # Errors
    ///
    /// Fails if the request fails or the server refuses it (unknown session, bad JSON-RPC).
    pub async fn call_tool(&self, name: &str, args: Map<String, Value>) -> Result<CallToolResult, CallerError> {
        Ok(self.typed("tools/call", json!({ "name": name, "arguments": args })).await?)
    }

    /// The tools the server lists: the first page only, which holds them all today.
    ///
    /// # Errors
    ///
    /// Fails if the request fails or the server refuses it.
    pub async fn list_tools(&self) -> Result<ListToolsResult, CallerError> {
        Ok(self.typed("tools/list", json!({})).await?)
    }

    /// Ends the session on the server.
    ///
    /// # Errors
    ///
    /// Fails if the request fails or the server answers with an error status.
    pub async fn close(self) -> Result<(), CallerError> {
        let method = "DELETE";
        let reply = self.headers(self.http.delete(&self.url)).send().await.context(HttpSnafu { method })?;
        ensure!(reply.status().is_success(), StatusSnafu { method, status: reply.status().as_u16() });
        Ok(())
    }

    async fn typed<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T, CallerInner> {
        let (_, result) = self.request(method, params).await?;
        serde_json::from_value(result).context(ShapeSnafu { method })
    }

    /// Sends one JSON-RPC request; returns the session header and the response's `result`.
    async fn request(&self, method: &str, params: Value) -> Result<(Option<String>, Value), CallerInner> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let reply = self.post(method, &body).await?;
        let session = reply.headers().get(SESSION_HEADER).and_then(|v| v.to_str().ok()).map(str::to_owned);
        let sse = reply
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|t| t.starts_with("text/event-stream"));
        let text = reply.text().await.context(HttpSnafu { method })?;
        let response = if sse { sse_response(&text, id) } else { json_response(&text, id) };
        let mut response = response.context(NoResponseSnafu { method })?;
        if let Some(error) = response.get_mut("error").map(Value::take) {
            let RpcError { code, message } = serde_json::from_value(error).context(ShapeSnafu { method })?;
            return RpcSnafu { method, code, message }.fail();
        }
        let result = response.get_mut("result").map(Value::take).context(NoResponseSnafu { method })?;
        Ok((session, result))
    }

    async fn notify(&self, method: &str) -> Result<(), CallerInner> {
        self.post(method, &json!({ "jsonrpc": "2.0", "method": method })).await.map(drop)
    }

    async fn post(&self, method: &str, body: &Value) -> Result<reqwest::Response, CallerInner> {
        let request = self
            .http
            .post(&self.url)
            .header(reqwest::header::ACCEPT, "application/json, text/event-stream")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string());
        let reply = self.headers(request).send().await.context(HttpSnafu { method })?;
        ensure!(reply.status().is_success(), StatusSnafu { method, status: reply.status().as_u16() });
        Ok(reply)
    }

    fn headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(a) = &self.authorization {
            request = request.header(reqwest::header::AUTHORIZATION, a);
        }
        if let Some(s) = &self.session {
            request =
                request.header(SESSION_HEADER, s).header(VERSION_HEADER, ProtocolVersion::V2025_11_25.to_string());
        }
        request
    }
}

/// The JSON-RPC response with `id` in a plain JSON body (one message or a batch).
fn json_response(text: &str, id: u64) -> Option<Value> {
    let value: Value = serde_json::from_str(text).ok()?;
    match value {
        Value::Array(items) => items.into_iter().find(|m| is_response(m, id)),
        message => is_response(&message, id).then_some(message),
    }
}

/// The JSON-RPC response with `id` among the events of an SSE body; other messages are skipped.
fn sse_response(text: &str, id: u64) -> Option<Value> {
    text.replace("\r\n", "\n").split("\n\n").find_map(|event| {
        let data: Vec<&str> =
            event.lines().filter_map(|l| l.strip_prefix("data:")).map(|d| d.strip_prefix(' ').unwrap_or(d)).collect();
        if data.is_empty() {
            return None;
        }
        json_response(&data.join("\n"), id)
    })
}

fn is_response(message: &Value, id: u64) -> bool {
    message.get("id").and_then(Value::as_u64) == Some(id)
        && (message.get("result").is_some() || message.get("error").is_some())
}

#[cfg(test)]
#[path = "caller_tests.rs"]
mod caller_tests;
