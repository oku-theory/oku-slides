//! Client MCP d'une session : un seul `client_runtime` vers le serveur d'outils de l'onglet.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use rust_mcp_sdk::mcp_client::{ClientHandler, ClientRuntime, client_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, ClientCapabilities, ContentBlock, Implementation, InitializeRequestParams,
    PaginatedRequestParams, ProtocolVersion,
};
use rust_mcp_sdk::{McpClient, RequestOptions, StreamableTransportOptions};
use serde_json::Value;
use snafu::ResultExt;

use crate::compact::ToolSpec;
use crate::sessions::McpAccess;
use crate::tool_error::{SchemaSnafu, ToolError};
use crate::turn::ToolPort;

/// Le client n'écoute aucune requête du serveur.
struct Quiet;

#[async_trait]
impl ClientHandler for Quiet {}

/// Les outils MCP de l'onglet, sur une connexion ouverte une fois par session.
pub(crate) struct McpTools {
    client: Arc<ClientRuntime>,
}

impl std::fmt::Debug for McpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("McpTools")
    }
}

impl McpTools {
    /// Se connecte au serveur MCP de la session, avec son en-tête `Authorization`.
    pub(crate) async fn connect(access: &McpAccess) -> Result<Self, ToolError> {
        let transport = StreamableTransportOptions {
            mcp_url: access.url.clone(),
            request_options: RequestOptions {
                custom_headers: Some(HashMap::from([("Authorization".to_owned(), access.authorization.clone())])),
                ..RequestOptions::default()
            },
        };
        let details = InitializeRequestParams {
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "oku-slides-local-agent".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                description: None,
                icons: vec![],
                website_url: None,
            },
            protocol_version: ProtocolVersion::V2025_11_25.into(),
            meta: None,
        };
        let client = client_runtime::with_transport_options(details, transport, Quiet, None, None, None);
        client.clone().start().await?;
        Ok(Self { client })
    }
}

/// Texte des blocs de contenu, un par ligne ; les autres blocs sont ignorés.
fn text_of(content: &[ContentBlock]) -> String {
    let texts: Vec<&str> = content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::TextContent(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    texts.join("\n")
}

#[async_trait]
impl ToolPort for McpTools {
    async fn list(&self) -> Result<Vec<ToolSpec>, ToolError> {
        let mut specs = Vec::new();
        let mut cursor = None;
        loop {
            let params = cursor.map(|c| PaginatedRequestParams { cursor: Some(c), meta: None });
            let page = self.client.request_tool_list(params).await?;
            for tool in page.tools {
                let schema = serde_json::to_value(&tool.input_schema).context(SchemaSnafu { name: &tool.name })?;
                specs.push(ToolSpec { name: tool.name, description: tool.description.unwrap_or_default(), schema });
            }
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => return Ok(specs),
            }
        }
    }

    async fn call(&self, name: &str, args: &Value) -> Result<String, ToolError> {
        let arguments = match args {
            Value::Object(map) => map.clone(),
            Value::Null => serde_json::Map::new(),
            other => return Err(ToolError::rejected(format!("arguments must be a JSON object, not {other}"))),
        };
        let params = CallToolRequestParams::new(name).with_arguments(arguments);
        let result = self.client.request_tool_call(params).await?;
        let text = text_of(&result.content);
        if result.is_error == Some(true) { Err(ToolError::rejected(text)) } else { Ok(text) }
    }
}

impl Drop for McpTools {
    /// Ferme la connexion en tâche de fond quand la session disparaît.
    fn drop(&mut self) {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let client = self.client.clone();
            handle.spawn(async move {
                if let Err(e) = client.shut_down().await {
                    tracing::warn!(error = %e, "fermeture du client MCP de l'agent local");
                }
            });
        }
    }
}
