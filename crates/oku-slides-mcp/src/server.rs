//! Serveur HTTP streamable des outils : `127.0.0.1`, port libre, jeton exigé.
//! Ses connexions ont les bornes de `[serve]` (délai d'en-têtes, plafond),
//! comme le serveur de développement (voir `oku_slides_platform::serve_connections`).

use std::sync::Arc;
use std::time::Duration;

use oku_slides_platform::serve_connections;
use rust_mcp_axum::axum::extract::{Request, State};
use rust_mcp_axum::axum::http::StatusCode;
use rust_mcp_axum::axum::http::header::{AUTHORIZATION, CONNECTION};
use rust_mcp_axum::axum::middleware::{self, Next};
use rust_mcp_axum::axum::response::{IntoResponse, Response};
use rust_mcp_axum::{McpMountOptions, mcp_routes};
use rust_mcp_sdk::id_generator::{FastIdGenerator, UuidGenerator};
use rust_mcp_sdk::mcp_http::{McpAppState, McpHttpHandler};
use rust_mcp_sdk::mcp_server::ToMcpServerHandler;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::session_store::InMemorySessionStore;
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;

use crate::Tools;
use crate::error::{BindSnafu, McpError, TokenSnafu};

/// Grâce laissée aux connexions à l'arrêt : seuls des flux SSE restent alors
/// ouverts (l'agent est arrêté), qui ne finissent jamais d'eux-mêmes.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(1);

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
    /// Starts the server; it stops when `cancel` is cancelled.
    ///
    /// # Errors
    ///
    /// Fails if the token cannot be drawn at random or if the local port cannot be bound.
    pub async fn start(tools: Tools, cancel: CancellationToken) -> Result<Self, McpError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).context(TokenSnafu)?;
        let token = hex::encode(bytes);
        let limits = tools.limits;
        let state = Arc::new(McpAppState {
            session_store: Arc::new(InMemorySessionStore::new()),
            id_generator: Arc::new(UuidGenerator {}),
            stream_id_gen: Arc::new(FastIdGenerator::new(Some("s_"))),
            server_details: Arc::new(details()),
            handler: tools.to_mcp_server_handler(),
            ping_interval: Duration::from_secs(12),
            transport_options: Arc::default(),
            enable_json_response: false,
            event_store: None,
            task_store: None,
            client_task_store: None,
            message_observer: None,
        });
        // Seul le condensat de l'en-tête attendu reste en mémoire du middleware.
        let expected = digest(format!("Bearer {token}").as_bytes());
        let app = mcp_routes(state, &McpMountOptions::default(), McpHttpHandler::new(None, vec![], None))
            .layer(middleware::from_fn_with_state(expected, require_bearer));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.context(BindSnafu)?;
        let addr = listener.local_addr().context(BindSnafu)?;
        // Personne n'attend cette tâche : l'arrêt est borné par `SHUTDOWN_GRACE`.
        // Un flux SSE encore ouvert à l'arrêt est normal : son abandon n'est pas signalé.
        tokio::spawn(serve_connections(listener, app, limits, SHUTDOWN_GRACE, cancel.cancelled_owned()));
        tracing::info!(%addr, "outils de l'agent prêts");
        Ok(Self { url: format!("http://{addr}/mcp"), token })
    }

    /// Address of the MCP endpoint, on `127.0.0.1` and the port drawn at startup.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Valeur de l'en-tête `Authorization` ; ne jamais la journaliser.
    #[must_use]
    pub fn authorization(&self) -> String {
        format!("Bearer {}", self.token)
    }
}

/// Condensat SHA-256 d'une valeur d'en-tête `Authorization`.
type Digest32 = [u8; 32];

fn digest(bytes: &[u8]) -> Digest32 {
    Sha256::digest(bytes).into()
}

/// Vrai si `received` (octets bruts de l'en-tête `Authorization`) est exactement
/// l'en-tête attendu, dont on ne garde que le condensat.
///
/// La comparaison porte sur les condensats avec `==`, qui n'est pas en temps
/// constant : mais elle ne révèle qu'un préfixe du condensat attendu, inexploitable
/// tant que le jeton est aléatoire à haute entropie (32 octets de `getrandom`),
/// puisque deviner le jeton demande d'inverser SHA-256.
/// La propriété temporelle n'est pas testable ; le contrat (seul
/// l'en-tête exact passe) l'est, par HTTP.
fn token_matches(expected: &Digest32, received: &[u8]) -> bool {
    digest(received) == *expected
}

async fn require_bearer(State(expected): State<Digest32>, req: Request, next: Next) -> Response {
    let ok = req.headers().get(AUTHORIZATION).is_some_and(|v| token_matches(&expected, v.as_bytes()));
    if !ok {
        // Un client refusé ne garde pas de place (`[serve] max_connections`).
        return (StatusCode::UNAUTHORIZED, [(CONNECTION, "close")]).into_response();
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
