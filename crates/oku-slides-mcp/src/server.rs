//! Serveur HTTP streamable des outils : `127.0.0.1`, port libre, jeton exigé.

use std::sync::Arc;
use std::time::Duration;

use rust_mcp_axum::axum::extract::{Request, State};
use rust_mcp_axum::axum::http::StatusCode;
use rust_mcp_axum::axum::http::header::AUTHORIZATION;
use rust_mcp_axum::axum::middleware::{self, Next};
use rust_mcp_axum::axum::response::{IntoResponse, Response};
use rust_mcp_axum::{McpMountOptions, axum, mcp_routes};
use rust_mcp_sdk::id_generator::{FastIdGenerator, UuidGenerator};
use rust_mcp_sdk::mcp_http::{McpAppState, McpHttpHandler};
use rust_mcp_sdk::mcp_server::ToMcpServerHandler;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::session_store::InMemorySessionStore;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;

use crate::Tools;
use crate::error::{BindSnafu, McpError, TokenSnafu};

/// Le serveur en marche : son adresse et son jeton.
pub struct ToolServer {
    url: String,
    token: String,
}

impl std::fmt::Debug for ToolServer {
    /// Le jeton n'apparaît jamais.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolServer").field("url", &self.url).finish_non_exhaustive()
    }
}

impl ToolServer {
    /// Démarre le serveur ; il s'arrête quand `cancel` est annulé.
    pub async fn start(tools: Tools, cancel: CancellationToken) -> Result<Self, McpError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).context(TokenSnafu)?;
        let token = hex::encode(bytes);
        let state = Arc::new(McpAppState {
            session_store: Arc::new(InMemorySessionStore::new()),
            id_generator: Arc::new(UuidGenerator {}),
            stream_id_gen: Arc::new(FastIdGenerator::new(Some("s_"))),
            server_details: Arc::new(details()),
            handler: tools.to_mcp_server_handler(),
            ping_interval: Duration::from_secs(12),
            transport_options: Default::default(),
            enable_json_response: false,
            event_store: None,
            task_store: None,
            client_task_store: None,
            message_observer: None,
        });
        let expected: Arc<str> = format!("Bearer {token}").into();
        let app = mcp_routes(state, &McpMountOptions::default(), McpHttpHandler::new(None, vec![], None))
            .layer(middleware::from_fn_with_state(expected, require_bearer));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.context(BindSnafu)?;
        let addr = listener.local_addr().context(BindSnafu)?;
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).with_graceful_shutdown(cancel.cancelled_owned()).await {
                tracing::warn!(error = %e, "serveur d'outils arrêté");
            }
        });
        tracing::info!(%addr, "outils de l'agent prêts");
        Ok(Self { url: format!("http://{addr}/mcp"), token })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Valeur de l'en-tête `Authorization` ; ne jamais la journaliser.
    pub fn authorization(&self) -> String {
        format!("Bearer {}", self.token)
    }
}

async fn require_bearer(State(expected): State<Arc<str>>, req: Request, next: Next) -> Response {
    let ok = req.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok()) == Some(&*expected);
    if !ok {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(req).await
}

fn details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "oku-slides".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Oku Slides".into()),
            description: None,
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: None,
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    }
}
