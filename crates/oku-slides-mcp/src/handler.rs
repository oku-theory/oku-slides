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

use crate::tool_error::UnknownToolSnafu;
use crate::tools::{Tools, list};

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
    ///
    /// Le tour de l'appel est lu ici, avant tout travail : les surfaces du fil qu'il crée y sont
    /// rangées, même si un autre tour s'est ouvert entre-temps (voir `SurfaceHub::turn`).
    pub(crate) async fn call(&self, name: &str, args: Value) -> CallToolResult {
        let turn = self.hub.turn();
        match name {
            "deck_outline" => self.deck_outline().await,
            "theme_tokens" => self.theme_tokens(args).await,
            "widgets" => self.widgets().await,
            "slide" => self.slide(args).await,
            "render_slide" => self.render_slide(args).await,
            "packs" => self.packs(args).await,
            "layouts" => self.layouts().await,
            "settings" => self.settings().await,
            "apply" => self.apply(args).await,
            "chart_kind" => self.chart_kind(args).await,
            "show_as" => self.show_as(args).await,
            "edit_pack" => self.edit_pack(turn, args).await,
            "add_pack" => self.add_pack(args).await,
            "remove_pack" => self.remove_pack(args).await,
            "set_theme_extends" => self.set_theme_extends(args).await,
            "widget_sources" => self.widget_sources(args).await,
            "widget_source" => self.widget_source(args).await,
            "set_widget" => self.set_widget(turn, args).await,
            "remove_widget" => self.remove_widget(turn, args).await,
            "preview_widget" => self.preview_widget(args).await,
            "set_widget_trial" => self.set_widget_trial(args).await,
            "import_report" => self.import_report().await,
            "import_diff" => self.import_diff(args).await,
            "focus" => self.focus(args),
            "ask_choice" => self.ask_choice(turn, args).await,
            "ask_confirm" => self.ask_confirm(args).await,
            "ask_rewrite" => self.ask_rewrite(args).await,
            "show_params" => self.show_params(turn, args).await,
            "show_variants" => self.show_variants(args).await,
            "show_themes" => self.show_themes(args).await,
            "show_packs" => self.show_packs().await,
            "report" => self.report(turn, args).await,
            "render" => self.render(turn, args),
            "save" => self.save(args).await,
            "open" => self.open(args).await,
            "new" => self.new_deck().await,
            "save_pack" => self.save_pack(args).await,
            "delete_pack" => self.delete_pack(args).await,
            "edit_theme" => self.edit_theme(args).await,
            "set_setting" => self.set_setting(args).await,
            "show_dictation" => self.show_dictation().await,
            "set_dictation" => self.set_dictation(args).await,
            "download_dictation_model" => self.download_dictation_model(args).await,
            "cancel_dictation_download" => self.cancel_dictation_download().await,
            "select_agent" => self.select_agent(args).await,
            "download_agent_model" => self.download_agent_model().await,
            "cancel_agent_model_download" => self.cancel_agent_model_download().await,
            "move" => self.move_file(args).await,
            "merge" => self.merge(args).await,
            "history" => self.history(args).await,
            "present" => self.present(args).await,
            "set_interface" => self.set_interface(args).await,
            "save_copy" => self.save_copy(args).await,
            _ => self.fail_with(&UnknownToolSnafu { name }.build()),
        }
    }
}
