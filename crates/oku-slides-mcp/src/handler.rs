//! Le serveur MCP : liste des outils et appel, journalisé (nom et durée).

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, RpcError,
    schema_utils::CallToolError,
};
use serde_json::Value;

use crate::tools::{Tools, fail, list};

#[async_trait]
impl ServerHandler for Tools {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult { tools: list(), meta: None, next_cursor: None })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let started = Instant::now();
        let name = params.name.clone();
        let args = Value::Object(params.arguments.unwrap_or_default());
        let result = self.call(&name, args).await;
        tracing::debug!(tool = %name, ms = started.elapsed().as_millis(), error = result.is_error == Some(true), "outil de l'agent");
        Ok(result)
    }
}

impl Tools {
    /// Un appel d'outil ; une erreur revient à l'agent en `isError`.
    pub(crate) async fn call(&self, name: &str, args: Value) -> CallToolResult {
        match name {
            "deck_outline" => self.deck_outline().await,
            "slide" => self.slide(args).await,
            "render_slide" => self.render_slide(args).await,
            "templates" => self.templates().await,
            "settings" => self.settings().await,
            "apply" => self.apply(args).await,
            "chart_kind" => self.chart_kind(args).await,
            "edit_templates" => self.edit_templates(args).await,
            "close_templates" => self.close_templates().await,
            "focus" => self.focus(args),
            "ask_choice" => self.ask_choice(args).await,
            "ask_confirm" => self.ask_confirm(args).await,
            "ask_rewrite" => self.ask_rewrite(args).await,
            "show_params" => self.show_params(args).await,
            "show_variants" => self.show_variants(args).await,
            "save" => self.save(args).await,
            "open" => self.open(args).await,
            "new" => self.new_deck().await,
            "save_templates" => self.save_templates(args).await,
            "delete_template_set" => self.delete_template_set(args).await,
            "set_setting" => self.set_setting(args).await,
            _ => fail(format!("unknown tool: {name}")),
        }
    }
}
